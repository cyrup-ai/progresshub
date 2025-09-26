# API Design Specification

## Overview

This document explains the new channel-based API design without backwards compatibility constraints.

## The New API

The MultiDownloadOrchestrator now returns both a Future and a channel receiver:

```rust
pub fn download(
    models: impl Into<OneOrMany<String>>,
) -> (impl Future<Output = Result<DownloadResult>>, ProgressReceiver)
```

This gives us the best of both worlds - progress updates via channel and final results via Future.

## Implementation

The orchestrator creates its own channel and returns the receiver:

```rust
impl MultiDownloadOrchestrator {
    pub fn download(
        &self,
        models: impl Into<OneOrMany<String>>,
    ) -> (impl Future<Output = Result<DownloadResult>>, ProgressReceiver) {
        let models = models.into().to_vec();
        
        // Create channel for this download session
        let (progress_tx, progress_rx) = create_progress_channel(1000);
        
        // Create the download future
        let future = async move {
            let mut results = Vec::new();
            
            for model in &models {
                // Create handler for this model
                let handler = ChannelProgressHandler::new(progress_tx.clone())
                    .for_model(model.clone());
                
                // Download with progress
                let result = self.download_one(model, handler).await?;
                results.push(result);
            }
            
            Ok(DownloadResult {
                models: results,
                // ... aggregate stats
            })
        };
        
        (future, progress_rx)
    }
}
```

## Usage in Main

The main.rs uses the new API:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Create orchestrator
    let orchestrator = MultiDownloadOrchestrator::new();
    
    // Start download - returns BOTH future and receiver
    let (download_future, progress_rx) = orchestrator.download(models);
    
    // Spawn download task
    let download_handle = tokio::spawn(download_future);
    
    // Create bandwidth channel separately
    let (bandwidth_tx, bandwidth_rx) = flume::bounded(100);
    let bandwidth_monitor = BandwidthMonitor::new(bandwidth_tx);
    tokio::spawn(bandwidth_monitor.run());
    
    // Create TUI with both receivers
    let mut app = App::new(progress_rx, bandwidth_rx);
    
    // Run TUI event loop
    app.run().await?;
    
    // Get download results
    let results = download_handle.await??;
    println!("Downloads complete: {:?}", results);
    
    Ok(())
}
```

## How ChannelProgressHandler Works

```rust
pub struct ChannelProgressHandler {
    tx: flume::Sender<ProgressData>,
    model_id: Option<String>,  // Set when downloading specific model
}

impl ChannelProgressHandler {
    pub fn new(tx: flume::Sender<ProgressData>) -> Self {
        Self { tx, model_id: None }
    }
    
    // Called by orchestrator for each model
    pub fn for_model(&self, model_id: String) -> Self {
        Self {
            tx: self.tx.clone(),
            model_id: Some(model_id),
        }
    }
}

impl ProgressHandler for ChannelProgressHandler {
    fn handle(&self, progress: DownloadProgress) {
        if let Some(model_id) = &self.model_id {
            let data = ProgressData {
                model_id: model_id.clone(),
                // ... convert progress
            };
            let _ = self.tx.send(data);
        }
    }
}
```

## Error Handling

Errors are sent through the channel only on final failure:

```rust
impl ChannelProgressHandler {
    fn handle_error(&self, error: DownloadError) {
        if error.is_final() {
            let data = ProgressData {
                model_id: self.model_id.clone().unwrap_or_default(),
                file_path: error.file_path,
                error: Some(error.to_string()),
                // ... other fields
            };
            let _ = self.tx.send(data);
        }
        // Transient errors are retried, not sent
    }
}
```

## Key Design Points

1. **API returns tuple** - `(Future, ProgressReceiver)` for maximum flexibility
2. **Channel per download** - Each download() call creates its own channel
3. **No ProgressHandler in API** - Orchestrator creates handlers internally
4. **Model ID injection** - Handler gets model_id when created for specific model
5. **Error semantics** - Only final failures sent, not transient errors

## Benefits

- Clean API - no trait objects needed
- Direct access to progress channel
- Each download session isolated
- Simple to understand and use
- No hidden channel creation