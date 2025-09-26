# Channel Architecture Specification

## Overview

This document clarifies the channel architecture for ProgressHub, resolving the confusion between "event bus" and "direct channels".

## The Truth: Direct Channels, No Global Bus

**There is NO global event bus.** The system uses direct mpsc channels for all communication.

## Channel Topology

```
┌─────────────────┐
│ Download Worker │──────┐
└─────────────────┘      │
                         ├──> Progress Channel ──> TUI
┌─────────────────┐      │    (mpsc::channel)
│ Download Worker │──────┘
└─────────────────┘

┌─────────────────┐
│ Bandwidth Monitor│──────> Bandwidth Channel ──> TUI
└─────────────────┘        (separate channel)
```

## Implementation Details

### 1. Progress Channel Creation

```rust
// In main.rs - NOT in MultiDownloadOrchestrator
use flume;

// Create channel in main
let (progress_tx, progress_rx) = flume::bounded::<ProgressData>(1000);

// Create handler with the sender
let progress_handler = ChannelProgressHandler::new(progress_tx);

// Pass handler to orchestrator (maintains existing API)
let orchestrator = MultiDownloadOrchestrator::new();
let result_future = orchestrator.download(models, Box::new(progress_handler));

// TUI owns the receiver
let app = App::new(progress_rx);
```

### 2. Channel Sharing Pattern

Each download worker gets a **clone** of the same `progress_tx`:

```rust
// In download worker
let progress_handler = ChannelProgressHandler::new(progress_tx.clone());
hf_client.download(model, progress_handler).await?;
```

### 3. Why Not Event Bus?

An event bus implies:
- Global singleton access
- Dynamic subscription/unsubscription
- Topic-based filtering
- Multiple consumers

We don't need any of that. We have:
- Single producer (orchestrator creates channel)
- Multiple senders (workers clone the sender)
- Single consumer (TUI owns receiver)
- No filtering needed

## Bandwidth Channel

Bandwidth monitoring is completely separate:

```rust
// Created independently in main
let (bandwidth_tx, bandwidth_rx) = flume::bounded(100);
let bandwidth_monitor = BandwidthMonitor::new(bandwidth_tx);

// TUI receives both channels
let app = App::new(progress_rx, bandwidth_rx);
```

## ChannelProgressHandler Implementation

The handler converts from existing types to channel types:

```rust
pub struct ChannelProgressHandler {
    tx: flume::Sender<ProgressData>,
    model_id: String,  // Passed in constructor
    last_sent: HashMap<String, Instant>,
}

impl ProgressHandler for ChannelProgressHandler {
    fn handle(&self, progress: DownloadProgress) {
        // Convert DownloadProgress to ProgressData
        let data = ProgressData {
            model_id: self.model_id.clone(),
            file_path: progress.file_path,
            bytes_downloaded: progress.bytes_downloaded,
            total_bytes: progress.total_bytes,
            timestamp: Instant::now(),
            is_cached: progress.from_cache,
            error: None,  // Only set on final failure
        };
        
        // Apply time-based throttling
        if self.should_send(&data) {
            let _ = self.tx.send(data);
        }
    }
}
```

## Key Points

1. **One progress channel per download session** - created in main.rs
2. **Shared sender** - wrapped in ChannelProgressHandler
3. **Single receiver** - TUI owns it exclusively  
4. **No global state** - channels passed explicitly
5. **Bandwidth separate** - different concern, different channel
6. **Uses flume** - not std::sync::mpsc, for better async performance
7. **Model ID passed down** - not extracted from paths

## What About TODO.md's EventBus?

The TODO.md is **outdated**. It shows an older design that we've since improved. The current architecture uses direct channels because:

- Simpler to reason about
- Better performance (no indirection)
- Clearer ownership model
- No hidden global state