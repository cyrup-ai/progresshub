//! Download orchestration module - ALL "brains" functionality
//!
//! This module contains orchestration intelligence for downloads
//! to maintain Pure Flume Channel Event-Driven Architecture compliance.
//!
//! As the central intelligence hub, the progress crate houses all download
//! coordination, task management, and orchestration logic.

pub mod download_tasks;
pub mod download_orchestrator;

// Re-export orchestration types
pub use crate::DownloadConfig;
pub use download_orchestrator::{DownloadOrchestrator, DownloadJob, OrchestratorStats, Priority, get_global_orchestrator};
