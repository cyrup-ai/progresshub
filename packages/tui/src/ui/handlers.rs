use std::path::PathBuf;

use tracing::{info, trace};

use progresshub_progress::{DownloadProgress, ProgressHandler};

/// Handler that calls a user-provided function with progress updates
pub struct CallbackHandler<F>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    callback: F,
}

impl<F> CallbackHandler<F>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    /// Create a new callback handler
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> ProgressHandler for CallbackHandler<F>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    fn handle(&self, progress: DownloadProgress) {
        (self.callback)(progress);
    }
}

impl<F> std::fmt::Debug for CallbackHandler<F>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackHandler")
            .field("callback", &"<function>")
            .finish()
    }
}

/// Handler that logs progress to the console through the tracing system
#[derive(Debug, Default)]
pub struct LogHandler {
    verbose: bool,
}

impl LogHandler {
    /// Create a new log handler
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl ProgressHandler for LogHandler {
    fn handle(&self, progress: DownloadProgress) {
        // Use progress crate functions for percentage calculations
        use progresshub_progress::calculator::ProgressPercentage;

        let percentage_struct = ProgressPercentage {
            value: Some(
                (progress.bytes_downloaded as f64).min(progress.total_bytes as f64)
                    / (progress.total_bytes as f64).max(1.0)
                    * 100.0,
            ),
            is_complete: progress.bytes_downloaded >= progress.total_bytes,
            is_indeterminate: progress.total_bytes == 0,
        };
        let percentage = percentage_struct.value_or(0.0);
        let file_name = PathBuf::from(&progress.path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| progress.path.clone());

        if self.verbose {
            info!(
                "{}: {:.1}% ({} / {} bytes, {:.2} MB/s)",
                file_name,
                percentage,
                progress.bytes_downloaded,
                progress.total_bytes,
                progress.speed_mbps
            );
        } else if percentage > 0.0 && (percentage % 10.0) < 1.0 {
            // Log only at 10% increments
            info!("{}: {:.0}%", file_name, percentage);
        }

        trace!("Download progress: {:?}", progress);
    }
}

/// Handler that silently discards progress information
#[derive(Debug, Default)]
pub struct SilentHandler;

impl SilentHandler {
    /// Create a new silent handler
    pub fn new() -> Self {
        Self
    }
}

impl ProgressHandler for SilentHandler {
    fn handle(&self, _progress: DownloadProgress) {
        // Silently discard progress updates
    }
}
