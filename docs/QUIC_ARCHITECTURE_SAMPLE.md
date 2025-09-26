# Sample ARCHITECTURE.md Section for QUIC Client

## Updated client_gix Description

Replace the current client_gix section with:

```markdown
#### client_gix (QUIC Protocol Client)
- High-performance downloads using QUIC protocol over UDP
- Powered by Crypt Quiche with post-quantum cryptography
- Multiplexed streams for concurrent operations
- Built-in connection persistence and automatic reconnect
- Progress tracking with zero-copy streaming
- Integrates with ../crypt/ infrastructure for security
```

## Updated Data Flow Section

```markdown
## Data Flow

```
User Call: download(models, progress_handler)
    ↓
OneOrMany Resolution (single model or batch)
    ↓
Multi-Download Orchestrator (client_selector)
    ↓
HfClient (client_progresshub) [per model]
    ↓
[Backend Detection: Manifest or Protocol Check]
    ↓
Backend Selection
    ├─→ CasClient (client_xet) - for XET-enabled repos
    └─→ QuicClient (client_gix) - for QUIC-enabled endpoints
    ↓
Progress Data → Channel → ProgressHandler
    ↓
[UI receives from channel]
    ↓
[UI widgets format and display data]
    ↓
[Separate: Bandwidth Monitor (bandwidth)]
    ↓
Future::Output = Result<DownloadResult> with paths & stats
```
```

## Updated Core Types Section

Add to the Core Types (client_progresshub) section:

```rust
// Backend selection with QUIC support
pub enum Backend {
    Cas(CasClient),
    Quic(QuicClient),
}

pub enum BackendKind {
    Cas,
    Quic,
}

// Download identifier
pub enum Identifier<'a> {
    Key(Key),      // For CAS backend  
    Path(&'a str), // For QUIC backend
}
```

## New QUIC Client Types Section

Add after the CAS Client Types section:

```markdown
### QUIC Client Types (client_gix)

```rust
// Core QUIC types
pub struct QuicClient {
    endpoint: String,
    connection: OnceCell<QuicConnection>,
}

pub struct QuicClientBuilder {
    endpoint: String,
    auth: Auth,
    timeout: Duration,
}

// QUIC-specific download interface
impl QuicClient {
    pub fn download_file(
        &self,
        path: &str,
        progress: Option<ProgressSender>,
    ) -> impl Stream<Item = Result<Bytes>>
}

// Authentication options
pub enum Auth {
    MutualTLS { cert: Vec<u8>, key: Vec<u8> },
    PSK { key: Vec<u8> },
    Anonymous, // Testing only
}

// Connection configuration
pub struct QuicConfig {
    pub idle_timeout: Duration,
    pub keep_alive_interval: Duration,
    pub max_concurrent_streams: u32,
}
```
```

## Updated Performance Considerations

Update the Smart Caching section:

```markdown
4. **Smart Caching**: 
   - XET: Content-addressable chunk cache with configurable size
   - QUIC: Connection state caching for 0-RTT reconnects
   - NOTE: Current implementation only validates cache by file size (needs improvement)
```

## Updated Security & Reliability

Update the relevant sections:

```markdown
1. **Hash Verification**: 
   - XET: All downloads verified via Merkle hashes
   - QUIC: SHA-256 integrity checks with post-quantum signatures

3. **Backend Selection**: Selects optimal backend based on available protocols and endpoints

4. **Resume Support**: 
   - XET: Chunk-level resume capability
   - QUIC: Stream-level resume with connection persistence

6. **Post-Quantum Security**:
   - QUIC: ML-KEM key exchange + ML-DSA signatures
   - Future-proof against quantum computing threats
```

## Component Relationships Update

```markdown
### Backend Selection Flow
1. User requests model download
2. HfClient lazily initializes on first use  
3. Checks for backend hints:
   - XET manifest file (`hf_xet_manifest.json`)
   - QUIC endpoint availability (port 11443)
   - Protocol headers or metadata
4. If XET manifest exists → Use CasClient
5. If QUIC endpoint available → Use QuicClient
6. Selection based on performance characteristics:
   - Large files (>1GB): Prefer QUIC for streaming
   - Many small files: Prefer XET for deduplication
```

## Future Considerations Update

Add to the Future Considerations section:

```markdown
6. **QUIC Protocol Enhancements**:
   - HTTP/3 support for web compatibility
   - Multipath QUIC for redundant connections
   - Custom congestion control algorithms
   - Integration with CDN infrastructure
```