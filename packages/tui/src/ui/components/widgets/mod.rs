//! Widget components for the ProgressHub UI
//!
//! This module contains reusable UI widgets that can be composed
//! to build the application interface.

pub mod bottom_bar;
// Decomposed download graph modules
pub mod download_stats;
pub mod graph_effects;
pub mod graph_renderer;
pub mod graph_state;
pub mod graph_widget;
pub mod header;
pub mod model_list;
pub mod model_list_state;
pub mod overall_progress;
pub mod remaining;

// Minimal re-exports required by existing modules. Keep surface area small.
pub use download_stats::DownloadStatsComponent;
pub use graph_state::DownloadGraphState;
pub use graph_widget::DownloadGraphWidget;
pub use model_list_state::ModelListState;
