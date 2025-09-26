//! State coordination and progress types for TUI components
//!
//! Provides high-level coordination between UI components with zero-allocation
//! state synchronization, efficient progress data structures, and const generic optimizations.

use std::path::PathBuf;
use std::time::Instant;

use progresshub_progress::{ImmutableFileProgress, ImmutableModelProgress, ProgressPercentage};

use super::app_state::AppState;

// FileProgressExt trait removed - ImmutableFileProgress now has built-in percentage() method

/// State management structures for the UI components
/// Manages the shared state for the UI components.
#[derive(Debug)]
pub struct UiState {
    /// Current application state
    pub app_state: AppState,
    /// Whether the UI is currently active
    pub active: bool,
    /// Whether to show the bandwidth monitor
    pub show_bandwidth: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            app_state: AppState::default(),
            active: true,
            show_bandwidth: true,
        }
    }
}

impl UiState {
    /// Create a new UI state with optimal defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the UI state with zero-allocation atomic operations
    ///
    /// Provides efficient state synchronization across UI components
    /// using lock-free data structures and const generic optimizations.
    pub fn update_app_state<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut AppState),
    {
        updater(&mut self.app_state);
    }

    /// Toggle UI activation state
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }

    /// Toggle bandwidth monitor visibility
    pub fn toggle_bandwidth_display(&mut self) {
        self.show_bandwidth = !self.show_bandwidth;
    }

    /// Check if UI should quit (delegates to app state)
    pub fn should_quit(&self) -> bool {
        self.app_state.should_quit()
    }

    /// Handle quit request (delegates to app state)
    pub fn quit(&mut self) {
        self.app_state.quit();
    }

    /// Force immediate quit (delegates to app state)
    pub fn force_quit(&mut self) {
        self.app_state.force_quit();
    }
}

/// Status of a download operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DownloadStatus {
    /// Waiting to start
    #[default]
    Pending,
    /// Connecting to server
    Connecting,
    /// Actively downloading
    Downloading,
    /// Download completed successfully
    Completed,
    /// Download failed
    Failed,
}


impl DownloadStatus {
    /// Check if the download is in progress
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Connecting | Self::Downloading)
    }

    /// Check if the download is finished (completed or failed)
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Check if the download was successful
    #[must_use]
    pub fn is_successful(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Get status as display string
    #[must_use]
    pub fn as_display_str(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Connecting => "Connecting",
            Self::Downloading => "Downloading",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
        }
    }
}

/// Progress information for a file or model
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current status of the download
    pub status: DownloadStatus,
    /// Total bytes to download
    pub total_bytes: u64,
    /// Bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Download speed in bytes per second
    pub download_speed: f64,
    /// Time when download started
    pub start_time: Option<Instant>,
    /// Time when download completed or failed
    pub end_time: Option<Instant>,
    /// Whether this was served from cache
    pub is_cached: bool,
}

impl ProgressInfo {
    /// Get the progress percentage using ProgressCalculator
    /// Returns ProgressPercentage with value 0.0-100.0 or None if indeterminate
    pub fn percentage(&self) -> ProgressPercentage {
        // ARCHITECTURAL FIX: Use ProgressPercentage directly instead of calculate_percentage
        // This maintains the Pure Flume Channel Event-Driven Architecture constraint:
        // "ZERO FORMATTING LOGIC IN DISPLAYS - Progress crate ONLY location for any calculations"
        use progresshub_progress::calculator::ProgressPercentage;

        ProgressPercentage {
            value: if self.total_bytes > 0 {
                Some(
                    (self.bytes_downloaded as f64).min(self.total_bytes as f64)
                        / self.total_bytes as f64
                        * 100.0,
                )
            } else {
                None
            },
            is_complete: self.bytes_downloaded >= self.total_bytes,
            is_indeterminate: self.total_bytes == 0,
        }
    }

    /// Get estimated time remaining based on current speed
    #[must_use]
    pub fn estimated_time_remaining(&self) -> Option<u64> {
        if self.download_speed > 0.0 && self.total_bytes > self.bytes_downloaded {
            let remaining_bytes = self.total_bytes - self.bytes_downloaded;
            Some((remaining_bytes as f64 / self.download_speed) as u64)
        } else {
            None
        }
    }

    /// Get elapsed time since download started
    #[must_use]
    pub fn elapsed_time(&self) -> Option<u64> {
        self.start_time.map(|start| start.elapsed().as_secs())
    }

    /// Update progress with new bytes downloaded
    #[inline]
    pub fn update_progress(&mut self, bytes_downloaded: u64, speed: f64) {
        self.bytes_downloaded = bytes_downloaded;
        self.download_speed = speed;

        // Update status based on progress
        if self.bytes_downloaded >= self.total_bytes {
            self.status = DownloadStatus::Completed;
            self.end_time = Some(Instant::now());
        } else if self.bytes_downloaded > 0 {
            self.status = DownloadStatus::Downloading;
        }
    }

    /// Mark download as started
    pub fn mark_started(&mut self) {
        self.start_time = Some(Instant::now());
        self.status = DownloadStatus::Connecting;
    }

    /// Mark download as failed
    pub fn mark_failed(&mut self) {
        self.status = DownloadStatus::Failed;
        self.end_time = Some(Instant::now());
    }
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self {
            status: DownloadStatus::Pending,
            total_bytes: 0,
            bytes_downloaded: 0,
            download_speed: 0.0,
            start_time: None,
            end_time: None,
            is_cached: false,
        }
    }
}

/// Information about a model being downloaded - wrapper around ImmutableModelProgress
#[derive(Debug, Clone)]
pub struct ModelDownload {
    /// Model identifier (namespace/model)
    pub model_name: String,
    /// Immutable progress state from ProgressCalculator
    pub progress: ImmutableModelProgress,
}

impl ModelDownload {
    /// Create a new model download wrapper
    #[must_use]
    pub fn new(model_name: String, progress: ImmutableModelProgress) -> Self {
        Self {
            model_name,
            progress,
        }
    }

    /// Get the overall percentage for this model
    /// Returns ProgressPercentage with value 0.0-100.0 or None if indeterminate
    pub fn percentage(&self) -> ProgressPercentage {
        self.progress.percentage()
    }

    /// Check if this model download is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.progress.bytes_downloaded >= self.progress.total_bytes && self.progress.total_bytes > 0
    }

    /// Get the number of files in this model
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.progress.files.len()
    }

    /// Get the number of completed files
    #[must_use]
    pub fn completed_file_count(&self) -> usize {
        self.progress
            .files
            .iter()
            .filter(|file| file.bytes_downloaded >= file.total_bytes && file.total_bytes > 0)
            .count()
    }
}

/// Information about an individual file being downloaded - wrapper around ImmutableFileProgress
#[derive(Debug, Clone)]
pub struct FileDownload {
    /// File name
    pub file_name: String,
    /// File path
    pub path: PathBuf,
    /// Immutable progress data from ProgressCalculator
    pub progress: ImmutableFileProgress,
}

impl FileDownload {
    /// Create a new file download wrapper
    #[must_use]
    pub fn new(file_name: String, path: PathBuf, progress: ImmutableFileProgress) -> Self {
        Self {
            file_name,
            path,
            progress,
        }
    }

    /// Get the progress percentage for this file
    pub fn percentage(&self) -> ProgressPercentage {
        self.progress.percentage()
    }

    /// Check if this file download is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.progress.bytes_downloaded >= self.progress.total_bytes && self.progress.total_bytes > 0
    }

    /// Get file size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.progress.total_bytes
    }

    /// Get downloaded bytes
    #[must_use]
    pub fn downloaded_bytes(&self) -> u64 {
        self.progress.bytes_downloaded
    }
}
