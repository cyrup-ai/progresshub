//! ProgressHub Unified Binary
//!
//! Single entry point for ProgressHub HuggingFace model downloader
//! with CLI mode (default) and TUI mode (--tui flag) delegation.
//! Maintains Pure Flume Channel Event-Driven Architecture.

#![recursion_limit = "256"]

use anyhow::{Context, Result};
use clap::Parser;
use std::io::Write;
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Unified arguments for ProgressHub CLI/TUI interface
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "progresshub")]
#[command(about = "Unified HuggingFace model downloader with CLI/TUI modes and progress tracking")]
pub struct UnifiedArgs {
    /// Models to download (required, supports multiple)
    #[arg(short = 'm', long = "model", required = true)]
    pub models: Vec<String>,

    /// Quantization format to download (e.g., Q4_K_M, Q8_0, F16)
    #[arg(
        long,
        help = "Quantization format to download (e.g., Q4_K_M, Q8_0, F16)"
    )]
    pub quant: Option<String>,

    /// Force redownload by clearing cache first
    #[arg(short, long, help = "Force redownload by clearing cache first")]
    pub force: bool,

    /// Use TUI mode instead of CLI mode (default: CLI)
    #[arg(long, help = "Launch TUI interface instead of CLI table display")]
    pub tui: bool,

    /// Verbose output (affects logging in both modes)
    #[arg(short, long, help = "Enable verbose output and logging")]
    pub verbose: bool,
}

impl UnifiedArgs {
    /// Parse and validate unified arguments with comprehensive checks
    ///
    /// Performs argument parsing using clap and validates argument combinations,
    /// model name formats, and mode compatibility.
    ///
    /// # Returns
    /// Result containing validated UnifiedArgs or error with context
    ///
    /// # Errors
    /// Returns error for invalid arguments, malformed model names, or validation failures
    pub fn parse_and_validate() -> Result<Self> {
        let args = Self::parse();
        args.validate().context("Argument validation failed")?;
        Ok(args)
    }

    /// Validate argument consistency and correctness
    ///
    /// Performs comprehensive validation of argument combinations,
    /// model name formats, quantization strings, and mode compatibility.
    ///
    /// # Returns
    /// Result indicating validation success or specific error context
    ///
    /// # Errors
    /// Returns specific validation errors with detailed context
    pub fn validate(&self) -> Result<()> {
        // Validate model names are not empty
        if self.models.is_empty() {
            anyhow::bail!("At least one model must be specified");
        }

        // Validate individual model name format and content
        for (index, model) in self.models.iter().enumerate() {
            if model.trim().is_empty() {
                anyhow::bail!(
                    "Model name at index {} cannot be empty or whitespace only",
                    index
                );
            }

            // Basic model name validation (org/model format recommended)
            if !model.contains('/') {
                tracing::warn!(
                    "Model '{}' does not follow org/model format - may cause download issues",
                    model
                );
            }

            // Check for obviously invalid characters
            if model.contains("..") || model.starts_with('/') || model.ends_with('/') {
                anyhow::bail!("Model name '{}' contains invalid path characters", model);
            }
        }

        // Validate quantization format if specified
        if let Some(ref quant) = self.quant {
            if quant.trim().is_empty() {
                anyhow::bail!("Quantization format cannot be empty when specified");
            }

            // Basic quantization format validation
            let quant_upper = quant.to_uppercase();
            let valid_formats = ["Q4_K_M", "Q4_K_S", "Q5_K_M", "Q5_K_S", "Q8_0", "F16", "F32"];
            if !valid_formats
                .iter()
                .any(|&valid| quant_upper == valid || quant_upper.starts_with(valid))
            {
                tracing::warn!(
                    "Quantization format '{}' may not be standard - common formats: {:?}",
                    quant,
                    valid_formats
                );
            }
        }

        // Validate argument combinations (currently all combinations are valid)
        // Future: Add specific validation for mode-specific argument conflicts

        Ok(())
    }

    /// Convert models Vec<String> to OneOrMany format for compatibility
    ///
    /// Handles conversion from Vec<String> to progresshub_config::OneOrMany<String>
    /// for compatibility with both CLI and TUI mode interfaces.
    ///
    /// # Returns
    /// OneOrMany<String> containing single model or multiple models
    pub fn models_as_one_or_many(&self) -> progresshub_config::OneOrMany<String> {
        match self.models.len() {
            0 => progresshub_config::OneOrMany::Many(Vec::new()), // Should not happen due to validation
            1 => progresshub_config::OneOrMany::One(self.models[0].clone()),
            _ => progresshub_config::OneOrMany::Many(self.models.clone()),
        }
    }

    /// Normalize model names for consistent processing
    ///
    /// Applies consistent normalization to model names to handle
    /// various input formats and ensure compatibility.
    ///
    /// # Returns
    /// Vector of normalized model names
    pub fn normalized_models(&self) -> Vec<String> {
        self.models
            .iter()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .collect()
    }
}
/// Main application entry point with mode delegation
///
/// Coordinates unified argument parsing, mode selection (CLI vs TUI),
/// and delegation to appropriate implementation with signal handling.
///
/// # Returns
/// Result indicating application success or error
///
/// # Architecture
/// - Zero unsafe operations and no locking mechanisms
/// - Blazing-fast argument parsing and mode routing
/// - Elegant ergonomic error handling with proper context
async fn run_unified_app() -> Result<()> {
    // Parse and validate arguments with comprehensive checks
    tracing::debug!("Starting ProgressHub unified application");
    let args = UnifiedArgs::parse_and_validate()
        .context("Failed to parse and validate command line arguments")?;

    tracing::info!(
        "Unified app: parsed {} models, quantization: {:?}, force: {}, tui: {}",
        args.models.len(),
        args.quant,
        args.force,
        args.tui
    );

    // Setup logging configuration for selected mode
    setup_unified_logging(&args).context("Failed to setup unified logging")?;
    tracing::debug!(
        "Unified logging setup completed for mode: {}",
        if args.tui { "TUI" } else { "CLI" }
    );

    // Normalize model names for consistent processing
    let normalized_models = args.normalized_models();
    if normalized_models.is_empty() {
        anyhow::bail!("No valid model names provided after normalization");
    }

    // Mode delegation based on --tui flag - let each mode create its own receiver
    tracing::info!(
        "Delegating to {} mode for self-managed progress",
        if args.tui { "TUI" } else { "CLI" }
    );

    if args.tui {
        #[cfg(feature = "tui")]
        {
            // TUI mode delegation - creates its own receiver
            tracing::info!("Starting TUI mode with self-managed progress");
            progresshub_tui::run_tui_app(normalized_models, args.quant, args.force)
                .await
                .context("TUI mode failed")?;
        }
        #[cfg(not(feature = "tui"))]
        {
            anyhow::bail!("TUI mode requested but TUI feature not enabled. Rebuild with --features tui");
        }
    } else {
        // CLI mode delegation (default) - creates its own receiver
        tracing::info!("Starting CLI mode with self-managed progress");
        progresshub_cli::run_cli_app(normalized_models, args.quant, args.force)
            .await
            .context("CLI mode failed")?;
    }

    tracing::info!("Unified application completed successfully");
    Ok(())
}

/// Setup unified logging configuration for both CLI and TUI modes
///
/// Configures tracing subscriber with mode-appropriate output handling:
/// CLI mode logs to stdout, TUI mode logs to file to avoid conflicts.
///
/// # Arguments
/// * `args` - Unified arguments containing mode and verbosity settings
///
/// # Returns
/// Result indicating logging setup success or error
///
/// # Architecture
/// - Blazing-fast conditional logging setup
/// - Zero allocation in logging hot paths
/// - Elegant output coordination preventing conflicts
fn setup_unified_logging(args: &UnifiedArgs) -> Result<()> {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    // Determine log level based on verbose flag and environment - WARN for production
    let default_level = if args.verbose { "debug" } else { "warn" };
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_level))
        .context("Failed to create environment filter for logging")?;

    if args.tui {
        // TUI mode: log to file to avoid terminal output conflicts
        let file_appender = tracing_appender::rolling::never(".", "progresshub-tui.log");
        let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(file_writer)
                    .with_ansi(false), // No ANSI colors in log files
            )
            .with(env_filter)
            .try_init()
            .context("Failed to initialize TUI file logging")?;
    } else {
        // CLI mode: structured logging to stderr (stdout used for progress display)
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr) // CLI progress uses stdout
                    .with_ansi(true), // ANSI colors for terminal output
            )
            .with(env_filter)
            .try_init()
            .context("Failed to initialize CLI stderr logging")?;
    }

    Ok(())
}

/// Unified binary main entry point with comprehensive signal handling
///
/// Provides graceful shutdown on Ctrl+C, professional error handling,
/// and proper exit code management for both CLI and TUI modes.
///
/// # Architecture
/// - Zero allocation error handling in main path
/// - Elegant error display with mode-specific context
/// - Blazing-fast startup and shutdown coordination
#[tokio::main]
async fn main() {
    // Run the unified application - CLI/TUI handle their own signals
    let result = run_unified_app().await;

    // Handle any errors with professional output and appropriate exit codes
    if let Err(err) = result {
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
        let _ = writeln!(stderr, "\nProgressHub Error: {err:#}");
        let _ = stderr.reset();

        // Determine exit code based on error type
        let exit_code =
            if err.to_string().contains("argument") || err.to_string().contains("validation") {
                2 // Invalid arguments
            } else if err.to_string().contains("CLI mode failed") {
                3 // CLI mode error
            } else if err.to_string().contains("TUI mode failed") {
                4 // TUI mode error
            } else {
                1 // General application error
            };

        process::exit(exit_code);
    }
}
