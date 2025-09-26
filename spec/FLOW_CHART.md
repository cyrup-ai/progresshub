# ProgressHub Data/Event Flow Chart

## Overview

This document provides a detailed visual representation of the **Pure Flume Channel Event-Driven Architecture** with **ProgressCalculator snapshots as events**.

## High-Level Flow Diagram

```
┌─────────────────┐    ┌─────────────────┐
│   User Input    │    │ Quantization    │
│  (CLI/Args)     │────│   Filtering     │
└─────────────────┘    └─────────────────┘
         │                       │
         ▼                       │
┌─────────────────┐              │
│ Client Selector │              │
│  (Orchestration │◄─────────────┘
│    Only)        │
└─────────────────┘
         │
         ▼
┌─────────────────┐    ┌─────────────────┐
│   XET Client    │    │   QUIC Client   │
│ (Pure Download) │    │ (Pure Download) │
└─────────────────┘    └─────────────────┘
         │                       │
         │    RawDownloadEvent   │
         │      (6 fields)       │
         ▼                       ▼
┌───────────────────────────────────────────┐
│        CentralProgressDispatcher          │
│             (ONLY "BRAINS")               │
│  ┌─────────────────────────────────────┐  │
│  │    FilesystemProgressEvaluator      │  │
│  │   (Monotonic Progress Guarantee)    │  │
│  └─────────────────────────────────────┘  │
│  ┌─────────────────────────────────────┐  │
│  │      ProgressCalculator Creator     │  │
│  │    (ALL Formatting Methods)        │  │
│  └─────────────────────────────────────┘  │
└───────────────────────────────────────────┘
         │
         │ ProgressCalculator Snapshots
         │     (Immutable Events)
         ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  CLI Display    │    │  TUI Display    │    │ Ratatui Display │
│ (ZERO Formatting│    │ (ZERO Formatting│    │ (ZERO Formatting│
│     Logic)      │    │     Logic)      │    │     Logic)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Detailed Component Data Flow

### 1. User Input & Quantization Processing

```
User Command: cargo run -- --quant Q4_K_M meta-llama/Llama-2-7b
     │
     ▼
┌─────────────────────────────────────────────┐
│            CLI Argument Parser              │
│                                             │
│ Input: --quant Q4_K_M meta-llama/Llama-2-7b│
│ Output: {                                   │
│   model_id: "meta-llama/Llama-2-7b",      │
│   quantization: "Q4_K_M"                   │
│ }                                          │
└─────────────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────────────┐
│           Client Selector                   │
│        (Orchestration Only)                 │
│                                             │
│ • Reads HuggingFace manifest                │
│ • Filters files by quantization             │
│ • Selects XET vs QUIC per file              │
│ • NO progress handling                      │
│                                             │
│ Output: Filtered file list with backend     │
│ selection + quantization metadata           │
└─────────────────────────────────────────────┘
```

### 2. Download Client Event Generation

```
┌─────────────────────────────────────────────┐
│                XET Client                   │
│            (Pure Download)                  │
│                                             │
│ • Downloads via content-addressable storage │
│ • NO calculations, NO formatting            │
│ • Dispatches events ONLY                    │
└─────────────────────────────────────────────┘
     │
     │ Raw Event Dispatch (Flume Channel)
     ▼
┌─────────────────────────────────────────────┐
│            RawDownloadEvent                 │
│              (6 Fields)                     │
│                                             │
│ {                                           │
│   model_id: "meta-llama/Llama-2-7b",      │
│   quant: "Q4_K_M",                         │
│   remote_url: "https://huggingface.co/...",│
│   local_filepath: "/cache/pytorch_model.bin│
│   bytes_downloaded: 1024576,               │
│   total_bytes: 2048000                     │
│ }                                          │
└─────────────────────────────────────────────┘
     │
     │ Same structure from QUIC Client
     ▼
┌─────────────────────────────────────────────┐
│               QUIC Client                   │
│            (Pure Download)                  │
│                                             │
│ • Downloads via QUIC transport              │
│ • NO calculations, NO formatting            │
│ • Dispatches identical event structure      │
└─────────────────────────────────────────────┘
```

### 3. Central Intelligence Processing

```
┌───────────────────────────────────────────────────────────────┐
│                CentralProgressDispatcher                     │
│                    (ONLY "BRAINS")                           │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │           Event Reception & Validation                  │  │
│  │                                                         │  │
│  │ • Receives RawDownloadEvent via flume                   │  │
│  │ • Validates event structure (6 fields)                 │  │
│  │ • Maintains state of all active downloads              │  │
│  └─────────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │        FilesystemProgressEvaluator                      │  │
│  │                                                         │  │
│  │ • Validates against actual filesystem state             │  │
│  │ • Prevents backwards progress                          │  │
│  │ • Ensures monotonic guarantees                         │  │
│  │ • Returns validated progress data                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │         ProgressCalculator Creator                      │  │
│  │                                                         │  │
│  │ • Creates immutable ProgressCalculator snapshots        │  │
│  │ • Performs ALL calculations:                           │  │
│  │   - Percentages                                        │  │
│  │   - Speed calculations                                 │  │
│  │   - ETA calculations                                   │  │
│  │   - Byte formatting                                    │  │
│  │ • Provides ALL formatting methods                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │          Debounced Dispatcher                           │  │
│  │                                                         │  │
│  │ • 100ms throttling during active periods              │  │
│  │ • Immediate dispatch after quietude                   │  │
│  │ • Prevents UI spam                                     │  │
│  │ • Lightning responsiveness                             │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
                              │
                              │ ProgressCalculator Snapshots (Events)
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                  Flume Channel                                │
│                                                               │
│ Event Type: ProgressCalculator (Immutable Snapshot)          │
│                                                               │
│ Contains ALL formatting methods:                              │
│ • percentage_formatted() → "76.4%"                           │
│ • bytes_formatted() → "1.2 GB / 3.4 GB"                     │
│ • speed_formatted() → "156 MB/s"                             │
│ • eta_formatted() → "2m 34s"                                 │
│ • files_remaining_formatted() → "12 files remaining"         │
│ • files_completed_formatted() → "8 of 20 files"              │
│ • overall_progress_formatted() → "Model: 45.2% complete"     │
└───────────────────────────────────────────────────────────────┘
```

### 4. Display Layer Consumption

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Display   │    │   TUI Display   │    │ Ratatui Display │
│                 │    │                 │    │                 │
│ ZERO Formatting │    │ ZERO Formatting │    │ ZERO Formatting │
│ ZERO Calculations│    │ ZERO Calculations│    │ ZERO Calculations│
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │ Receives ProgressCalculator Snapshots via Flume │
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Display Processing                              │
│                                                                 │
│ CLI Example:                                                    │
│   println!("Progress: {}", progress.percentage_formatted());   │
│   // Displays: "Progress: 76.4%"                              │
│                                                                 │
│ TUI Example:                                                    │
│   text.push_line(progress.bytes_formatted());                  │
│   // Displays: "1.2 GB / 3.4 GB"                              │
│                                                                 │
│ Ratatui Example:                                                │
│   speed_widget.set_text(progress.speed_formatted());           │
│   // Displays: "156 MB/s"                                      │
│                                                                 │
│ STYLING ONLY: Colors, fonts, positioning                       │
│ NO DATA FORMATTING: All strings pre-formatted                  │
└─────────────────────────────────────────────────────────────────┘
```

## Key Data Structures

### RawDownloadEvent (Client → CentralProgressDispatcher)

```rust
pub struct RawDownloadEvent {
    /// Model identifier (e.g., "meta-llama/Llama-2-7b")
    pub model_id: String,
    
    /// Quantization specification (e.g., "Q4_K_M", "Q8_0", "F16")
    /// Enables selective downloading - only requested quantization
    pub quant: String,
    
    /// Remote URL being downloaded
    pub remote_url: String,
    
    /// Local filepath where file is being written
    pub local_filepath: String,
    
    /// Bytes downloaded so far for this specific file
    pub bytes_downloaded: u64,
    
    /// Total bytes expected for this specific file
    pub total_bytes: u64,
}
```

### ProgressCalculator Snapshot (CentralProgressDispatcher → Displays)

```rust
/// ProgressCalculator: The immutable event flowing through flume channels
/// Contains ALL data and formatting methods - NO custom event types needed
pub struct ProgressCalculator {
    // Internal state (private)
    model_data: ModelProgressData,
    timing_data: TimingData,
    quantization_info: QuantizationInfo,
    // ... other private fields
}

impl ProgressCalculator {
    /// All formatting methods return precisely formatted strings
    /// Displays call these methods and get identical results
    
    pub fn percentage_formatted(&self) -> String {
        // Returns: "76.4%", "100.0%", "---"
    }
    
    pub fn bytes_formatted(&self) -> String {
        // Returns: "1.2 GB / 3.4 GB", "512 MB / 1.0 GB"
    }
    
    pub fn speed_formatted(&self) -> String {
        // Returns: "156 MB/s", "3.8 GB/s", "N/A"
    }
    
    pub fn eta_formatted(&self) -> String {
        // Returns: "2m 34s", "1h 15m", "N/A"
    }
    
    pub fn files_remaining_formatted(&self) -> String {
        // Returns: "12 files remaining", "1 file remaining", "Complete"
    }
    
    pub fn files_completed_formatted(&self) -> String {
        // Returns: "8 of 20 files", "15 of 15 files"
    }
    
    pub fn overall_progress_formatted(&self) -> String {
        // Returns: "Model: 45.2% complete", "Model: Download complete"
    }
    
    pub fn quantization_info_formatted(&self) -> String {
        // Returns: "Quantization: Q4_K_M", "Quantization: F16"
    }
    
    // ... Every formatting permutation as accessor methods
    // All methods fully unit tested for consistency
}
```

## Critical Architectural Guarantees

### 1. Monotonic Progress

```
RawDownloadEvent(bytes: 1000) ──┐
                                 ├─→ FilesystemProgressEvaluator
RawDownloadEvent(bytes: 800)  ───┘     │
(out of order)                         │
                                       ▼
                        Filesystem Check: actual size = 1000 bytes
                                       │
                                       ▼
                        ProgressCalculator: bytes = max(1000, 800) = 1000
                                       │
                        Progress NEVER goes backwards
```

### 2. Perfect Display Consistency

```
ProgressCalculator.percentage_formatted() → "76.4%"
                   │
                   ├─→ CLI displays: "76.4%"
                   ├─→ TUI displays: "76.4%"  
                   └─→ Ratatui displays: "76.4%"

ALL displays get IDENTICAL strings - perfect consistency
```

### 3. Zero Duplicate Logic

```
❌ FORBIDDEN in Displays:
   - format_bytes(1024) → "1.0 KB"
   - calculate_percentage(50, 100) → 50.0
   - Any calculation or formatting functions

✅ REQUIRED in Displays:
   - progress.bytes_formatted() → "1.0 KB / 2.0 KB"
   - progress.percentage_formatted() → "50.0%"
   - Only accessor method calls
```

### 4. Quantization Control Flow

```
User: --quant Q4_K_M
     │
     ▼
Client Selector: Filter manifest → only Q4_K_M files
     │
     ▼
Download Tasks: Include quant in RawDownloadEvent
     │
     ▼
ProgressCalculator: Display quantization info
     │
     ▼
All Displays: Show "Quantization: Q4_K_M"

Result: NO unwanted quantizations downloaded
```

## Performance Characteristics

### Debounced Dispatch

```
Events: ──•─•─•─•─•──────•─•─•──────────•─•─•─•─•──
Time:     0   100ms     200ms        300ms    400ms
          │              │                   │
          ▼              ▼                   ▼
Dispatch: •              •                   •

• Maximum 1 dispatch per 100ms during active periods
• Immediate dispatch after quietude
• Lightning responsiveness without UI spam
```

### Memory Efficiency

```
ProgressCalculator Snapshots:
• Immutable - safe to share across threads
• Efficient cloning via structural sharing
• No locks or mutexes needed
• Perfect for flume channel events
```

This flow chart ensures every component has a clear, specific role with zero overlap, creating a maintainable and performant architecture with perfect display consistency.