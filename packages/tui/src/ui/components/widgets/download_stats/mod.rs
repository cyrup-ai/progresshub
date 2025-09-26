//! Download statistics widget - PURE EVENT-DRIVEN ONLY.
//!
//! Consumes ONLY bandwidth and ProgressCalculator event streams via flume channels.

pub mod state_manager;
pub mod types;

// Re-export main component - PURE EVENT-DRIVEN ONLY
pub use types::DownloadStatsComponent;
