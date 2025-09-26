# ProgressHub Architecture: Pure Flume Channel Event-Driven System

## CORE ARCHITECTURAL PRINCIPLE

**Pure Event-Driven Flume Channel Architecture with ProgressCalculator Snapshots as Events**

The Progress crate is the ONLY "brains" in the operation. ProgressCalculator provides ALL formatting methods and flows as immutable events through flume channels to displays that have ZERO formatting logic.

## Event Flow Architecture

```
XET Client ──┐                                                                          ┌─→ CLI Display
             ├─→ RawDownloadEvent ─→ CentralProgressDispatcher ─→ ProgressCalculator ─→ ├─→ TUI Display  
QUIC Client ─┘     (6 fields)          │                          Snapshots (events)   └─→ Ratatui
                                        │                               │
                                        ├─ FilesystemProgressEvaluator  │
                                        └─ Creates immutable snapshots  │
                                                                        │
                                        (ONLY "BRAINS")         (ZERO FORMATTING LOGIC)
```

## Critical Quantization Architecture

**User Quantization Control**: Users specify `--quant Q4_K_M` to download ONLY desired quantization, preventing unwanted downloads.

**Quantization Flow**: CLI input → client_selector filtering → download tasks → raw events → ProgressCalculator snapshots

## Separation of Concerns

### 1. **XET/QUIC Clients** - Pure Download Execution
**Role**: Download files, dispatch raw progress events
**Responsibilities**:
- Download files via their respective protocols
- Send `RawDownloadEvent` (6 fields including quantization) via flume channels
- **NO calculations, NO formatting, NO intelligence**
- **NO direct filesystem state writing**

### 2. **Progress Crate** - CENTRAL INTELLIGENCE HUB (ONLY "BRAINS")
**Role**: ALL progress intelligence, calculations, and formatting
**Components**:
- **CentralProgressDispatcher**: Receives raw events, manages state, creates ProgressCalculator snapshots
- **FilesystemProgressEvaluator**: Prevents out-of-order/backwards progress via filesystem validation  
- **ProgressCalculator**: Immutable snapshots with ALL formatting methods - flows as events via flume
**Responsibilities**:
- **ONLY location for ANY progress intelligence, calculations, or formatting**
- Create immutable ProgressCalculator snapshots that flow as events
- Provide ALL formatting via ProgressCalculator accessor methods
- Ensure monotonic progress guarantees

### 3. **CLI/TUI/Ratatui** - Pure Display Logic
**Role**: Display progress data with ZERO formatting logic
**Responsibilities**:
- Receive `ProgressCalculator` snapshots via flume channels
- **ZERO calculations whatsoever**
- **ZERO formatting logic** - call ProgressCalculator accessor methods only:
  - `progress.percentage_formatted()` → "76.4%"
  - `progress.bytes_formatted()` → "1.2 GB / 3.4 GB"
  - `progress.speed_formatted()` → "156 MB/s"
  - `progress.eta_formatted()` → "2m 34s"
- Handle ONLY colors/styling - no data formatting
- Pure reactive event consumers

### **STRICT RENDERING ARCHITECTURE ENFORCEMENT**

**Official Rendering Systems**:
1. **CLI Mode**: ./forks/termcolor (StandardStream, WriteColor, ColorSpec)
2. **TUI Mode**: ratatui components and widgets
3. **Web Mode**: dioxus components and elements

**Prohibited User-Facing Output**:
- println!(), eprintln!(), print!() are FORBIDDEN for customer display
- These primitive functions are development/debug tools only
- ALL user-facing content MUST use official rendering systems

### 4. **Client Selector** - Orchestration Only
**Role**: Read manifests, select clients, filter by quantization
**Responsibilities**:
- Read HuggingFace manifests
- Determine XET vs QUIC per file basis
- **Quantization filtering** - only select files matching user's quantization specification
- **ONLY orchestration logic**
- **NO progress handling**

## Key Architectural Features

### **ProgressCalculator as the Event**
- **ProgressCalculator** is the immutable event that flows through flume channels
- **Perfect for events** - fully immutable, contains all needed data and formatting methods
- **Perfect consistency** - all displays call same accessor methods, get identical formatted strings
- **Fully unit tested** - every formatting permutation tested in ProgressCalculator
- **No custom events needed** - ProgressCalculator IS the event

### **Zero Formatting Logic in Displays**
- **CLI**: `progress.percentage_formatted()` → displays "76.4%"
- **TUI**: `progress.bytes_formatted()` → displays "1.2 GB / 3.4 GB"  
- **Ratatui**: `progress.speed_formatted()` → displays "156 MB/s"
- **Perfect consistency** - all displays get identical strings
- **Views handle ONLY colors/styling** - no data formatting logic

### **Monotonic Progress Guarantee**
- **Problem**: Async events can arrive out of order, causing progress to move backwards
- **Solution**: FilesystemProgressEvaluator validates against actual filesystem state
- **Result**: Progress never goes backwards, always monotonically increasing

### **Debounced Intelligent Dispatch**
- **100ms throttling**: Maximum one ProgressCalculator snapshot every 100ms during active periods
- **Immediate dispatch**: After any period of quietude, first snapshot dispatched immediately
- **Lightning responsiveness**: No artificial delays when downloads start/complete
- **Efficient UI updates**: Prevents UI spam while maintaining responsiveness

### **Pure Flume Channel Communication**
- **NO shared state** - pure event-driven message passing
- **NO mutexes, locks, or semaphores** - flume channels handle all synchronization
- **Direct channel communication** - no global event bus
- **ProgressCalculator snapshots flow as events** - no custom event types needed

## Critical Design Constraints

### **ZERO DUPLICATE CALCULATIONS AND FORMATTING**
- **Progress crate ONLY** location for any calculations or formatting
- **ProgressCalculator snapshots** provide ALL formatting via accessor methods
- **NO formatting functions** anywhere else in the codebase
- **NO percentage calculations** in CLI/TUI - call `.percentage_formatted()`
- **NO byte formatting** outside ProgressCalculator - call `.bytes_formatted()`

### **COMPLEXITY IS INTENTIONAL**
- This is **DESIGNED TO BE COMPLEX** - complexity enables proper separation of concerns
- Each crate has **DISTINCT and PROPER** separation of concerns
- **NO simplification allowed** - simplification breaks the architecture

### **NO STATEFUL SHARED STATE**
- **ONLY flume channels** for inter-component communication
- **NO global state variables**
- **NO Arc<Mutex<T>>** patterns
- **Event-driven reactive** architecture only

## Event Type Definitions

### **RawDownloadEvent** (XET/QUIC → CentralProgressDispatcher)
```rust
pub struct RawDownloadEvent {
    pub model_id: String,
    pub quant: String,           // Quantization specification (e.g., "Q4_K_M")
    pub remote_url: String,
    pub local_filepath: String,
    pub bytes_downloaded: u64,  
    pub total_bytes: u64,
}
```

### **ProgressCalculator Snapshots** (CentralProgressDispatcher → CLI/TUI/Ratatui)
```rust
// ProgressCalculator is the immutable event that flows via flume
// Contains ALL formatting methods - no custom event types needed
impl ProgressCalculator {
    pub fn percentage_formatted(&self) -> String         // "76.4%"
    pub fn bytes_formatted(&self) -> String              // "1.2 GB / 3.4 GB"
    pub fn speed_formatted(&self) -> String              // "156 MB/s" 
    pub fn eta_formatted(&self) -> String                // "2m 34s"
    pub fn files_remaining_formatted(&self) -> String    // "12 files remaining"
    pub fn files_completed_formatted(&self) -> String    // "8 of 20 files"
    pub fn overall_progress_formatted(&self) -> String   // "Model: 45.2% complete"
    // ... every formatting permutation as accessor methods
}
```

## Workspace Package Roles

### **packages/progresshub** - PUBLIC API SURFACE
- **Library Interface**: External users import this crate (`use progresshub::*`)
- **Single Binary Export**: ONE AND ONLY binary the workspace exports (`cargo run --bin progresshub`)
- **Mode Delegation**: Routes to CLI or TUI implementations internally based on flags
- **Builder Pattern API**: External facing builder interface for library usage
- **Public Entry Points**: `run_cli_app()`, `run_tui_app()`, `interface::cli()`, `interface::tui()`
- **Re-exports**: Provides clean public API surface by re-exporting functionality from internal packages

### **packages/cli** - CLI DISPLAY IMPLEMENTATION
- Command-line table display and terminal output using termcolor architecture
- **ZERO formatting logic** - consumes ProgressCalculator snapshots via flume
- **ZERO calculations** - calls ProgressCalculator accessor methods only
- Pure display logic with professional terminal output (StandardStream, WriteColor, ColorSpec)
- **NO business logic** - purely reactive display consumer

### **packages/tui** - TUI DISPLAY IMPLEMENTATION  
- Ratatui-based interactive terminal user interface
- **ZERO formatting logic** - consumes ProgressCalculator snapshots via flume
- **ZERO calculations** - calls ProgressCalculator accessor methods only
- Subscribes to lib_bandwydth events for bandwidth monitoring alongside progress events
- **NO business logic** - purely reactive display consumer

### **packages/progress** - CENTRAL INTELLIGENCE HUB
- **CENTRAL INTELLIGENCE HUB (ONLY "BRAINS")**
- CentralProgressDispatcher (receives raw events, creates ProgressCalculator snapshots)
- FilesystemProgressEvaluator (reactive filesystem checking)
- ProgressCalculator (immutable snapshots with ALL formatting methods)
- **ONLY location for progress logic, calculations, and formatting**
- **ALL filesystem operations** when called by packages/progresshub (e.g., cache management)

### **client_xet**
- XET protocol implementation
- Downloads via content-addressable storage
- Sends RawDownloadEvent (6 fields including quant) via flume
- **NO progress calculations, NO formatting**

### **client_quic** 
- QUIC protocol implementation
- Downloads via QUIC transport
- Sends RawDownloadEvent (6 fields including quant) via flume
- **NO progress calculations, NO formatting**

### **client_selector**
- Multi-download orchestration
- Manifest reading and backend selection
- **Quantization filtering** - only download requested quantization
- **NO progress handling, ONLY orchestration**

### **config**
- Environment variable handling (HF_HOME, HF_HUB_CACHE, HF_TOKEN)
- HuggingFace standard paths and quantization specifications
- Configuration management only

This architecture ensures **clean separation of concerns**, **blazing performance** through reactive event-driven design, **zero duplicate logic** through centralized intelligence in the Progress crate, and **perfect display consistency** through ProgressCalculator accessor methods.