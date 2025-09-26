//! Progress-related types for download tracking
//!
//! This module contains all types related to progress reporting and handling,
//! extracted from the config package to maintain proper separation of concerns.

/// Status of a file download
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// File is queued for download
    Pending,
    /// File is currently being downloaded
    Downloading,
    /// File download completed successfully
    Completed,
    /// File download failed
    Failed,
    /// File download encountered an error
    Error,
}

/// Progress information for a file being downloaded
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Path of the file being downloaded
    pub path: String,
    /// Number of bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total size of the file in bytes (if known)
    pub total_bytes: u64,
    /// Download speed in megabytes per second
    pub speed_mbps: f64,
    /// Whether this file was served from cache
    pub from_cache: bool,
    /// Current status of the file download
    pub status: FileStatus,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

/// Progress handler trait for receiving download progress updates
pub trait ProgressHandler: Send + Sync {
    /// Handle a progress update for a file download
    fn handle(&self, progress: DownloadProgress);
}

// Blanket implementation for Box<dyn ProgressHandler>
impl<T: ProgressHandler + ?Sized> ProgressHandler for Box<T> {
    fn handle(&self, progress: DownloadProgress) {
        (**self).handle(progress)
    }
}

// Blanket implementation for Arc<dyn ProgressHandler>
impl<T: ProgressHandler + ?Sized> ProgressHandler for std::sync::Arc<T> {
    fn handle(&self, progress: DownloadProgress) {
        (**self).handle(progress)
    }
}

/// Configuration for download operations.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Optional quantization filter (e.g., "Q4_K_M", "Q8_0", "F16")
    /// When specified, only files matching this quantization will be downloaded
    pub quantization: Option<String>,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self { quantization: None }
    }
}
