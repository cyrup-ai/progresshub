//! Zero-allocation debounced progress tracking for XET client integration
//!
//! Provides AsyncWrite trait wrapper that sends progress updates through existing ProgressHandler
//! interface with immediate first update (speed lane) and debounced subsequent updates.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use tokio::io::{self, AsyncWrite};

use crate::types::{DownloadProgress, FileStatus, ProgressHandler};

/// AsyncWrite wrapper that tracks progress and sends updates to a ProgressHandler
///
/// Integrates with existing XET client architecture by wrapping async File writes
/// and calling progress_handler.handle() with proper DownloadProgress events.
pub struct DebouncedProgressTrackingWriter<W: AsyncWrite + Unpin> {
    inner: W,
    progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    file_path: String,
    total_bytes: u64,
    bytes_written: u64,
    last_update_time: Option<Instant>,
    update_interval_ms: u64,
}

impl<W: AsyncWrite + Unpin> DebouncedProgressTrackingWriter<W> {
    /// Create a new progress tracking writer
    ///
    /// # Arguments
    /// - `writer`: The underlying writer to wrap
    /// - `progress_handler`: Handler to receive progress events
    /// - `file_path`: Path of the file being written
    /// - `total_bytes`: Total expected bytes (0 if unknown)
    #[inline]
    pub fn new(
        writer: W,
        progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
        file_path: String,
        total_bytes: u64,
    ) -> Self {
        Self {
            inner: writer,
            progress_handler,
            file_path,
            total_bytes,
            bytes_written: 0,
            last_update_time: None,
            update_interval_ms: 50, // 50ms debounce interval
        }
    }

    /// Send progress update with debouncing logic
    #[inline]
    fn send_progress_update(&mut self, force_send: bool) {
        let now = Instant::now();
        let should_send = force_send
            || self.last_update_time.is_none_or(|last| {
                now.duration_since(last).as_millis() >= self.update_interval_ms as u128
            });

        if should_send {
            let progress = DownloadProgress {
                path: self.file_path.clone(),
                bytes_downloaded: self.bytes_written,
                total_bytes: self.total_bytes,
                speed_mbps: 0.0, // Will be calculated by UI layer
                from_cache: false,
                status: if self.bytes_written >= self.total_bytes && self.total_bytes > 0 {
                    FileStatus::Completed
                } else {
                    FileStatus::Downloading
                },
                error_message: None,
            };

            self.progress_handler.handle(progress);
            self.last_update_time = Some(now);
        }
    }

    /// Get the current bytes written
    #[inline]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Get the total expected bytes
    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Set the total expected bytes
    #[inline]
    pub fn set_total_bytes(&mut self, total_bytes: u64) {
        self.total_bytes = total_bytes;
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for DebouncedProgressTrackingWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let bytes_written = match Pin::new(&mut self.inner).poll_write(cx, buf) {
            Poll::Ready(result) => result?,
            Poll::Pending => return Poll::Pending,
        };

        self.bytes_written += bytes_written as u64;

        // Send initial progress immediately (speed lane)
        let is_first_write = self.last_update_time.is_none();
        self.send_progress_update(is_first_write);

        Poll::Ready(Ok(bytes_written))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        if result.is_ready() {
            // Send final progress on flush
            self.send_progress_update(true);
        }
        result
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let result = Pin::new(&mut self.inner).poll_shutdown(cx);
        if result.is_ready() {
            // Send final progress on shutdown
            self.send_progress_update(true);
        }
        result
    }
}

impl<W: AsyncWrite + Unpin> Drop for DebouncedProgressTrackingWriter<W> {
    fn drop(&mut self) {
        // Send final completion progress on drop
        if self.bytes_written > 0 {
            let completion_progress = DownloadProgress {
                path: self.file_path.clone(),
                bytes_downloaded: self.bytes_written,
                total_bytes: self.bytes_written.max(self.total_bytes),
                speed_mbps: 0.0,
                from_cache: false,
                status: FileStatus::Completed,
                error_message: None,
            };
            self.progress_handler.handle(completion_progress);
        }
    }
}

/// Create a progress tracking writer for XET client integration
///
/// # Arguments
/// - `writer`: The underlying async writer to wrap
/// - `progress_handler`: Handler to receive progress events
/// - `file_path`: Path of the file being written
/// - `total_bytes`: Total expected bytes (0 if unknown)
///
/// # Returns
/// - Configured `DebouncedProgressTrackingWriter`
#[inline]
pub fn create_progress_tracking_writer<W: AsyncWrite + Unpin>(
    writer: W,
    progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    file_path: String,
    total_bytes: u64,
) -> DebouncedProgressTrackingWriter<W> {
    DebouncedProgressTrackingWriter::new(writer, progress_handler, file_path, total_bytes)
}

impl<W: AsyncWrite + Unpin> std::fmt::Debug for DebouncedProgressTrackingWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebouncedProgressTrackingWriter")
            .field("file_path", &self.file_path)
            .field("bytes_written", &self.bytes_written)
            .field("total_bytes", &self.total_bytes)
            .field("update_interval_ms", &self.update_interval_ms)
            .finish()
    }
}
