# ProgressHub

**Production-grade Rust application implementing Pure Flume Channel Event-Driven Architecture for downloading models from HuggingFace Hub with real-time progress tracking and precise quantization control.**

## Features

- **Pure Event-Driven Architecture**: Flume channel-based communication with zero shared state
- **Dual Interface**: CLI mode (default) and TUI mode (`--tui`) 
- **Quantization Control**: Download only specified quantizations (e.g., `--quant Q4_K_M`)
- **Real-time Progress**: Live progress tracking with monotonic guarantees
- **Multiple Protocols**: XET and QUIC download backends with automatic selection
- **Professional UI**: Termcolor CLI output and Ratatui TUI interface

## Quick Start

### Installation

```bash
cargo build --release
```

### Basic Usage

```bash
# CLI mode (default)
cargo run --bin progresshub -- --model meta-llama/Llama-2-7b

# TUI mode
cargo run --bin progresshub -- --tui --model microsoft/DialoGPT-medium

# Specific quantization
cargo run --bin progresshub -- --quant Q4_K_M --model meta-llama/Llama-2-7b

# Multiple models
cargo run --bin progresshub -- --model model1 --model model2 --model model3

# Force redownload
cargo run --bin progresshub -- --force --model meta-llama/Llama-2-7b
```

## Architecture Overview

ProgressHub implements **Pure Flume Channel Event-Driven Architecture**:

```
XET Client ──┐                                                                          ┌─→ CLI Display
             ├─→ RawDownloadEvent ─→ CentralProgressDispatcher ─→ ProgressCalculator ─→ ├─→ TUI Display  
QUIC Client ─┘                          (ONLY "BRAINS")              (Events)          └─→ Library API
```

### Package Roles

- **`packages/progresshub`** - **PUBLIC API SURFACE**: Single binary export and library interface
- **`packages/cli`** - **CLI DISPLAY**: Command-line interface with termcolor output
- **`packages/tui`** - **TUI DISPLAY**: Ratatui-based terminal user interface  
- **`packages/progress`** - **CENTRAL INTELLIGENCE**: All progress calculations and formatting
- **`client_xet`/`client_quic`** - **DOWNLOAD CLIENTS**: Protocol implementations only
- **`client_selector`** - **ORCHESTRATION**: Manifest reading and quantization filtering

## Configuration

Environment variables:

- **`HF_TOKEN`** - HuggingFace authentication token for private models
- **`HF_HOME`** - Override default HuggingFace cache directory  
- **`HF_HUB_CACHE`** - Specific cache directory for model storage
- **`RUST_LOG`** - Logging configuration (debug, info, warn, error)

## Development

### Build Commands

```bash
# Build all packages
cargo build

# Release build
cargo build --release

# Build specific package
cargo build -p progresshub-progress
```

### Testing

```bash
# Run all tests (preferred - fast parallel execution)
cargo nextest run

# Run tests for specific package
cargo nextest run -p progresshub-progress
```

### Code Quality

```bash
# Lint with strict workspace settings
cargo clippy --workspace --all-targets --all-features

# Format check
cargo fmt --check
```

## Library Usage

ProgressHub can be used as a library:

```rust
use progresshub::{run_cli_app, run_tui_app};

// CLI mode
let models = vec!["meta-llama/Llama-2-7b".to_string()];
run_cli_app(models, Some("Q4_K_M".to_string()), false).await?;

// TUI mode  
let models = vec!["microsoft/DialoGPT-medium".to_string()];
run_tui_app(models, None, false).await?;

// Builder pattern
use progresshub::ProgressHub;
let result = ProgressHub::new()
    .model("meta-llama/Llama-2-7b")
    .quantization("Q4_K_M")
    .cli()
    .await?;
```

## Architecture Principles

### Event-Driven Design
- **Pure flume channels** for all inter-component communication
- **No shared state** - zero `Arc<Mutex<T>>` patterns
- **ProgressCalculator snapshots** flow as immutable events

### Separation of Concerns
- **Progress crate ONLY** handles calculations and formatting
- **Display crates** consume formatted strings via accessor methods
- **Download clients** send raw events only - no intelligence

### Professional Rendering
- **CLI mode**: `./forks/termcolor` for professional terminal output
- **TUI mode**: `ratatui` for interactive terminal interfaces
- **Prohibited**: `println!()`, `print!()` for user-facing display

## Contributing

1. Follow the **Pure Flume Channel Event-Driven Architecture**
2. All progress logic belongs in the `progress` crate only
3. Display code must call `ProgressCalculator` accessor methods only
4. Use `cargo nextest run` for testing
5. Maintain strict clippy compliance

## License

MIT License - see LICENSE file for details.