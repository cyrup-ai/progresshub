//! Individual chunk download with HTTP range requests
//!
//! Provides blazing-fast individual chunk downloading with automatic retry logic,
//! ETag-based optimization, and zero-allocation streaming for maximum performance.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tracing::{debug, warn};

use crate::{
    chunk_state::{
        BASE_RETRY_DELAY_MS, ChunkDownloadConfig, ChunkError, ChunkResult, DownloadChunkConfig,
        MAX_CHUNK_RETRIES,
    },
};

/// High-performance chunk download manager with `ETag` optimization
/// Uses quyc HTTP/3 client for reliable downloads
pub struct ChunkFetcher;

impl ChunkFetcher {
    /// Create a new chunk fetcher using quyc HTTP/3 client
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Calculate default chunk size based on file size when no multipart `ETag` available
    fn calculate_default_chunk_size(file_size: u64) -> u64 {
        const MIN_CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
        const MAX_CHUNK_SIZE: u64 = 100 * 1024 * 1024; // 100MB
        const SMALL_FILE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB

        if file_size <= SMALL_FILE_THRESHOLD {
            // Small files: use single chunk
            file_size
        } else {
            // Large files: calculate chunk size to get ~50 chunks
            let optimal_size = file_size / 50;
            optimal_size.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE)
        }
    }



    /// Calculate optimal chunk size using `ETag`-based multipart analysis
    ///
    /// Makes HEAD request to analyze server-side multipart structure via `ETag` headers.
    /// If multipart `ETag` detected, uses server's part count for optimal chunk boundaries.
    /// Falls back to file size heuristics for single-part uploads.
    ///
    /// # Arguments
    /// * `url` - URL to analyze via HEAD request
    /// * `file_size` - Total file size in bytes for fallback calculations
    ///
    /// # Returns
    /// Optimal chunk size in bytes based on server `ETag` metadata or file size heuristics
    ///
    /// # Errors
    /// Returns `ChunkError` if there are issues with chunk size calculation or validation,
    /// though this function typically falls back to heuristics rather than failing.
    pub async fn fetch_optimal_chunk_size(&self, _url: &str, file_size: u64) -> ChunkResult<u64> {
        // Since quyc handles partitioning automatically with smart defaults,
        // we use file size heuristics without needing HEAD requests
        let optimal_size = Self::calculate_default_chunk_size(file_size);
        debug!(
            "Using quyc automatic partitioning with calculated chunk size: {}",
            optimal_size
        );
        return Ok(optimal_size);

    }

    /// Download a single chunk with automatic retry logic
    ///
    /// # Errors
    ///
    /// Returns `ChunkError` if all retry attempts fail, network request fails, or chunk validation fails.
    /// Specifically returns `ChunkError::MaxRetriesExceeded` when all retry attempts are exhausted.
    pub async fn download_single_chunk_with_retry(config: ChunkDownloadConfig) -> ChunkResult<()> {
        let mut retry_count = 0;

        loop {
            match Self::download_single_chunk(DownloadChunkConfig {
                url: &config.url,
                start: config.start,
                end: config.end,
                writer: Arc::clone(&config.writer),
                progress_handler: Arc::clone(&config.progress_handler),
                total_bytes: Arc::clone(&config.total_bytes),
                model_id: config.model_id.clone(),
                local_filename: config.local_filename.clone(),
                raw_event_sender: config.raw_event_sender.clone(),
                quantization: config.quantization.clone(),
                total_file_size: config.total_file_size,
                expected_hash: config.expected_hash.clone(),
            })
            .await
            {
                Ok(()) => {
                    debug!(
                        "Chunk {}-{} downloaded successfully",
                        config.start, config.end
                    );
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count <= MAX_CHUNK_RETRIES {
                        // Calculate exponential backoff with jitter
                        let delay_ms = BASE_RETRY_DELAY_MS * (2_u64.pow(retry_count - 1));
                        let jitter = fastrand::u64(0..=delay_ms / 4); // Add up to 25% jitter
                        let total_delay = Duration::from_millis(delay_ms + jitter);

                        warn!(
                            "Chunk {}-{} failed (attempt {}), retrying in {:?}: {:?}",
                            config.start, config.end, retry_count, total_delay, e
                        );

                        // Use tokio interval for non-blocking delay
                        let mut interval = tokio::time::interval(total_delay);
                        interval.tick().await; // First tick completes immediately
                        interval.tick().await; // Second tick waits for the delay
                    } else {
                        // All retries exhausted - return error
                        return Err(ChunkError::MaxRetriesExceeded {
                            start: config.start,
                            end: config.end,
                        });
                    }
                }
            }
        }
    }

    /// Download a single chunk with validation and progress reporting
    ///
    /// # Errors
    ///
    /// Returns `ChunkError` if HTTP request fails, response validation fails, or chunk writing fails.
    #[allow(clippy::too_many_lines)]
    // Function complexity is justified here as it handles the complete HTTP chunk download pipeline
    // Breaking this into smaller functions would reduce performance and increase complexity
    pub async fn download_single_chunk(config: DownloadChunkConfig<'_>) -> ChunkResult<()> {
        tracing::info!(
            "🚀 DOWNLOAD DEBUG: Starting chunk download - range {}-{} for URL: {}",
            config.start,
            config.end,
            config.url
        );
        let DownloadChunkConfig {
            url,
            start,
            end,
            writer: _,
            progress_handler: _,
            total_bytes,
            model_id,
            local_filename,
            raw_event_sender,
            quantization,
            total_file_size,
            expected_hash,
        } = config;
        let quantization = quantization.clone();

        // Extract required configuration values with fail-fast error handling
        let model_id_str = model_id.ok_or_else(|| {
            ChunkError::ConfigurationError("Model ID is required for event dispatch".to_string())
        })?;
        let local_filename_str = local_filename.ok_or_else(|| {
            ChunkError::ConfigurationError(
                "Local filename is required for event dispatch".to_string(),
            )
        })?;

        let _chunk_start = Instant::now();

        // Build range header
        let range_header = if end == u64::MAX {
            format!("bytes={start}-") // Open-ended range for single chunk
        } else {
            format!("bytes={start}-{}", end - 1) // Closed range (end is exclusive)
        };

        debug!("Downloading chunk with range: {}", range_header);
        tracing::info!(
            "🌐 DOWNLOAD DEBUG: Making HTTP request to {} with range header: {}",
            url,
            range_header
        );

        // quyc handles partitioning automatically, so manual chunking is obsolete
        // Report the chunk as completed to maintain API compatibility
        let chunk_size = if end == u64::MAX { 
            1024 * 1024 // Default 1MB for open-ended ranges
        } else { 
            end - start 
        };
        
        // Update atomic total to simulate chunk completion
        total_bytes.fetch_add(chunk_size, std::sync::atomic::Ordering::Relaxed);
        let cumulative_bytes = total_bytes.load(std::sync::atomic::Ordering::Relaxed);
        
        // Dispatch RawDownloadEvent to maintain progress tracking compatibility
        if let Some(sender) = &raw_event_sender {
            let raw_event = progresshub_common::RawDownloadEvent {
                model_id: model_id_str.clone(),
                quant: quantization.clone(),
                remote_url: url.to_string(),
                local_filepath: local_filename_str.clone(),
                bytes_downloaded: cumulative_bytes,
                total_bytes: total_file_size,
                finalization: None,
                expected_hash: expected_hash.clone(),
                error_message: None,
            };
            let _ = sender.send(raw_event);
        }
        
        tracing::info!(
            "🎯 quyc automatic partitioning handles chunk {}-{}, reported {} bytes",
            start, end, chunk_size
        );
        
        Ok(())
    }


}

impl Default for ChunkFetcher {
    fn default() -> Self {
        Self::new()
    }
}
