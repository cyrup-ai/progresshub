# HuggingFace Download Specification

This document describes the complete HuggingFace model download architecture, caching system, and proper implementation patterns for progresshub.

## Overview

HuggingFace uses a sophisticated multi-layer storage system designed for:
- **Deduplication**: Avoid downloading the same model multiple times
- **Sharing**: Multiple projects can reference the same cached model
- **Resumption**: Interrupted downloads can be resumed
- **Performance**: Efficient storage and retrieval of large models

## Architecture Components

### 1. Cache Layer (Central Storage)
The cache serves as the **single source of truth** for all downloaded models:

```
$HF_HUB_CACHE/
├── models--{org}--{model}/
│   ├── refs/
│   │   └── main              # Points to specific revision SHA
│   ├── snapshots/
│   │   └── {revision_sha}/   # Actual model files live here
│   │       ├── config.json
│   │       ├── pytorch_model.bin
│   │       └── tokenizer.json
│   └── blobs/
│       └── {blob_sha}        # Deduplicated file content by hash
```

### 2. Project/Application Layer (Final Destinations)
Applications create **references** to cached models:
- **Symlinks** (preferred): Point to cache, no duplication
- **Hard links**: Reference same inode, space-efficient  
- **Copies**: Full duplication (fallback for incompatible filesystems)

## Environment Variables

### Core Directories
| Variable | Default | Purpose |
|----------|---------|---------|
| `HF_HOME` | `~/.cache/huggingface` | Root directory for all HF data |
| `HF_HUB_CACHE` | `$HF_HOME/hub` | **Primary model cache** |
| `HF_XET_CACHE` | `$HF_HOME/xet` | XET chunk cache for large files |
| `HF_ASSETS_CACHE` | `$HF_HOME/assets` | Downstream library assets |

### Authentication
| Variable | Purpose |
|----------|---------|
| `HF_TOKEN` | User access token for private/gated models |
| `HF_TOKEN_PATH` | Custom token file location |

### Performance & Behavior
| Variable | Default | Purpose |
|----------|---------|---------|
| `HF_HUB_ENABLE_HF_TRANSFER` | `false` | Use Rust-based fast transfers |
| `HF_HUB_OFFLINE` | `false` | Only use cached files |

## Download Flow

### Phase 1: Cache Population
```
1. Check if model exists in cache ($HF_HUB_CACHE/models--{org}--{model}/)
2. If missing or outdated:
   a. Download manifest/metadata
   b. Choose backend (regular HTTP vs XET)
   c. Download files to cache with deduplication
   d. Update refs to point to new revision
3. Cache is now populated and ready for use
```

### Phase 2: Application Integration
```
1. Application requests model at specific path
2. progresshub creates references from final location to cache:
   - Symlinks (preferred): ln -s $HF_HUB_CACHE/snapshots/{sha}/file $DEST/file
   - Hard links (fallback): ln $HF_HUB_CACHE/blobs/{hash} $DEST/file  
   - Copy (last resort): cp $HF_HUB_CACHE/snapshots/{sha}/file $DEST/file
3. Application uses model from final location
```

## Storage Backends

### Regular Backend (HTTP)
- Standard file downloads via requests
- Files stored directly in snapshots/{sha}/
- Good for smaller models and standard use cases

### XET Backend (Advanced)
- Chunk-based deduplication system
- Files split into chunks, stored in blobs/
- Reconstructed on-demand via symlinks or direct access
- Optimal for very large models (>100MB files)
- Enabled automatically when `hf-xet` package installed

## Cache Structure Details

### Blob Storage (Deduplication)
```
$HF_HUB_CACHE/
└── models--microsoft--DialoGPT-medium/
    ├── blobs/
    │   ├── abc123def456...    # Actual file content (by SHA hash)
    │   └── 789ghi012jkl...    # Shared across models if identical
    ├── snapshots/
    │   └── 4a1b2c3d4e5f.../  # Revision-specific view
    │       ├── config.json -> ../../blobs/abc123def456...
    │       └── model.bin   -> ../../blobs/789ghi012jkl...
    └── refs/
        └── main              # Contains: 4a1b2c3d4e5f...
```

### Deduplication Benefits
- Same file in multiple models = stored once
- Different revisions share unchanged files
- Massive space savings for large model collections

## Resume & Caching Logic

### File Validation
```rust
fn is_file_cached(cache_path: &Path, expected_size: u64, expected_sha: &str) -> bool {
    if !cache_path.exists() { return false; }
    
    let metadata = fs::metadata(cache_path)?;
    if metadata.len() != expected_size { return false; }
    
    // Optional: Verify SHA hash for critical applications
    // let actual_sha = calculate_sha256(cache_path)?;
    // actual_sha == expected_sha
    
    true
}
```

### Resume Strategy
1. **Check blob existence**: If `blobs/{sha}` exists with correct size → skip
2. **Partial download detection**: If partial file exists → resume from offset
3. **Integrity verification**: Optional SHA validation for critical files
4. **Atomic completion**: Use temporary files, rename on completion

## Security Considerations

### Path Traversal Prevention
```rust
fn sanitize_path(path: &str) -> Result<PathBuf> {
    let normalized = Path::new(path).normalize()?;
    if normalized.components().any(|c| c == Component::ParentDir) {
        return Err("Path traversal detected");
    }
    Ok(normalized.to_path_buf())
}
```

### Token Security
- Tokens stored in `$HF_HOME/token` with 600 permissions
- Never log or expose tokens in error messages
- Support token rotation and expiration

## Implementation Guidelines for progresshub

### 1. Respect HuggingFace Conventions
```rust
// ✅ Correct: Use HF environment variables
let cache_dir = env::var("HF_HUB_CACHE")
    .unwrap_or_else(|_| format!("{}/.cache/huggingface/hub", env::var("HOME")?));

// ❌ Wrong: Hardcode paths
let cache_dir = "/tmp/models";
```

### 2. Proper Cache Integration
```rust
// ✅ Correct: Check cache first, download to cache, then link to destination
async fn download_model(model_id: &str, destination: &Path) -> Result<()> {
    let cached_path = ensure_model_cached(model_id).await?;
    create_model_reference(cached_path, destination)?;
    Ok(())
}

// ❌ Wrong: Download directly to destination
async fn download_model(model_id: &str, destination: &Path) -> Result<()> {
    download_files_directly(model_id, destination).await?;
}
```

### 3. Handle Both Backends
```rust
// Check manifest to determine optimal backend
let manifest = fetch_manifest(model_id).await?;
let use_xet = manifest.has_large_files() && xet_available();

if use_xet {
    download_via_xet(model_id, &manifest).await?;
} else {
    download_via_http(model_id, &manifest).await?;
}
```

### 4. Graceful Degradation
- Symlinks → Hard links → Copy (filesystem compatibility)
- XET → HTTP (backend availability)
- Fast transfer → Standard requests (package availability)

## Error Handling

### Common Issues
1. **Insufficient disk space**: Check before download
2. **Network interruption**: Implement resumption
3. **Permission errors**: Fallback to user directories
4. **Filesystem limitations**: Degrade gracefully from symlinks

### Recovery Strategies
```rust
// Cleanup incomplete downloads
fn cleanup_partial_download(path: &Path) -> Result<()> {
    if path.exists() && !is_complete_download(path)? {
        fs::remove_file(path)?;
    }
    Ok(())
}

// Verify and repair cache integrity
fn verify_cache_integrity(cache_dir: &Path) -> Result<()> {
    // Check for orphaned blobs, broken symlinks, corrupted files
    // Attempt repair or mark for re-download
}
```

## Performance Optimization

### Concurrent Downloads
- Respect `HF_XET_NUM_CONCURRENT_RANGE_GETS` (default: 16)
- Implement bandwidth-aware concurrency limiting
- Prioritize large files first (better cache utilization)

### Progress Reporting
- Report progress per-file and overall
- Account for cache hits (instant "download")
- Show deduplication savings

### Memory Management
- Stream large files, don't load into memory
- Use memory-mapped files for hash verification
- Cleanup temporary resources promptly

## Best Practices

1. **Always use the cache**: Never bypass HuggingFace's caching system
2. **Respect environment variables**: Users configure their setup for a reason
3. **Implement proper resumption**: Large models take time to download
4. **Handle offline mode**: Support `HF_HUB_OFFLINE=1`
5. **Graceful backend selection**: XET when beneficial, HTTP as fallback
6. **Security first**: Validate paths, protect tokens, verify integrity
7. **Performance awareness**: Use bandwidth monitoring and adaptive concurrency

---

*This specification ensures progresshub integrates seamlessly with the HuggingFace ecosystem while providing optimal performance and user experience.*