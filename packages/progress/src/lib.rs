//! Progress tracking for model downloads
//!
//! This module provides direct channel-based communication for tracking download progress
//! without global state or event buses.

#![recursion_limit = "256"]
// Allow precision loss for UI display - only matters for files >4.5PB
#![allow(clippy::cast_precision_loss)]
// Allow duration truncation - only matters for durations >584 years
#![allow(clippy::cast_possible_truncation)]

pub mod builder;
pub mod calculator;
pub mod central_dispatcher;
pub mod channel;
pub mod debounced;
pub mod debounced_progress;

pub mod events;
pub mod filesystem_evaluator;
pub mod handler;
pub mod handlers;
pub mod manifest;
pub mod memory_ordering;
pub mod orchestration;
pub mod results;
pub mod types;

pub use builder::{InternalProgressBuilder, internal_builder};
pub use calculator::{
    ImmutableFileProgress, ImmutableModelProgress, ImmutableProgressState, ModelData,
    ProgressCalculator, ProgressPercentage, TimingData, from_manifest_and_progress,
    from_raw_progress_data, with_updated_file,
};
// NOTE: Formatting and calculation functions are internal only - displays must use ProgressCalculator accessor methods
pub use central_dispatcher::{
    CentralProgressDispatcher, DispatcherConfig, ValidatedProgress, start_central_dispatcher,
};
pub use debounced::{DebouncedMetrics, DebouncedProgressStream, debounced_progress_stream};
pub use debounced_progress::{DebouncedProgressTrackingWriter, create_progress_tracking_writer};

pub use channel::{ProgressReceiver, ProgressSender};
// Re-export from common to maintain API compatibility
pub use filesystem_evaluator::{
    DownloadStateFile, FilesystemProgressEvaluator, FilesystemValidationResult, ManifestFileInfo,
    ProgressValidationResult, get_default_state_dir, write_download_state,
};
pub use handler::{FileState};
pub use handlers::AtomicFileState;
pub use progresshub_common::RawDownloadEvent;
pub use results::{DownloadResult, DownloadStatus, FileResult, ModelResult, ZeroOneOrMany};
// Re-export from common to maintain API compatibility
pub use progresshub_common::{DownloadProgress, FileStatus, NoOpProgressHandler, ProgressHandler};

// Re-export manifest functionality
pub use manifest::{
    FileInfo, ManifestVersionInfo, RepoManifest, create_hf_client, fetch_hf_manifest,
};

// Re-export DownloadConfig from types
pub use types::DownloadConfig;

// Re-export orchestrator functionality
pub use orchestration::{DownloadOrchestrator, DownloadJob, OrchestratorStats, Priority, get_global_orchestrator};

// Re-export memory ordering utilities for external validation
pub use memory_ordering::{
    MemoryOrderingValidator, CounterDirection, FetchOperationPurpose, SynchronizationPairType,
};
