//! CLI argument parsing for ProgressHub
//!
//! Independent argument parsing with validation and environment variable support
//! for blazing-fast CLI operations with elegant ergonomic interfaces.

use crate::CliResult;
use clap::{ArgGroup, Parser};

/// CLI arguments for ProgressHub with comprehensive validation
///
/// Provides complete CLI interface with model selection, quantization control,
/// cache management, and display options for professional CLI experience.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "progresshub-cli")]
#[command(about = "Command-line HuggingFace model downloader with progress tracking")]
#[command(group(
    ArgGroup::new("progress_control")
        .args(["no_progress"])
        .multiple(false),
))]
#[command(group(
    ArgGroup::new("sorting_control")
        .args(["top", "bottom"])
        .multiple(false),
))]
pub struct CliArgs {
    /// Models to download (required)
    #[arg(short = 'm', long = "model", required = true)]
    pub models: Vec<String>,

    /// Force redownload by clearing cache first
    #[arg(short, long, help = "Force redownload by clearing cache first")]
    pub force: bool,

    /// Disable progress display entirely
    #[arg(long, help = "Disable progress display entirely")]
    pub no_progress: bool,

    /// Sort progress display with most complete files at the top
    #[arg(
        long,
        help = "Sort progress display with most complete files at the top"
    )]
    pub top: bool,

    /// Sort progress display with most complete files at the bottom
    #[arg(
        long,
        help = "Sort progress display with most complete files at the bottom"
    )]
    pub bottom: bool,

    /// Quantization format to download (e.g., Q4_K_M, Q8_0, F16)
    #[arg(
        long,
        help = "Quantization format to download (e.g., Q4_K_M, Q8_0, F16)"
    )]
    pub quant: Option<String>,

    /// Output directory for downloaded models (optional)
    #[arg(
        short = 'o',
        long = "output",
        help = "Output directory for downloaded models"
    )]
    pub output_dir: Option<std::path::PathBuf>,

    /// HuggingFace authentication token
    #[arg(long, help = "HuggingFace authentication token")]
    pub token: Option<String>,

    /// Custom cache directory
    #[arg(long, help = "Custom cache directory for models")]
    pub cache_dir: Option<std::path::PathBuf>,

    /// Verbose output
    #[arg(short, long, help = "Enable verbose output")]
    pub verbose: bool,
}

impl CliArgs {
    /// Parse CLI arguments with validation
    ///
    /// Parses command line arguments using clap and performs comprehensive
    /// validation with elegant error handling and context.
    ///
    /// # Returns
    /// Result containing parsed and validated CliArgs or error
    ///
    /// # Errors
    /// Returns error for invalid arguments, conflicting options, or validation failures
    pub fn parse() -> CliResult<Self> {
        let args = <Self as Parser>::parse();
        args.validate()?;
        Ok(args)
    }

    /// Validate CLI arguments for consistency and correctness
    ///
    /// Performs comprehensive validation of argument combinations,
    /// model name formats, and configuration consistency.
    ///
    /// # Returns
    /// Result indicating validation success or specific error
    ///
    /// # Errors
    /// Returns specific validation errors with context
    pub fn validate(&self) -> CliResult<()> {
        // Validate model names are not empty
        if self.models.is_empty() {
            return Err(crate::CliError::Arguments(
                "At least one model must be specified".to_string(),
            ));
        }

        // Validate model name format (basic check)
        for model in &self.models {
            if model.trim().is_empty() {
                return Err(crate::CliError::Arguments(
                    "Model names cannot be empty or whitespace only".to_string(),
                ));
            }

            // Basic model name validation (org/model format)
            if !model.contains('/') {
                tracing::warn!(
                    "Model '{}' does not follow org/model format - may cause issues",
                    model
                );
            }
        }

        // Validate quantization format if specified
        if let Some(ref quant) = self.quant {
            if quant.trim().is_empty() {
                return Err(crate::CliError::Arguments(
                    "Quantization format cannot be empty".to_string(),
                ));
            }
        }

        // Validate output directory exists if specified
        if let Some(ref output_dir) = self.output_dir {
            if !output_dir.exists() {
                return Err(crate::CliError::Arguments(format!(
                    "Output directory does not exist: {}",
                    output_dir.display()
                )));
            }
            if !output_dir.is_dir() {
                return Err(crate::CliError::Arguments(format!(
                    "Output path is not a directory: {}",
                    output_dir.display()
                )));
            }
        }

        // Validate cache directory if specified
        if let Some(ref cache_dir) = self.cache_dir {
            if let Some(parent) = cache_dir.parent() {
                if !parent.exists() {
                    return Err(crate::CliError::Arguments(format!(
                        "Cache directory parent does not exist: {}",
                        parent.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get effective cache directory
    ///
    /// Returns the cache directory to use, considering CLI argument,
    /// environment variables, and system defaults.
    ///
    /// # Returns
    /// Path to cache directory to use
    pub fn get_cache_dir(&self) -> std::path::PathBuf {
        if let Some(ref cache_dir) = self.cache_dir {
            cache_dir.clone()
        } else {
            progresshub_config::environment::get_hf_hub_cache()
        }
    }

    /// Get effective output directory
    ///
    /// Returns the output directory to use, defaulting to cache directory
    /// if no explicit output directory is specified.
    ///
    /// # Returns
    /// Path to output directory to use
    pub fn get_output_dir(&self) -> std::path::PathBuf {
        self.output_dir
            .clone()
            .unwrap_or_else(|| self.get_cache_dir())
    }

    /// Check if progress display should be shown
    ///
    /// Determines whether progress display should be active based on
    /// CLI arguments and output context.
    ///
    /// # Returns
    /// Boolean indicating whether to show progress display
    #[inline]
    pub fn should_show_progress(&self) -> bool {
        !self.no_progress
    }
}
