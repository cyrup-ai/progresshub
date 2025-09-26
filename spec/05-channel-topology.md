# Channel Topology Specification

## Overview

This document provides a complete map of all channels in the system and how they connect.

## Channel Inventory

The system has exactly **TWO** types of channels:

1. **Progress Channel** - For download progress updates
2. **Bandwidth Channel** - For network bandwidth statistics

## Detailed Channel Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     MultiDownloadOrchestrator                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Creates ONE progress channel on construction:        │   │
│  │                                                      │   │
│  │ let (progress_tx, progress_rx) = mpsc::channel(1000)│   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │  Worker 1   │  │  Worker 2   │  │  Worker 3   │       │
│  │ tx.clone()  │  │ tx.clone()  │  │ tx.clone()  │       │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘       │
│         │                 │                 │               │
│         └─────────────────┴─────────────────┘               │
│                           │                                 │
│                           ▼                                 │
│                    progress_tx ────────────────────────────►│ progress_rx
└─────────────────────────────────────────────────────────────┘      │
                                                                      │
                                                                      ▼
┌─────────────────────────────────────────────────────────────┐     TUI
│                     BandwidthMonitor                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Created separately in main:                          │   │
│  │                                                      │   │
│  │ let (bandwidth_tx, bandwidth_rx) = mpsc::channel(100)│  │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  bandwidth_tx ──────────────────────────────────────────────►│ bandwidth_rx
└─────────────────────────────────────────────────────────────┘
```

## Channel Cardinality

### Progress Channel
- **ONE channel per application lifetime**
- **ONE receiver** (owned by TUI)
- **MANY senders** (one per worker, all clones of same tx)
- **Buffer size**: 1000 messages
- **Lifetime**: Created at startup, dropped at shutdown

### Bandwidth Channel  
- **ONE channel per application lifetime**
- **ONE receiver** (owned by TUI)
- **ONE sender** (bandwidth monitor)
- **Buffer size**: 100 messages
- **Lifetime**: Created at startup, dropped at shutdown

## Not Per-Model or Per-File!

**Critical point**: We do NOT create:
- ❌ One channel per model
- ❌ One channel per file  
- ❌ Dynamic channels based on downloads

We create:
- ✅ ONE progress channel when orchestrator is created
- ✅ All workers share it via cloned senders

## Channel Lifecycle

### Progress Channel

```rust
// 1. Created at application startup in main
let (progress_tx, progress_rx) = flume::bounded(1000);

// 2. Receiver given to TUI
let app = App::new(progress_rx);

// 3. Sender wrapped in handler
let progress_handler = ChannelProgressHandler::new(progress_tx);

// 4. Handler passed to downloads (can be reused)
orchestrator.download(models1, progress_handler.clone()).await?;
// ... later ...
orchestrator.download(models2, progress_handler.clone()).await?;

// 5. Channel lives for entire application lifetime
// Dropped when app exits
```

### Bandwidth Channel

```rust
// 1. Created in main
let (bandwidth_tx, bandwidth_rx) = mpsc::channel(100);

// 2. Given to monitor
let monitor = BandwidthMonitor::new(bandwidth_tx);

// 3. Receiver given to TUI
let app = App::new(progress_rx, bandwidth_rx);

// 4. Lives for entire application lifetime
```

## TUI Channel Handling

The TUI handles both channels in its event loop:

```rust
pub struct App {
    progress_rx: mpsc::Receiver<ProgressData>,
    bandwidth_rx: mpsc::Receiver<BandwidthStats>,
    state: AppState,
}

impl App {
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                // Handle progress from ANY worker
                Some(progress) = self.progress_rx.recv() => {
                    self.state.update_progress(progress);
                    self.render();
                }
                
                // Handle bandwidth updates
                Some(bandwidth) = self.bandwidth_rx.recv() => {
                    self.state.bandwidth = bandwidth;
                    self.render();
                }
                
                // Handle user input...
            }
        }
    }
}
```

## Message Identification

Since all progress goes through one channel, messages identify themselves:

```rust
pub struct ProgressData {
    pub model_id: String,      // Which model this is for
    pub file_path: String,     // Which file within model
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    // ...
}
```

The TUI groups and tracks progress by model_id and file_path.

## Why This Design?

1. **Simple** - Just two channels to manage
2. **Efficient** - No dynamic channel creation/destruction
3. **Predictable** - Fixed topology, no surprises
4. **Fast** - Minimal synchronization overhead
5. **Clear ownership** - TUI owns receivers exclusively