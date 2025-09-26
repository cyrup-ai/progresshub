#![recursion_limit = "256"]

//! ProgressHub CLI Library
//!
//! Command-line interface for ProgressHub HuggingFace model downloader with
//! Pure Flume Channel Event-Driven Architecture.
//!
//! This library provides a clean, ergonomic interface for CLI operations while
//! maintaining zero allocation patterns and blazing-fast performance.

pub mod args;
pub mod display;
pub mod logging;

pub use args::CliArgs;

use anyhow::{Context, Result};

use thiserror::Error;

// Tokio imports for TTY-independent stdin reading
use tokio::io::BufReader;

/// CLI-specific error types with zero allocation error handling
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Display error: {0}")]
    Display(String),

    #[error("Argument parsing error: {0}")]
    Arguments(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),
}

/// Type alias for ergonomic error handling throughout CLI
pub type CliResult<T> = Result<T, CliError>;

pub use flume::{Receiver as ProgressReceiver, Sender as ProgressSender};
pub use progresshub_config::environment;
/// Re-export key types for convenience
pub use progresshub_progress::ProgressCalculator;

/// CLI configuration constants
pub mod config {
    /// Default channel buffer size for ProgressCalculator events
    pub const DEFAULT_PROGRESS_BUFFER_SIZE: usize = 1000;

    /// Default timeout for graceful shutdown
    pub const DEFAULT_SHUTDOWN_TIMEOUT_MS: u64 = 500;

    /// Maximum concurrent downloads for CLI mode
    pub const MAX_CONCURRENT_DOWNLOADS: usize = 4;
}

/// Run CLI application with Pure Flume Channel Event-Driven Architecture
///
/// Public library function for executing CLI mode downloads with provided arguments.
/// Uses the progress crate's internal builder for all business logic, then delegates
/// to the pure receiver pattern for display.
///
/// # Arguments
/// * `models` - Vector of model IDs to download
/// * `quantization` - Optional quantization filter (e.g., "Q4_K_M", "Q8_0", "F16")
/// * `force` - Force redownload by clearing cache first
///
/// # Returns
/// Result indicating CLI execution success or error
///
/// # Architecture
/// - All business logic handled by progress crate's internal builder
/// - CLI performs ZERO calculations - only beautiful display logic
/// - RawDownloadEvent → CentralProgressDispatcher → ProgressCalculator → CLI display
pub async fn run_cli_app(
    models: Vec<String>,
    quantization: Option<String>,
    force: bool,
) -> Result<()> {
    tracing::debug!("Starting ProgressHub CLI application via library interface");

    // Validate models input
    if models.is_empty() {
        anyhow::bail!("At least one model must be provided to CLI interface");
    }

    tracing::debug!("CLI library: processing {} models", models.len());

    // Use progress crate's internal builder for all business logic
    let mut builder = progresshub_progress::internal_builder();

    // Configure builder with models
    for model in models {
        builder = builder.model(model);
    }

    // Configure quantization if specified
    if let Some(quant) = quantization {
        builder = builder.quantization(quant);
    }

    // Configure force flag for cache clearing
    builder = builder.force(force);

    // Start the download process and get receiver for ProgressCalculator events
    let receiver = builder.receiver();

    // Delegate to pure receiver pattern with beautiful hierarchical display
    run_cli_with_receiver(receiver)
        .await
        .context("CLI receiver display failed")?;

    Ok(())
}

/// Run CLI application with ProgressCalculator receiver from internal builder
///
/// Consumes ProgressCalculator events from the progress crate's internal builder
/// and displays them using the CLI table interface until downloads are complete.
///
/// # Arguments
/// * `progress_receiver` - Flume receiver for ProgressCalculator events
///
/// # Returns
/// Result indicating CLI execution success or error
///
/// # Architecture
/// - Consumes ProgressCalculator events via flume channels
/// - Uses ProgressCalculator.is_complete() to determine when to stop
/// - Maintains Pure Flume Channel Event-Driven Architecture
/// - CLI performs ZERO calculations - only display logic
/// - Supports graceful interruption: Ctrl+C, Ctrl+D, 'q', 'escape'
pub async fn run_cli_with_receiver(
    progress_receiver: flume::Receiver<progresshub_progress::ProgressCalculator>,
) -> Result<()> {
    tracing::debug!("Starting CLI with ProgressCalculator receiver");
    tracing::info!("🎯 CLI: Waiting for ProgressCalculator events from flume channel...");

    // Initialize the beautiful hierarchical progress renderer
    let mut renderer = display::HierarchicalProgressRenderer::new();

    // Set up TTY-independent stdin reader for keyboard input (q, escape, Ctrl+D)
    // Works in any environment: TTY, non-TTY, pipes, CI/CD, redirected output
    let stdin = tokio::io::stdin();
    let mut stdin_reader = BufReader::new(stdin);

    // Process progress events until complete with full signal and input handling
    loop {
        tokio::select! {
            // Receive progress updates from flume channel
            result = progress_receiver.recv_async() => {
                match result {
                    Ok(progress) => {
                        tracing::debug!("🎯 CLI: Received ProgressCalculator event from channel");
                        // Beautiful hierarchical display - the "sweet hierarchical layout"
                        if let Err(e) = renderer.render_progress(&progress) {
                            tracing::warn!("Progress rendering failed: {}", e);
                        }

                        // Check if downloads are complete
                        let completion_status = progress.is_complete();
                        tracing::debug!(
                            "🔍 CLI checking completion: is_complete()={}, downloaded={}, total={}",
                            completion_status,
                            progress.progress_state.overall_bytes_downloaded,
                            progress.progress_state.overall_total_bytes
                        );

                        if completion_status {
                            tracing::info!(
                                "✅ Downloads marked complete - CLI shutting down (downloaded={}, total={})",
                                progress.progress_state.overall_bytes_downloaded,
                                progress.progress_state.overall_total_bytes
                            );
                            break;
                        }
                    }
                    Err(e) => {
                        // Check if this is a normal channel disconnection (expected when downloads complete)
                        if matches!(e, flume::RecvError::Disconnected) {
                            tracing::info!("✅ CLI: Progress channel disconnected - downloads completed, shutting down");
                        } else {
                            tracing::error!("❌ CLI: Progress receiver error: {:?} - CLI shutting down", e);
                        }
                        break;
                    }
                }
            }

            // Handle Ctrl+C signal
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C signal, gracefully shutting down CLI");
                println!("\n🛑 Download interrupted by Ctrl+C - shutting down gracefully...");
                break;
            }

            // Handle SIGTERM and SIGINT signals (Note: EOF/Ctrl+D handled via stdin above)
            _ = wait_for_sigterm_and_sigint() => {
                tracing::info!("Received SIGTERM/SIGINT signal, gracefully shutting down CLI");
                println!("\n🛑 Download interrupted by SIGTERM - shutting down gracefully...");
                break;
            }

            // Handle keyboard input (q, escape, Ctrl+D) via TTY-independent stdin reading
            stdin_input = async {
                let mut input_byte = [0u8; 1];
                match tokio::io::AsyncReadExt::read(&mut stdin_reader, &mut input_byte).await {
                    Ok(0) => Some(4u8), // EOF detected - treat as Ctrl+D
                    Ok(_) => Some(input_byte[0]),
                    Err(_) => None, // Stdin error - continue processing
                }
            } => {
                match stdin_input {
                    Some(4) => {
                        // Ctrl+D (ASCII 4) or EOF detected
                        tracing::info!("User requested exit via Ctrl+D/EOF");
                        println!("\n🛑 Download interrupted by Ctrl+D - shutting down gracefully...");
                        break;
                    }
                    Some(113) => {
                        // 'q' key (ASCII 113) detected
                        tracing::info!("User requested exit via 'q' key");
                        println!("\n🛑 Download interrupted by 'q' key - shutting down gracefully...");
                        break;
                    }
                    Some(27) => {
                        // Escape key (ASCII 27) detected
                        tracing::info!("User requested exit via Escape key");
                        println!("\n🛑 Download interrupted by Escape key - shutting down gracefully...");
                        break;
                    }
                    Some(_) => {
                        // Other input - ignore and continue processing
                    }
                    None => {
                        // Stdin error - non-fatal, continue processing
                        tracing::debug!("Stdin read error (non-fatal) - continuing");
                    }
                }
            }
        }
    }

    tracing::info!("CLI receiver mode completed successfully");
    Ok(())
}

/// Wait for SIGTERM or SIGINT signals (cross-platform)
/// Note: EOF/Ctrl+D is handled via stdin reading, not signals
async fn wait_for_sigterm_and_sigint() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        tokio::select! {
            _ = async {
                let mut sigterm = signal(SignalKind::terminate())?;
                sigterm.recv().await;
                Ok::<(), anyhow::Error>(())
            } => Ok(()),
            _ = async {
                let mut sigint = signal(SignalKind::interrupt())?;
                sigint.recv().await;
                Ok::<(), anyhow::Error>(())
            } => Ok(()),
        }
    }

    #[cfg(windows)]
    {
        use tokio::signal::windows;
        tokio::select! {
            _ = async {
                let mut ctrl_close = windows::ctrl_close()?;
                ctrl_close.recv().await;
                Ok::<(), anyhow::Error>(())
            } => Ok(()),
            _ = async {
                let mut ctrl_c = windows::ctrl_c()?;
                ctrl_c.recv().await;
                Ok::<(), anyhow::Error>(())
            } => Ok(()),
        }
    }
}
