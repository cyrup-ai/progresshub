# ProgressHub API Documentation

## Overview

ProgressHub provides a modular API for downloading models from HuggingFace Hub with real-time progress tracking and bandwidth monitoring. The API is organized into several crates, each with specific responsibilities.

## Core Crates

### 1. progresshub-config

**Purpose**: Configuration management and type definitions

**Key Types**:
```rust
// Download configuration
pub struct DownloadConfig {
    pub show_progress: bool,
    pub use_cache: bool,
}

// Progress information
pub struct DownloadProgress {
    pub path: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed_mbps: f64,
}

// Download result
pub struct DownloadResult {
    pub path: PathBuf,
    pub file_count: usize,
    pub total_bytes: u64,
    pub file_paths: Vec<PathBuf>,
}
```

**Key Traits**:
```rust
// Progress handler for receiving updates
pub trait ProgressHandler: Send + Sync {
    fn handle(&self, progress: DownloadProgress);
}

// Configuration trait
pub trait ConfigTrait {
    fn get_hf_home(&self) -> PathBuf;
    fn get_hf_hub_cache(&self) -> PathBuf;
    fn get_xet_cache_dir(&self) -> PathBuf;
    fn get_hf_token(&self) -> Option<String>;
}
```

### 2. progresshub-progress

**Purpose**: Event-driven progress tracking system

**Key APIs**:
```rust
// Global event bus access
pub fn event_bus() -> &'static ProgressEventBus;

// Subscribe to download events
pub fn subscribe_download_events() -> Receiver<DownloadCompleted>;

// Subscribe to bandwidth changes
pub fn subscribe_bandwidth_changes() -> Receiver<BandwidthChanged>;

// Progress event builder
pub fn event() -> ProgressEventBuilder;
```

**Key Types**:
```rust
pub struct ProgressEvent {
    pub model_id: String,
    pub file_name: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub download_speed: f64,
}

pub struct BandwidthChanged {
    pub download_mbps: f64,
    pub upload_mbps: f64,
}

pub struct DownloadCompleted {
    pub model_id: String,
    pub total_bytes: u64,
    pub duration_secs: f64,
}
```

### 3. progresshub-client-xet

**Purpose**: XET protocol implementation for efficient downloads

**Key APIs**:
```rust
// Main client builder
pub struct CasClientBuilder {
    // Configure endpoints, cache, etc.
}

// Main client interface
pub struct CasClient {
    // Methods for upload/download
}

// Key types
pub struct MerkleHash([u8; 32]);
pub struct Key {
    pub tenant: String,
    pub hash: MerkleHash,
}

// Progress tracking
pub trait ProgressReporter {
    fn update(&self, bytes: u64);
    fn finish(&self);
}
```

**Usage Example**:
```rust
let client = CasClient::for_huggingface();
let hash = MerkleHash::from_hex("...")?;
let key = Key::new("default", hash);

// Stream download
let mut stream = client.download_xorb(key, None);
while let Some(chunk) = stream.next().await {
    // Process chunk
}
```

### 4. progresshub-bandwidth

**Purpose**: Network bandwidth monitoring

**Key APIs**:
```rust
// Start bandwidth monitoring
pub fn tick(display_bandwidth: bool) -> JoinHandle<()>;

// Get bandwidth statistics
pub fn bandwidth_event_bus() -> &'static BandwidthEventBus;

// Bandwidth classification
#[derive(Debug, Clone, Copy)]
pub enum BandwidthClass {
    VeryLow,   // < 10 Mbps
    Low,       // 10-50 Mbps  
    Medium,    // 50-100 Mbps
    High,      // 100-500 Mbps
    VeryHigh,  // > 500 Mbps
}
```

### 5. progresshub-client-selector

**Purpose**: Backend selection and download orchestration

**Key APIs**:
```rust
// Multi-download orchestrator
pub struct MultiDownloadOrchestrator {
    // Manages concurrent downloads
}

// Backend selection
pub enum Backend {
    Xet,
    Http,
}

// Main client interface
pub struct Client {
    // Unified interface over all backends
}
```

### 6. progresshub-tui

**Purpose**: Terminal user interface and main binary

**Key Components**:
- Ratatui-based UI
- Model list with collapsible details
- Real-time progress visualization
- Bandwidth monitoring display
- CyrupTheme styling

## Usage Examples

### Basic Download
```rust
use progresshub::{ModelDownloader, DownloadConfig};

// Downloads go to HF standard cache directories automatically
let results = ModelDownloader::new()
    .model("gpt2")
    .model("bert-base-uncased")
    .download()
    .await?;
```

### Custom Progress Handler
```rust
use progresshub::{ProgressHandler, DownloadProgress};

struct MyProgressHandler;

impl ProgressHandler for MyProgressHandler {
    fn handle(&self, progress: DownloadProgress) {
        println!("{}: {}%", 
            progress.path,
            (progress.bytes_downloaded as f64 / progress.total_bytes as f64) * 100.0
        );
    }
}
```

### Event-Based Progress Tracking
```rust
use progresshub::progress::{subscribe_download_events, subscribe_bandwidth_changes};

// Subscribe to events
let download_rx = subscribe_download_events();
let bandwidth_rx = subscribe_bandwidth_changes();

// Process events
tokio::spawn(async move {
    while let Ok(event) = download_rx.recv_async().await {
        println!("Download completed: {} ({} bytes)", 
            event.model_id, 
            event.total_bytes
        );
    }
});
```

## Environment Variables

- `HF_HOME`: Base directory for HuggingFace cache
- `HF_HUB_CACHE`: Model cache directory
- `HF_XET_CACHE`: XET-specific cache
- `HF_TOKEN`: Authentication token
- `HF_HUB_OFFLINE`: Offline mode flag
- `HF_XET_CHUNK_CACHE_SIZE_BYTES`: Cache size limit

## Performance Tips

1. **Use XET backend when available**: It's significantly faster for large models
2. **Enable bandwidth monitoring**: Helps optimize download strategies
3. **Configure chunk cache size**: Based on available disk space
4. **Use async APIs**: For maximum concurrency
5. **Batch downloads**: The orchestrator handles multiple models efficiently

## Error Handling

All APIs use `Result<T, E>` with specific error types:
- `CasError`: XET-specific errors
- `anyhow::Error`: General errors with context

Always handle errors appropriately and provide meaningful context to users.