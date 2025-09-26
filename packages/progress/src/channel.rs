//! Channel type aliases for progress data communication
//!
//! Provides type aliases for flume-based channel communication used throughout
//! the progress tracking system for zero-allocation event passing.

use crate::types::{DownloadProgress, FileStatus};
use std::time::Instant;

/// Extended progress data transmitted through internal channels
///
/// Contains all fields from DownloadProgress plus additional internal tracking
/// fields needed by the channel communication system.
#[derive(Debug, Clone)]
pub struct ProgressData {
    /// Model ID being downloaded
    pub model_id: String,
    /// File path being downloaded
    pub file_path: String,
    /// Number of bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total size of the file in bytes
    pub total_bytes: u64,
    /// Download speed in megabytes per second
    pub speed_mbps: f64,
    /// Whether this file was served from cache
    pub from_cache: bool,
    /// Current status of the file download
    pub status: FileStatus,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// Timestamp when this progress event was created
    pub timestamp: Instant,
    /// Whether this file is cached (alias for from_cache)
    pub is_cached: bool,
    /// Error information (alias for error_message)
    pub error: Option<String>,
}

impl From<DownloadProgress> for ProgressData {
    /// Convert DownloadProgress to ProgressData with additional fields
    fn from(progress: DownloadProgress) -> Self {
        // Extract model_id from path (simplified heuristic)
        let model_id = progress
            .path
            .split('/')
            .find(|part| part.contains('-') && !part.ends_with(".bin") && !part.ends_with(".json"))
            .unwrap_or("unknown")
            .to_string();

        Self {
            model_id,
            file_path: progress.path.clone(),
            bytes_downloaded: progress.bytes_downloaded,
            total_bytes: progress.total_bytes,
            speed_mbps: progress.speed_mbps,
            from_cache: progress.from_cache,
            status: progress.status,
            error_message: progress.error_message.clone(),
            timestamp: Instant::now(),
            is_cached: progress.from_cache,
            error: progress.error_message,
        }
    }
}

impl From<ProgressData> for DownloadProgress {
    /// Convert ProgressData to DownloadProgress (lossy conversion)
    fn from(data: ProgressData) -> Self {
        Self {
            path: data.file_path,
            bytes_downloaded: data.bytes_downloaded,
            total_bytes: data.total_bytes,
            speed_mbps: data.speed_mbps,
            from_cache: data.from_cache,
            status: data.status,
            error_message: data.error_message,
        }
    }
}

/// Sender for progress data events
pub type ProgressSender = flume::Sender<ProgressData>;

/// Receiver for progress data events  
pub type ProgressReceiver = flume::Receiver<ProgressData>;

/// Create a bounded progress channel with specified capacity
///
/// Returns a sender/receiver pair for ProgressData communication.
/// Uses flume for blazing-fast cross-thread communication.
///
/// # Arguments
/// * `capacity` - Maximum number of buffered messages
///
/// # Returns
/// Tuple of (sender, receiver) for ProgressData
///
/// # Performance
/// - Zero-allocation message passing after initialization
/// - Lock-free operations using flume's optimized implementation
/// - Bounded capacity prevents memory exhaustion under backpressure
pub fn progress_channel(capacity: usize) -> (ProgressSender, ProgressReceiver) {
    flume::bounded(capacity)
}

/// Create an unbounded progress channel
///
/// Returns a sender/receiver pair with unlimited capacity.
/// Use with caution as this can lead to memory exhaustion
/// under sustained backpressure.
///
/// # Returns
/// Tuple of (sender, receiver) for ProgressData
pub fn unbounded_progress_channel() -> (ProgressSender, ProgressReceiver) {
    flume::unbounded()
}
