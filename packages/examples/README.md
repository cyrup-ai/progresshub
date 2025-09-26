# ProgressHub Examples

This directory contains example projects demonstrating different ways to use the ProgressHub library.

## Available Examples

### 1. Default CLI Progress

Shows how to use ProgressHub with its beautiful default CLI progress display.

```bash
# Navigate to the example directory
cd default-cli

# Run with default model (bert-base-uncased)
cargo run --release

# Or specify a different model
cargo run --release -- gpt2
```

### 2. Custom Progress Display

Demonstrates how to create a custom progress display by implementing the `ProgressDisplay` trait.

```bash
# Navigate to the example directory
cd custom-display

# Run with default model (bert-base-uncased)
cargo run --release

# Or specify a different model
cargo run --release -- gpt2
```

## Building and Running

1. Ensure you have Rust and Cargo installed
2. Navigate to the example directory you want to run
3. Build and run with `cargo run --release`

## Dependencies

Each example has its own `Cargo.toml` with the necessary dependencies. The examples use:

- `tokio` for async runtime
- `anyhow` for error handling
- `tracing` for logging
- `indicatif` (in the custom display example) for progress bars

## Notes

- The examples download models to the default Hugging Face cache directory
- You can monitor progress and debug output by setting the `RUST_LOG` environment variable:
  ```bash
  RUST_LOG=debug cargo run --release
  ```
- For best performance, always use `--release` when running the examples
