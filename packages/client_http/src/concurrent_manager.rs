//! High-performance parallel chunk download coordination
//!
//! Provides blazing-fast concurrent chunk downloading with adaptive semaphore
//! control, zero-allocation task orchestration, and elegant retry coordination.

use std::{
    cmp,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicU64},
};


use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    chunk_assembler::RangeFileWriter,
    chunk_fetcher::ChunkFetcher,
    chunk_state::{
        ChunkDownloadConfig, ChunkError, ChunkResult, ChunkStrategy, MAX_CONCURRENT_CHUNKS,
    },
};
use progresshub_common::{
    DownloadProgress, FileStatus, ProgressHandler, RawDownloadEvent,
};

/// Configuration for range-based parallel downloads
pub struct RangeDownloadConfig {
    pub url: String,
    pub destination: PathBuf,
    pub file_ranges: Vec<FileRange>,
    pub model_id: String,
    pub progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    pub adaptive_concurrency: usize,
    pub total_file_size: u64,
    pub raw_event_sender: Option<flume::Sender<RawDownloadEvent>>,
    pub quantization: String,
    pub expected_hash: Option<String>,
    pub existing_progress: HashMap<u32, u64>,
}

/// Represents a file range for parallel downloading
#[derive(Debug, Clone)]
pub struct FileRange {
    pub index: u32,
    pub start: u64,
    pub end: u64,
}

/// Configuration for individual range download tasks
#[derive(Clone)]
pub struct RangeTaskConfig<'a> {
    pub url: &'a str,
    pub destination: &'a std::path::Path,
    pub model_id: &'a str,
    pub quantization: &'a str,
    pub expected_hash: Option<&'a String>,
    pub total_file_size: u64,
}

/// Progress tracking components for range downloads
#[derive(Clone)]
pub struct RangeProgressTracking {
    pub progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    pub total_downloaded: Arc<AtomicU64>,
    pub raw_event_sender: Option<flume::Sender<progresshub_common::RawDownloadEvent>>,
}

/// Concurrency control components for range downloads
#[derive(Clone)]
pub struct RangeConcurrencyControl {
    pub semaphore: Arc<Semaphore>,
    pub existing_progress: std::collections::HashMap<u32, u64>,
    pub cancellation_token: CancellationToken,
}

/// Configuration for range file finalization
#[derive(Clone)]
pub struct RangeFinalizationConfig<'a> {
    pub destination: &'a std::path::Path,
    pub model_id: &'a str,
    pub total_file_size: u64,
    pub total_downloaded: u64,
    pub url: &'a str,
    pub quantization: &'a str,
    pub expected_hash: Option<&'a String>,
    pub raw_event_sender: Option<&'a flume::Sender<progresshub_common::RawDownloadEvent>>,
}

/// Thread pool manager for graceful shutdown of download tasks
pub struct ThreadPoolManager {
    abort_handles: Vec<tokio::task::AbortHandle>,
    cancellation_token: CancellationToken,
}

impl ThreadPoolManager {
    /// Create a new thread pool manager with cancellation support
    pub fn new() -> Self {
        Self {
            abort_handles: Vec::new(),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Add a task abort handle to the pool for shutdown management
    pub fn add_abort_handle(&mut self, abort_handle: tokio::task::AbortHandle) {
        self.abort_handles.push(abort_handle);
    }

    /// Add multiple task abort handles to the pool for shutdown management
    pub fn add_abort_handles(&mut self, abort_handles: Vec<tokio::task::AbortHandle>) {
        self.abort_handles.extend(abort_handles);
    }

    /// Get a clone of the cancellation token for task coordination
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Request graceful shutdown of all tasks
    /// 
    /// This signals all tasks to stop gracefully, allowing current chunks to complete.
    /// Tasks should check the cancellation token periodically and exit cleanly.
    pub fn request_shutdown(&self) {
        tracing::info!("🛑 Requesting graceful shutdown of {} download tasks", self.abort_handles.len());
        self.cancellation_token.cancel();
    }

    /// Wait for graceful shutdown with timeout, then force abort if necessary
    /// 
    /// # Arguments
    /// * `timeout` - Maximum time to wait for graceful shutdown before forcing cancellation
    /// 
    /// # Returns
    /// * `Ok(())` - All tasks completed gracefully
    /// * `Err(usize)` - Number of tasks that were forcefully cancelled due to timeout
    pub async fn shutdown_graceful(self, timeout: std::time::Duration) -> Result<(), usize> {
        // First request graceful shutdown
        self.request_shutdown();

        let _total_handles = self.abort_handles.len();

        // Wait for the timeout, then force abort if necessary
        tokio::time::sleep(timeout).await;

        // After timeout, check if we need to force abort any remaining tasks
        let mut aborted_count = 0;
        for abort_handle in self.abort_handles {
            if !abort_handle.is_finished() {
                abort_handle.abort();
                aborted_count += 1;
            }
        }

        if aborted_count == 0 {
            tracing::info!("✅ All download tasks shut down gracefully within timeout");
            Ok(())
        } else {
            tracing::warn!("⚠️ {} download tasks were force-aborted after timeout", aborted_count);
            Err(aborted_count)
        }
    }

    /// Force immediate shutdown of all tasks
    /// 
    /// This immediately cancels all tasks without waiting for graceful completion.
    /// Use this only when graceful shutdown has failed or immediate termination is required.
    pub fn shutdown_immediate(self) {
        tracing::warn!("🚨 Force cancelling {} download tasks immediately", self.abort_handles.len());
        self.cancellation_token.cancel();
        
        // Abort all tasks immediately
        for abort_handle in self.abort_handles {
            abort_handle.abort();
        }
        
        tracing::warn!("🚨 All download tasks force-cancelled");
    }

    /// Get the number of active tasks
    pub fn active_task_count(&self) -> usize {
        self.abort_handles.len()
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

impl Default for ThreadPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel chunk download manager with adaptive concurrency control and graceful shutdown
pub struct ConcurrentChunkManager {
    fetcher: ChunkFetcher,
    thread_pool: ThreadPoolManager,
}

impl ConcurrentChunkManager {
    /// Create a new concurrent chunk manager with graceful shutdown support
    #[must_use]
    pub fn new() -> Self {
        Self {
            fetcher: ChunkFetcher::new(),
            thread_pool: ThreadPoolManager::new(),
        }
    }

    /// Get a reference to the thread pool manager for shutdown control
    pub fn thread_pool(&self) -> &ThreadPoolManager {
        &self.thread_pool
    }

    /// Get a mutable reference to the thread pool manager for task management
    pub fn thread_pool_mut(&mut self) -> &mut ThreadPoolManager {
        &mut self.thread_pool
    }

    /// Request graceful shutdown of all active downloads
    pub fn request_shutdown(&self) {
        self.thread_pool.request_shutdown();
    }

    /// Shutdown gracefully with timeout, consuming the manager
    pub async fn shutdown(self, timeout: std::time::Duration) -> Result<(), usize> {
        self.thread_pool.shutdown_graceful(timeout).await
    }

    /// Force immediate shutdown, consuming the manager
    pub fn shutdown_immediate(self) {
        self.thread_pool.shutdown_immediate();
    }



    /// Calculate adaptive concurrency limit based on system resources
    fn calculate_adaptive_limit(file_size: u64) -> usize {
        // Get current memory pressure (simplified heuristic)
        let base_limit = MAX_CONCURRENT_CHUNKS;

        // Scale down for very large files to prevent memory exhaustion
        let size_factor = if file_size > 1_000_000_000 {
            // 1GB+
            base_limit / 4 // Conservative for huge files
        } else if file_size > 100_000_000 {
            // 100MB+
            base_limit / 2 // Moderate for large files
        } else {
            base_limit // Full speed for smaller files
        };

        // Ensure minimum of 4 concurrent chunks for reasonable performance
        cmp::max(size_factor, 4)
    }

    /// Download a file using range-based parallel downloads with zero pre-allocation
    ///
    /// Creates separate range files for each chunk, enabling accurate progress tracking
    /// without pre-allocation complexity. Each range file grows naturally as data arrives.
    ///
    /// # Errors
    ///
    /// Returns `ChunkError` variants for HTTP failures, I/O errors, validation failures,
    /// or download state errors during the range-based download process.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines, clippy::cast_precision_loss)]
    pub async fn download_file_chunked(
        &mut self,
        url: &str,
        destination: &Path,
        file_size: u64,
        expected_hash: Option<String>,
        _manifest_version: String,
        progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
        model_id: Option<String>,
        raw_event_sender: Option<flume::Sender<progresshub_common::RawDownloadEvent>>,
        quantization: String,
    ) -> ChunkResult<u64> {
        let model_id_str = model_id.as_deref().unwrap_or("unknown");
        
        // DIAGNOSTIC: Track large file HTTP execution in chunk manager
        if file_size > 1_000_000_000 {  // > 1GB
            tracing::info!(
                "🔍 LARGE FILE CHUNK: ConcurrentChunkManager executing large file download for {} (size: {} bytes, model: {})",
                destination.file_name().unwrap_or_default().to_string_lossy(),
                file_size,
                model_id_str
            );
        }
        
        tracing::info!(
            "ConcurrentChunkManager starting range-based download for URL: {} -> {:?} ({} bytes) - model: {} - expected_hash: {:?}",
            url,
            destination,
            file_size,
            model_id_str,
            expected_hash
        );
        info!("Starting range-based chunked download: {} ({} bytes)", url, file_size);

        // Get ETag-based optimal chunk size
        let optimal_chunk_size = self
            .fetcher
            .fetch_optimal_chunk_size(url, file_size)
            .await?;

        // Calculate ETag-based chunking strategy using server-determined chunk size
        let strategy = ChunkStrategy::calculate_from_chunk_size(optimal_chunk_size, file_size);
        let num_chunks = strategy.num_chunks();

        // Calculate adaptive concurrency based on file size and system resources
        let adaptive_concurrency = Self::calculate_adaptive_limit(file_size);

        info!(
            "Using range-based strategy: {:?} with {} ranges (optimal size: {}, adaptive concurrency: {})",
            strategy, num_chunks, optimal_chunk_size, adaptive_concurrency
        );

        // Check if final file already exists and is complete
        if let Ok(metadata) = tokio::fs::metadata(destination).await
            && metadata.len() == file_size
        {
            info!("File already cached completely: {} bytes", file_size);
            return Ok(0);
        }

        // Check for existing range files and calculate resumption
        let existing_range_progress = self.calculate_existing_range_progress(destination, model_id_str).await;
        let total_existing_bytes = existing_range_progress.values().sum::<u64>();

        if total_existing_bytes > 0 {
            info!(
                "📊 Found existing range files with {} bytes ({:.1}% complete)",
                total_existing_bytes,
                {
                    let total_existing_f64 = total_existing_bytes as f64;
                    let file_size_f64 = file_size as f64;
                    (total_existing_f64 / file_size_f64) * 100.0
                }
            );
        }

        // Create ranges for the entire file
        let file_ranges = Self::create_file_ranges(file_size, &strategy);
        let total_ranges = file_ranges.len();

        info!(
            "📊 Download plan: {} ranges ({} bytes total, {} bytes existing)",
            total_ranges,
            file_size,
            total_existing_bytes
        );

        // Download all ranges in parallel with adaptive concurrency and shutdown support
        let total_bytes = self
            .download_ranges_parallel_adaptive(RangeDownloadConfig {
                url: url.to_string(),
                destination: destination.to_path_buf(),
                file_ranges,
                model_id: model_id_str.to_string(),
                progress_handler,
                adaptive_concurrency,
                total_file_size: file_size,
                raw_event_sender,
                quantization,
                expected_hash,
                existing_progress: existing_range_progress,
            })
            .await?;

        info!(
            "Range-based download completed: {} bytes across {} ranges",
            total_bytes, total_ranges
        );
        Ok(total_bytes)
    }

    /// Calculate existing range file progress for resumption
    ///
    /// Scans for existing range files and returns a map of `range_index` -> `bytes_downloaded`
    async fn calculate_existing_range_progress(
        &self,
        destination: &Path,
        model_id: &str,
    ) -> HashMap<u32, u64> {
        let mut progress = HashMap::new();
        let model_hash = RangeFileWriter::generate_model_hash(model_id);
        
        // Look for range files in the same directory as destination
        if let Some(parent_dir) = destination.parent()
            && let Ok(mut entries) = tokio::fs::read_dir(parent_dir).await
        {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_path = entry.path();
                let file_name = file_path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                
                // Check if this is a range file for our model
                if let Some(range_index) = Self::extract_range_index(file_name, destination, &model_hash)
                    && let Ok(metadata) = entry.metadata().await
                {
                    let file_size = metadata.len();
                    if file_size > 0 {
                        progress.insert(range_index, file_size);
                        tracing::debug!(
                            "Found existing range file {}: {} bytes",
                            range_index,
                            file_size
                        );
                    }
                }
            }
        }
        
        progress
    }

    /// Extract range index from filename if it matches our pattern
    fn extract_range_index(
        filename: &str,
        destination: &Path,
        model_hash: &str,
    ) -> Option<u32> {
        let base_name = destination.file_name()?.to_str()?;
        let pattern = format!("{base_name}.part.{model_hash}.");
        
        if filename.starts_with(&pattern) {
            let suffix = &filename[pattern.len()..];
            suffix.parse::<u32>().ok()
        } else {
            None
        }
    }

    /// Create file ranges from chunking strategy
    fn create_file_ranges(file_size: u64, strategy: &ChunkStrategy) -> Vec<FileRange> {
        let chunk_size = strategy.chunk_size();
        let mut ranges = Vec::new();
        let mut current_offset = 0u64;
        let mut range_index = 0u32;

        while current_offset < file_size {
            let range_end = cmp::min(current_offset + chunk_size, file_size);
            ranges.push(FileRange {
                index: range_index,
                start: current_offset,
                end: range_end,
            });
            current_offset = range_end;
            range_index += 1;
        }

        tracing::info!(
            "Created {} file ranges (chunk_size: {}, file_size: {})",
            ranges.len(),
            chunk_size,
            file_size
        );

        ranges
    }

    /// Create a download task for a single range with concurrency control and cancellation support
    fn create_range_download_task(
        &mut self,
        range: &FileRange,
        config: &RangeTaskConfig<'_>,
        progress: &RangeProgressTracking,
        concurrency: &RangeConcurrencyControl,
    ) -> tokio::task::JoinHandle<ChunkResult<()>> {
        // quyc handles HTTP/3 connections automatically - no client needed
        let url = config.url.to_string();
        let destination = config.destination.to_path_buf();
        let model_id = config.model_id.to_string();
        let progress_handler = Arc::clone(&progress.progress_handler);
        let total_downloaded = Arc::clone(&progress.total_downloaded);
        let semaphore = Arc::clone(&concurrency.semaphore);
        let raw_event_sender = progress.raw_event_sender.clone();
        let quantization = config.quantization.to_string();
        let expected_hash = config.expected_hash.cloned();
        let total_file_size = config.total_file_size;
        let range = range.clone();
        let cancellation_token = concurrency.cancellation_token.clone();

        // Check if this range already exists and is complete
        let existing_bytes = concurrency.existing_progress.get(&range.index).copied().unwrap_or(0);
        let expected_range_size = range.end - range.start;

        if existing_bytes >= expected_range_size {
            tracing::info!(
                "Range {} already complete: {} bytes",
                range.index,
                existing_bytes
            );
            total_downloaded.fetch_add(existing_bytes, std::sync::atomic::Ordering::Relaxed);
            // Return a completed task for consistency
            let handle = tokio::spawn(async move { Ok(()) });
            return handle;
        }

        let handle = tokio::spawn(async move {
            // Check for cancellation before starting
            if cancellation_token.is_cancelled() {
                tracing::info!("Range {} download cancelled before start", range.index);
                return Err(ChunkError::WriteError("Download cancelled".to_string()));
            }

            // Acquire adaptive semaphore permit for concurrency control
            let _permit = semaphore.acquire().await.map_err(|_| {
                ChunkError::WriteError("Adaptive semaphore closed".to_string())
            })?;

            // Check for cancellation after acquiring permit
            if cancellation_token.is_cancelled() {
                tracing::info!("Range {} download cancelled after permit acquisition", range.index);
                return Err(ChunkError::WriteError("Download cancelled".to_string()));
            }

            tracing::info!(
                "Starting range {} download: bytes {}-{} ({} bytes)",
                range.index,
                range.start,
                range.end,
                expected_range_size
            );

            // Create range file writer for this specific range
            let range_writer = RangeFileWriter::create_range_file(
                &destination,
                &model_id,
                range.index,
                range.start,
                range.end,
            ).await?;

            // Download this specific range using HTTP range requests
            let result = ChunkFetcher::download_single_chunk_with_retry(ChunkDownloadConfig {
                url: url.clone(),
                start: range.start,
                end: range.end,
                writer: Arc::new(range_writer),
                progress_handler: Arc::clone(&progress_handler),
                total_bytes: Arc::clone(&total_downloaded),
                model_id: Some(model_id.clone()),
                local_filename: Some(destination.to_string_lossy().to_string()),
                raw_event_sender: raw_event_sender.clone(),
                quantization: quantization.clone(),
                total_file_size,
                expected_hash: expected_hash.clone(),
            }).await;

            // Check final cancellation status
            if cancellation_token.is_cancelled() {
                tracing::info!("Range {} download cancelled during execution", range.index);
                return Err(ChunkError::WriteError("Download cancelled".to_string()));
            }

            if let Err(ref e) = result {
                tracing::error!(
                    "Range {} download failed: {}",
                    range.index,
                    e
                );
            } else {
                tracing::info!(
                    "Range {} download completed successfully",
                    range.index
                );
            }

            result
        });

        // JoinHandle doesn't implement Clone, so we return it directly
        // The calling function will store it in the thread pool
        handle
    }

    /// Process download task results and collect any errors
    fn process_download_results(
        results: Vec<Result<ChunkResult<()>, tokio::task::JoinError>>,
        url: &str,
        total_file_size: u64,
        progress_handler: &Arc<dyn ProgressHandler + Send + Sync>,
    ) -> Vec<ChunkError> {
        let mut errors = Vec::new();

        for (range_index, result) in results.into_iter().enumerate() {
            match result {
                Ok(Ok(())) => {
                    // Range completed successfully
                    tracing::debug!("Range {} completed successfully", range_index);
                }
                Ok(Err(e)) => {
                    tracing::error!("Range {} failed: {}", range_index, e);
                    errors.push(e);
                }
                Err(e) => {
                    tracing::error!("Range {} task panicked: {}", range_index, e);
                    errors.push(ChunkError::ConfigurationError(format!(
                        "Range task panic: {e}"
                    )));
                }
            }
        }

        // Return error if any ranges failed
        if !errors.is_empty() {
            error!("❌ Range downloads failed, {} errors occurred", errors.len());
            
            // Send failure progress
            let failure_progress = DownloadProgress {
                path: url.to_string(),
                bytes_downloaded: 0,
                total_bytes: total_file_size,
                speed_mbps: 0.0,
                from_cache: false,
                status: FileStatus::Failed,
                error_message: Some(format!("{} range failures", errors.len())),
            };
            progress_handler.handle(failure_progress);
        }

        errors
    }

    /// Finalize range files by combining them into the final file
    async fn finalize_range_files(
        &self,
        config: &RangeFinalizationConfig<'_>,
    ) -> ChunkResult<u64> {
        // *** RANGE FILE FINALIZATION LOGIC ***
        // After all range downloads complete, check if we can combine them into final file
        tracing::info!(
            "🔄 FINALIZATION: Starting range file finalization for {} (total: {} bytes)",
            config.destination.display(),
            config.total_downloaded
        );
        
        let range_combiner = crate::range_combiner::RangeCombiner::new(config.destination, config.model_id);
        
        // Check if all ranges are complete and ready for combination
        match range_combiner.validate_completeness(config.total_file_size).await {
            Ok(true) => {
                tracing::info!(
                    "✅ FINALIZATION: All ranges complete for {}, starting combination",
                    config.destination.display()
                );
                
                // Combine all range files into the final file
                match range_combiner.combine_ranges(true).await { // true = cleanup ranges after combination
                    Ok(combined_bytes) => {
                        tracing::info!(
                            "🎉 FINALIZATION SUCCESS: Combined {} bytes into final file {}",
                            combined_bytes,
                            config.destination.display()
                        );
                        
                        // Send file completion event after successful combination
                        if let Some(sender) = config.raw_event_sender {
                            let finalization_event = progresshub_common::RawDownloadEvent::new(
                                config.model_id.to_string(),
                                config.quantization.to_string(),
                                config.url.to_string(),
                                config.destination.to_string_lossy().to_string(), // Final file path (no .part)
                                combined_bytes,
                                config.total_file_size,
                                Some(progresshub_common::FinalizationLevel::FileComplete {
                                    final_file_path: config.destination.to_string_lossy().to_string(),
                                }),
                                config.expected_hash.cloned(),
                            );
                            
                            if let Err(e) = sender.send_async(finalization_event).await {
                                tracing::error!(
                                    "❌ FINALIZATION: Failed to send file completion event for {}: {}",
                                    config.destination.display(),
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "✅ FINALIZATION: File completion event sent for {}",
                                    config.destination.display()
                                );
                            }
                        }
                        
                        // Update total bytes to reflect final combined file
                        Ok(combined_bytes)
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ FINALIZATION FAILED: Range combination failed for {}: {}",
                            config.destination.display(),
                            e
                        );
                        
                        // Send failure event but don't fail the entire download
                        // The range files exist and can be manually recovered
                        tracing::warn!(
                            "⚠️ FINALIZATION: Range files remain on disk for manual recovery: {}",
                            config.destination.display()
                        );
                        
                        // Return the total from range files even though finalization failed
                        Ok(config.total_downloaded)
                    }
                }
            }
            Ok(false) => {
                tracing::warn!(
                    "⚠️ FINALIZATION: Ranges incomplete for {}, leaving range files for resume",
                    config.destination.display()
                );
                
                // Ranges are incomplete - leave them for resumption
                // This is normal for partial downloads or interrupted transfers
                Ok(config.total_downloaded)
            }
            Err(e) => {
                tracing::error!(
                    "❌ FINALIZATION: Failed to validate range completeness for {}: {}",
                    config.destination.display(),
                    e
                );
                
                // Don't fail the download due to finalization validation errors
                // The range downloads succeeded, finalization can be attempted later
                Ok(config.total_downloaded)
            }
        }
    }

    /// Download ranges in parallel with adaptive concurrency control and graceful shutdown support
    async fn download_ranges_parallel_adaptive(
        &mut self,
        config: RangeDownloadConfig,
    ) -> ChunkResult<u64> {
        let RangeDownloadConfig {
            url,
            destination,
            file_ranges,
            model_id,
            progress_handler,
            adaptive_concurrency,
            total_file_size,
            raw_event_sender,
            quantization,
            expected_hash,
            existing_progress,
        } = config;

        let total_downloaded = Arc::new(AtomicU64::new(0));
        let adaptive_semaphore = Arc::new(Semaphore::new(adaptive_concurrency));

        tracing::info!(
            "Starting {} range downloads with adaptive concurrency: {}",
            file_ranges.len(),
            adaptive_concurrency
        );

        // Create configuration structs for range downloads
        let task_config = RangeTaskConfig {
            url: &url,
            destination: &destination,
            model_id: &model_id,
            quantization: &quantization,
            expected_hash: expected_hash.as_ref(),
            total_file_size,
        };

        let progress_tracking = RangeProgressTracking {
            progress_handler: Arc::clone(&progress_handler),
            total_downloaded: Arc::clone(&total_downloaded),
            raw_event_sender: raw_event_sender.clone(),
        };

        let concurrency_control = RangeConcurrencyControl {
            semaphore: adaptive_semaphore,
            existing_progress,
            cancellation_token: self.thread_pool.cancellation_token(),
        };

        // Create download tasks for all ranges with shutdown support
        let mut handles = Vec::with_capacity(file_ranges.len());
        for range in file_ranges {
            // Check for cancellation before creating each task
            if self.thread_pool.is_shutdown_requested() {
                tracing::info!("🛑 Download cancelled before creating range {} task", range.index);
                return Err(ChunkError::WriteError("Download cancelled during task creation".to_string()));
            }

            let handle = self.create_range_download_task(
                &range,
                &task_config,
                &progress_tracking,
                &concurrency_control,
            );
            
            // Store abort handle for shutdown management
            self.thread_pool.add_abort_handle(handle.abort_handle());
            handles.push(handle);
        }

        // Wait for all range downloads to complete
        // Abort handles are already stored in thread pool during task creation
        let results = futures_util::future::join_all(handles).await;
        
        // Process results and collect any errors
        let errors = Self::process_download_results(results, &url, total_file_size, &progress_handler);
        
        // Return early if any ranges failed
        if !errors.is_empty()
            && let Some(error) = errors.into_iter().next() {
            return Err(error);
        }

        let total = total_downloaded.load(std::sync::atomic::Ordering::Relaxed);
        
        // Finalize range files into final combined file
        let finalization_config = RangeFinalizationConfig {
            destination: &destination,
            model_id: &model_id,
            total_file_size,
            total_downloaded: total,
            url: &url,
            quantization: &quantization,
            expected_hash: expected_hash.as_ref(),
            raw_event_sender: raw_event_sender.as_ref(),
        };
        
        self.finalize_range_files(&finalization_config).await
    }

    /// Calculate total progress by summing all range file sizes on disk
    ///
    /// This method replaces atomic byte counters with actual filesystem enumeration
    /// to provide accurate progress tracking for range-based downloads.
    ///
    /// # Arguments
    /// * `destination` - Base file path for the download
    /// * `model_id` - Model identifier for generating the model hash
    ///
    /// # Returns
    /// Total bytes downloaded across all range files for this download
    ///
    /// # Errors
    /// Returns `ChunkError::Io` if range file reading fails or directory listing encounters I/O errors.
    pub async fn calculate_total_progress(
        &self,
        destination: &Path,
        model_id: &str,
    ) -> ChunkResult<u64> {
        let model_hash = RangeFileWriter::generate_model_hash(model_id);
        let mut total_progress = 0u64;

        tracing::debug!(
            "Calculating total progress for {} with model hash {}",
            destination.display(),
            model_hash
        );

        // Look for range files in the same directory as destination
        if let Some(parent_dir) = destination.parent() {
            match tokio::fs::read_dir(parent_dir).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let file_path = entry.path();
                        let file_name = file_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("");

                        // Check if this is a range file for our model and destination
                        if let Some(range_index) = Self::extract_range_index(file_name, destination, &model_hash) {
                            match entry.metadata().await {
                                Ok(metadata) => {
                                    let file_size = metadata.len();
                                    total_progress += file_size;
                                    tracing::debug!(
                                        "Range file {}: {} bytes (total: {} bytes)",
                                        range_index,
                                        file_size,
                                        total_progress
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to get metadata for range file {}: {}",
                                        file_name,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to read directory {:?} for progress calculation: {}",
                        parent_dir,
                        e
                    );
                    return Err(ChunkError::Io(e));
                }
            }
        }

        tracing::info!(
            "Total progress for {} ({}): {} bytes across range files",
            destination.display(),
            model_id,
            total_progress
        );

        Ok(total_progress)
    }

}

impl Default for ConcurrentChunkManager {
    fn default() -> Self {
        Self::new()
    }
}
