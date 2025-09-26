# QUIC Client API Documentation

## Overview

The Crypt Quiche QUIC client provides high-performance, secure file transfers using the QUIC protocol with post-quantum cryptography. This document outlines the API for integrating QUIC transport into progresshub.

## Key Features

- **Transport**: Pure QUIC over UDP (no TCP fallback)
- **Cryptography**: Post-quantum ML-KEM + ML-DSA by default
- **Multiplexing**: Multiple protocols over single connection
- **Persistence**: Long-lived connections with automatic reconnect
- **Zero-config**: Auto-generates ephemeral certs when needed

## Basic Usage

```rust
use cryypt::quiq::{Auth, Quique, Transport};

// Create a QUIC client connection
let connection = Quique::client(Transport::UDP)
    .auth(Auth::Anonymous)  // Use Auth::MutualTLS in production
    .connect("server.example.com:11443")
    .await?;

// Simple file download
let result = connection.download_file("models/llama-2-7b.bin")
    .await?;

// Download with progress tracking
let result = connection.download_file("models/llama-2-7b.bin")
    .with_progress(|p| {
        println!("Progress: {}% ({:.1} MB/s)", p.percent, p.mbps);
    })
    .await?;
```

## Authentication Options

```rust
pub enum Auth {
    /// Mutual TLS with certificate and private key
    MutualTLS {
        cert: Vec<u8>,  // DER or PEM format
        key: Vec<u8>,   // DER or PEM format
    },
    
    /// Pre-shared key authentication
    PSK {
        key: Vec<u8>,
    },
    
    /// Anonymous connection (testing only)
    Anonymous,
}
```

## Multiplexed Protocols

The QUIC connection supports multiple protocols over the same connection:

```rust
// File Transfer Protocol
connection.stream(|stream| async move {
    let result = stream.file_transfer()
        .download("large-model.bin")
        .compressed()
        .with_resume()  // Automatic resume on disconnect
        .await?;
    Ok(())
}).await?;

// Messaging Protocol (for metadata, notifications)
connection.stream(|stream| async move {
    stream.messaging()
        .send("Download started")
        .reliable()
        .await?;
    Ok(())
}).await?;

// RPC Protocol (for API calls)
connection.stream(|stream| async move {
    let response = stream.rpc()
        .call("get_model_info", "llama-2-7b")
        .timeout(Duration::from_secs(30))
        .await?;
    Ok(())
}).await?;
```

## Error Handling

```rust
use cryypt::quiq::error::{CryptoTransportError, Result};

// Error types
pub enum CryptoTransportError {
    Io(std::io::Error),
    Quiche(quiche::Error),
    CertificateInvalid(String),
    HandshakeFailed(String),
    ConnectionLost(String),
    Internal(String),
}

// Handling connection errors
match connection.download_file("model.bin").await {
    Ok(result) => println!("Success: {} bytes", result.bytes_transferred),
    Err(CryptoTransportError::ConnectionLost(msg)) => {
        // Automatic reconnect handled internally
        println!("Connection lost: {}, retrying...", msg);
    }
    Err(e) => eprintln!("Download failed: {}", e),
}
```

## Progress Tracking

```rust
#[derive(Debug, Clone)]
pub struct FileTransferProgress {
    pub file_id: Uuid,
    pub filename: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub throughput_mbps: f64,
    pub eta_seconds: Option<u64>,
}

// Progress callback
connection.download_file("model.bin")
    .with_progress(|progress: FileTransferProgress| {
        let percent = (progress.bytes_transferred as f64 / progress.total_bytes as f64) * 100.0;
        println!(
            "{}: {:.1}% complete ({:.1} MB/s, ETA: {:?}s)",
            progress.filename,
            percent,
            progress.throughput_mbps,
            progress.eta_seconds
        );
    })
    .await?;
```

## Transfer Results

```rust
#[derive(Debug)]
pub struct TransferResult {
    pub file_id: Uuid,
    pub filename: String,
    pub bytes_transferred: u64,
    pub duration: Duration,
    pub checksum: String,  // SHA-256 hex
    pub success: bool,
}
```

## Integration with ProgressHub

For progresshub integration, the QUIC client can be wrapped to match the existing download interface:

```rust
// In client_gix/src/quic_client.rs
use cryypt::quiq::{Quique, Transport, Auth};
use progresshub_progress::{ProgressData, ProgressSender};

pub struct QuicClient {
    endpoint: String,
    auth: Auth,
}

impl QuicClient {
    pub fn download_file(
        &self,
        path: &str,
        progress_tx: Option<ProgressSender>,
    ) -> impl Stream<Item = Result<Bytes>> {
        // Implementation that:
        // 1. Connects to QUIC endpoint
        // 2. Downloads file with progress
        // 3. Converts progress to ProgressData format
        // 4. Streams bytes back to caller
    }
}
```

## Configuration

The QUIC client supports various configuration options:

```rust
Quique::client(Transport::UDP)
    .auth(Auth::MutualTLS { cert, key })
    .idle_timeout(Duration::from_secs(30))
    .keep_alive_interval(Duration::from_secs(10))
    .max_concurrent_streams(100)
    .connect(endpoint)
    .await?;
```

## Security Considerations

1. **Always use Auth::MutualTLS in production** - Anonymous auth is only for testing
2. **Post-quantum crypto is enabled by default** - No action needed
3. **Certificate pinning recommended** for known servers
4. **Connection state is encrypted** - Prevents middlebox tampering

## Performance Tips

1. **Reuse connections** - Don't create new connections for each transfer
2. **Use compression** for text/JSON files: `.compressed()`
3. **Enable resume** for large files: `.with_resume()`
4. **Tune concurrent streams** based on bandwidth and server capacity
5. **Monitor progress callbacks** shouldn't do heavy work

## Example: Complete Download Flow

```rust
async fn download_model(model_path: &str) -> Result<PathBuf> {
    // 1. Establish connection (reuse if possible)
    let connection = Quique::client(Transport::UDP)
        .auth(Auth::MutualTLS {
            cert: load_cert()?,
            key: load_key()?,
        })
        .connect("models.huggingface.co:11443")
        .await?;
    
    // 2. Download with all features
    let result = connection.download_file(model_path)
        .compressed()           // Auto-decompress if needed
        .with_resume()         // Resume from interruption
        .with_integrity_check() // Verify checksum
        .with_progress(|p| {
            // Update UI or send to progress channel
            update_progress_bar(p);
        })
        .to_path("/tmp/models/") // Download destination
        .await?;
    
    // 3. Verify and return
    println!("Downloaded {} in {:?}", result.filename, result.duration);
    println!("Checksum: {}", result.checksum);
    
    Ok(PathBuf::from(format!("/tmp/models/{}", result.filename)))
}
```