//! No-operation progress handler for testing and scenarios where progress tracking is disabled.
//!
//! Provides a blazing-fast no-op implementation that discards all progress events
//! with zero allocation and zero overhead.

use crate::types::{DownloadProgress, ProgressHandler};

/// A progress handler that does nothing.
///
/// This handler discards all progress events and is useful for:
/// - Testing scenarios where progress tracking should be disabled
/// - Performance benchmarks where progress overhead should be eliminated
/// - Scenarios where progress data is not needed
///
/// # Performance
/// - Zero allocation - no heap allocations or data structures
/// - Blazing-fast execution - single function call with immediate return
/// - No locking - completely lock-free implementation
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpProgressHandler;

impl ProgressHandler for NoOpProgressHandler {
    /// Handle a progress event by discarding it.
    ///
    /// This method does nothing and returns immediately, providing
    /// the fastest possible progress handler implementation.
    ///
    /// # Arguments
    /// * `_progress` - Progress event to discard (unused)
    #[inline]
    fn handle(&self, _progress: DownloadProgress) {
        // Do nothing - fastest possible implementation
    }
}
