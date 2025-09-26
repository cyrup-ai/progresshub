//! CLI Display Components
//!
//! Pure display logic for CLI interface consuming ProgressCalculator events
//! from flume channels with zero-allocation patterns and blazing-fast performance.

pub mod display_manager;
pub mod icons;
pub mod output_formatter;
pub mod progress_display;
pub mod progress_renderer;
pub mod result_processor;
pub mod theme;

// Re-export key components for convenience
pub use display_manager::run_cli_table_display_with_receiver;
pub use icons::{CliIcons, CliSpinner, CliStatusIcon, StatusType};
pub use progress_renderer::HierarchicalProgressRenderer;
pub use result_processor::{CliResultDisplay, DownloadResultProcessor};
pub use theme::{CliColorSpecs, CliColors, CliStyling, CliTheme, TerminalCapabilities};
