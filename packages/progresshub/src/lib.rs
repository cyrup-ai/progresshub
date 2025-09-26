//! HuggingFace model downloader with progress tracking
//!
//! This crate provides functionality for downloading models from Hugging Face Hub
//! with progress tracking that can be displayed in CLI or TUI interfaces.
//! Single library interface for ProgressHub HuggingFace model downloader
//! providing both CLI and TUI functionality for external usage and closures.
//! Maintains Pure Flume Channel Event-Driven Architecture.

#![recursion_limit = "256"]

// Internal modules
mod builder;
mod custom_progress;
mod termcolor;

// Re-export the main entry points for external usage
pub use builder::ProgressHub;
pub use progresshub_cli::run_cli_app;
pub use progresshub_tui::run_tui_app;

// Re-export termcolor utility
pub use termcolor::Termcolor;

// Re-export modules from other crates
pub use progresshub_config as config;
pub use progresshub_progress as progress;

// Re-export bandwidth types directly from lib_bandwydth
// pub use lib_bandwydth::{BandwidthClass, BandwidthStats};  // Temporarily disabled due to async-graphql conflict

// Re-export backend selection types only

// Re-export task types from TUI
pub use progresshub_tui::{AsyncStream, AsyncTask};

// Re-export types from config and progress crates
pub use progresshub_config::types::{DownloadConfig as TuiDownloadConfig, DownloadConfigBuilder};
pub use progresshub_config::{OneOrMany, environment};
pub use progresshub_progress::{
    DownloadProgress, DownloadResult, DownloadStatus, FileResult, ModelResult, ProgressCalculator,
    ProgressHandler, ProgressReceiver, ZeroOneOrMany,
};

// Re-export flume channels for external event handling
pub use flume::{Receiver, Sender};

/// Unified interface for external usage with closures
pub mod interface {
    use super::*;
    use anyhow::Result;

    /// CLI mode interface for external usage
    pub async fn cli(models: Vec<String>, quantization: Option<String>, force: bool) -> Result<()> {
        run_cli_app(models, quantization, force).await
    }

    /// TUI mode interface for external usage  
    pub async fn tui(models: Vec<String>, quantization: Option<String>, force: bool) -> Result<()> {
        run_tui_app(models, quantization, force).await
    }
}

/// Configuration and utility types for external usage
pub mod types {
    pub use progresshub_config::types::*;
    pub use progresshub_progress::ZeroOneOrMany;
}
