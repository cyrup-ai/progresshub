//! Logging setup for the application.

use anyhow::{Context, Result};
use std::env;
use std::fs::File;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Set up logging with both console and file output.
///
/// Respects RUST_LOG environment variable for filtering.
/// In CLI mode, logs to both stderr and a file.
/// In TUI mode, logs only to file to avoid corrupting display.
pub fn setup_logging() -> Result<()> {
    // Get log file path - workspace root
    let mut log_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
    } else {
        env::current_dir()?
    };

    if log_path.ends_with("tui") {
        log_path.pop();
    }
    log_path.push("progresshub.log");

    // Create log file
    let log_file = File::create(&log_path)
        .with_context(|| format!("Failed to create log file at {}", log_path.display()))?;

    // Configure environment filter - defaults to "warn" if RUST_LOG not set
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    // File layer - always active
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_file)
        .with_ansi(false); // No ANSI colors in file

    // Console layer - only active if not in TUI mode (determine via args later)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr) // Use stderr to avoid corrupting stdout
        .with_ansi(true); // Colors on console

    // Initialize subscriber with both layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .try_init()
        .with_context(|| "Failed to initialize tracing subscriber")?;

    tracing::info!("Logging initialized - writing to {}", log_path.display());

    Ok(())
}
