//! HuggingFace model downloader with progress tracking
//!
//! This crate provides functionality for downloading models from Hugging Face Hub
//! with progress tracking that can be displayed in a TUI interface.

#![recursion_limit = "256"]

use anyhow::{Context, Result};

// TUI-only imports
pub mod ui;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub mod errors;
pub mod task;

// Re-export modules from other crates
pub use progresshub_config as config;
pub use progresshub_progress as progress;

pub use task::{AsyncStream, AsyncTask};

// Re-export types from config and progress crates
pub use config::types::{DownloadConfig as TuiDownloadConfig, DownloadConfigBuilder, OneOrMany};
pub use progresshub_progress::{
    DownloadProgress, DownloadResult, DownloadStatus, FileResult, ModelResult, ProgressCalculator,
    ProgressHandler, ZeroOneOrMany,
};

/// Run TUI application with Pure Flume Channel Event-Driven Architecture
///
/// Public library function that provides TUI interface for model downloads
/// while maintaining all existing architectural patterns and functionality.
///
/// # Arguments
/// * `models` - Vector of model IDs to download
/// * `quantization` - Optional quantization filter (e.g., "Q4_K_M", "Q8_0", "F16")
/// * `force` - Force redownload by clearing cache first
///
/// # Returns
/// Result indicating TUI execution success or error
///
/// # Architecture
/// - Maintains Pure Flume Channel Event-Driven Architecture
/// - Zero allocation event processing with blazing-fast performance
/// - Elegant terminal handling with proper cleanup
/// - RawDownloadEvent → CentralProgressDispatcher → ProgressCalculator → TUI display
pub async fn run_tui_app(
    models: Vec<String>,
    quantization: Option<String>,
    force: bool,
) -> Result<()> {
    tracing::debug!("Starting ProgressHub TUI application via library interface");

    // Setup TUI-specific logging (file-based to avoid terminal conflicts)
    setup_tui_logging().context("Failed to set up TUI logging")?;
    tracing::debug!("TUI logging setup completed");

    // Validate models input
    if models.is_empty() {
        anyhow::bail!("At least one model must be provided to TUI interface");
    }

    tracing::debug!("TUI library: processing {} models", models.len());

    // Use progress crate's internal builder for all business logic (same as CLI)
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

    // Delegate to pure receiver pattern with beautiful TUI display
    run_tui_with_receiver(receiver)
        .await
        .context("TUI receiver display failed")?;

    Ok(())
}

/// Setup TUI-specific logging to file to avoid terminal conflicts
///
/// Configures tracing subscriber to write to file instead of terminal
/// to prevent interference with ratatui terminal output.
///
/// # Returns
/// Result indicating logging setup success or error
///
/// # Architecture
/// - File-based logging for TUI mode to avoid output conflicts
/// - Non-blocking writer for blazing-fast logging performance
/// - Environment filter support for flexible log level control
fn setup_tui_logging() -> Result<()> {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let file_appender = tracing_appender::rolling::never(".", "progresshub-tui.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false), // No ANSI colors in log files
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init()
        .context("Failed to initialize TUI file logging")?;

    Ok(())
}

/// Run TUI application with ProgressCalculator receiver from internal builder
///
/// Consumes ProgressCalculator events from the progress crate's internal builder
/// and displays them using the TUI interface until downloads are complete.
///
/// # Arguments
/// * `progress_receiver` - Flume receiver for ProgressCalculator events
///
/// # Returns
/// Result indicating TUI execution success or error
///
/// # Architecture
/// - Consumes ProgressCalculator events via flume channels
/// - Uses ProgressCalculator.is_complete() to determine when to stop
/// - Maintains Pure Flume Channel Event-Driven Architecture
/// - TUI performs ZERO calculations - only display logic
pub async fn run_tui_with_receiver(
    progress_receiver: flume::Receiver<progresshub_progress::ProgressCalculator>,
) -> Result<()> {
    tracing::debug!("Starting TUI with ProgressCalculator receiver");

    // Setup TUI-specific logging
    setup_tui_logging().context("Failed to set up TUI logging")?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create done channel for completion signaling
    let (_done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();

    // Initialize the App with the progress receiver and done channel
    let mut app = ui::app::app_core::App::new(progress_receiver, done_rx);

    // Run the main UI loop
    app.run_ui_loop(&mut terminal)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    tracing::info!("TUI receiver mode completed successfully");
    Ok(())
}
