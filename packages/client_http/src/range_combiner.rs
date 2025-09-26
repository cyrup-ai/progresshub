//! Range File Combiner for Zero-Allocation Streaming Combination
//!
//! Provides high-performance combination of separately downloaded range files
//! into final complete files with atomic operations and comprehensive error handling.

use std::{
    path::{Path, PathBuf},
};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::{debug, error, info, warn};

use crate::chunk_assembler::RangeFileWriter;
use crate::chunk_state::{ChunkError, ChunkResult};

/// Range File Combiner for streaming combination and atomic recombination
pub struct RangeCombiner {
    /// Base file path for the final combined file
    destination: PathBuf,
    /// Model identifier for generating hash and finding range files
    model_id: String,
    /// Model hash for consistent range file identification
    model_hash: String,
}

/// Information about a discovered range file
#[derive(Debug, Clone)]
struct RangeFileInfo {
    /// Range index number
    index: u32,
    /// Full path to the range file
    file_path: PathBuf,
    /// Size of the range file in bytes
    file_size: u64,
}

impl RangeCombiner {
    /// Create a new range combiner for the specified destination and model
    #[must_use]
    pub fn new(destination: &Path, model_id: &str) -> Self {
        let model_hash = RangeFileWriter::generate_model_hash(model_id);
        
        Self {
            destination: destination.to_path_buf(),
            model_id: model_id.to_string(),
            model_hash,
        }
    }

    /// Discover all range files for this model and destination
    ///
    /// Scans the destination directory for range files matching the pattern:
    /// `{filename}.part.{model_hash}.{range_index}`
    ///
    /// # Returns
    /// Vector of `RangeFileInfo` sorted by range index
    async fn discover_range_files(&self) -> ChunkResult<Vec<RangeFileInfo>> {
        let mut range_files = Vec::new();
        
        // Look for range files in the same directory as destination
        if let Some(parent_dir) = self.destination.parent() {
            match tokio::fs::read_dir(parent_dir).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let file_path = entry.path();
                        let file_name = file_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("");

                        // Extract range index if this is a range file for our model
                        if let Some(range_index) = self.extract_range_index(file_name) {
                            match entry.metadata().await {
                                Ok(metadata) => {
                                    let file_size = metadata.len();
                                    
                                    let range_info = RangeFileInfo {
                                        index: range_index,
                                        file_path: file_path.clone(),
                                        file_size,
                                    };
                                    
                                    range_files.push(range_info);
                                    
                                    debug!(
                                        "Discovered range file {}: {} bytes",
                                        range_index,
                                        file_size
                                    );
                                }
                                Err(e) => {
                                    warn!(
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
                    error!(
                        "Failed to read directory {:?} for range discovery: {}",
                        parent_dir,
                        e
                    );
                    return Err(ChunkError::Io(e));
                }
            }
        }

        // Sort range files by index to ensure correct combination order
        range_files.sort_by_key(|info| info.index);

        info!(
            "Discovered {} range files for {} (model: {})",
            range_files.len(),
            self.destination.display(),
            self.model_id
        );

        Ok(range_files)
    }

    /// Extract range index from filename if it matches our pattern
    fn extract_range_index(&self, filename: &str) -> Option<u32> {
        let base_name = self.destination.file_name()?.to_str()?;
        let pattern = format!("{}.part.{}.", base_name, self.model_hash);
        
        if filename.starts_with(&pattern) {
            let suffix = &filename[pattern.len()..];
            suffix.parse::<u32>().ok()
        } else {
            None
        }
    }

    /// Combine all range files into the final destination file with streaming I/O
    ///
    /// This method performs atomic combination by:
    /// 1. Creating a temporary file for the combination
    /// 2. Streaming each range file in order into the temporary file
    /// 3. Atomically moving the temporary file to the final destination
    /// 4. Optionally cleaning up range files after successful combination
    ///
    /// # Arguments
    /// * `cleanup_ranges` - Whether to delete range files after successful combination
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::ConfigurationError` if no range files are found for combination.
    /// Returns `ChunkError::WriteError` for file I/O failures including:
    /// - Failed to create temporary file for atomic combination
    /// - Failed to write range data to output file
    /// - Failed to flush or sync output file to disk
    /// - Failed to atomically move temporary file to final destination
    ///   Returns `ChunkError::Io` for range file reading failures.
    ///   Returns `ChunkError::ValidationFailed` if range file size doesn't match expected size.
    ///
    /// # Returns
    /// Total bytes written to the final file
    pub async fn combine_ranges(&self, cleanup_ranges: bool) -> ChunkResult<u64> {
        info!(
            "Starting range file combination for {} (model: {})",
            self.destination.display(),
            self.model_id
        );

        // Discover all range files
        let range_files = self.discover_range_files().await?;
        
        if range_files.is_empty() {
            return Err(ChunkError::ConfigurationError(
                "No range files found for combination".to_string(),
            ));
        }

        // Create temporary file for atomic combination
        let temp_destination = self.destination.with_extension("tmp");
        let mut output_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_destination)
            .await
            .map_err(|e| {
                ChunkError::WriteError(format!("Failed to create temporary file: {e}"))
            })?;

        let mut total_bytes_written = 0u64;
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer for streaming

        info!(
            "Combining {} range files in order using streaming I/O",
            range_files.len()
        );

        // Stream each range file in order into the output file
        for range_info in &range_files {
            debug!(
                "Combining range file {} ({} bytes)",
                range_info.index,
                range_info.file_size
            );

            // Open range file for reading
            let mut input_file = File::open(&range_info.file_path)
                .await
                .map_err(|e| {
                    ChunkError::Io(e)
                })?;

            // Stream the range file content into the output file
            let mut bytes_copied = 0u64;
            loop {
                match input_file.read(&mut buffer).await {
                    Ok(0) => break, // End of file
                    Ok(bytes_read) => {
                        output_file.write_all(&buffer[..bytes_read]).await.map_err(|e| {
                            ChunkError::WriteError(format!(
                                "Failed to write range {} data: {}",
                                range_info.index, e
                            ))
                        })?;
                        bytes_copied += bytes_read as u64;
                        total_bytes_written += bytes_read as u64;
                    }
                    Err(e) => {
                        return Err(ChunkError::Io(e));
                    }
                }
            }

            // Verify that we copied all expected bytes
            if bytes_copied != range_info.file_size {
                return Err(ChunkError::ValidationFailed {
                    expected: range_info.file_size.to_string(),
                    actual: bytes_copied.to_string(),
                });
            }

            debug!(
                "Successfully combined range {} ({bytes_copied} bytes)",
                range_info.index
            );
        }

        // Ensure all data is written to disk
        output_file.flush().await.map_err(|e| {
            ChunkError::WriteError(format!("Failed to flush output file: {e}"))
        })?;

        output_file.sync_all().await.map_err(|e| {
            ChunkError::WriteError(format!("Failed to sync output file: {e}"))
        })?;

        // Close the output file
        drop(output_file);

        // Atomically move temporary file to final destination
        tokio::fs::rename(&temp_destination, &self.destination)
            .await
            .map_err(|e| {
                ChunkError::WriteError(format!(
                    "Failed to move temporary file to destination: {e}"
                ))
            })?;

        info!(
            "✅ Range combination completed: {total_bytes_written} bytes written to {}",
            self.destination.display()
        );

        // Clean up range files if requested
        if cleanup_ranges {
            self.cleanup_range_files(&range_files).await?;
        }

        Ok(total_bytes_written)
    }

    /// Clean up range files after successful combination
    ///
    /// This method attempts to delete all range files, but continues with warnings
    /// if some deletions fail (non-fatal errors).
    async fn cleanup_range_files(&self, range_files: &[RangeFileInfo]) -> ChunkResult<()> {
        info!(
            "Cleaning up {} range files after successful combination",
            range_files.len()
        );

        let mut cleanup_errors = Vec::new();

        for range_info in range_files {
            match tokio::fs::remove_file(&range_info.file_path).await {
                Ok(()) => {
                    debug!(
                        "Deleted range file {} ({} bytes)",
                        range_info.index,
                        range_info.file_size
                    );
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to delete range file {}: {}",
                        range_info.index, e
                    );
                    warn!("{}", error_msg);
                    cleanup_errors.push(error_msg);
                }
            }
        }

        if cleanup_errors.is_empty() {
            info!("✅ All range files cleaned up successfully");
            Ok(())
        } else {
            // Non-fatal errors - combination succeeded, but cleanup had issues
            warn!(
                "Range combination succeeded, but {} cleanup errors occurred: {:?}",
                cleanup_errors.len(),
                cleanup_errors
            );
            Ok(()) // Still return success since combination worked
        }
    }

    /// Check if all expected ranges are present for combination
    ///
    /// This method verifies that range files form a complete, contiguous sequence
    /// starting from index 0. Gaps in the sequence indicate missing or incomplete downloads.
    ///
    /// # Arguments
    /// * `expected_total_size` - Expected total size of the final combined file
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::Io` for file system errors during range file discovery,
    /// including failures to read the destination directory or get file metadata.
    ///
    /// # Returns
    /// True if all ranges are present and ready for combination
    ///
    /// # Panics
    /// Panics if range index exceeds `u32::MAX`, which should never happen in practice
    /// as range counts are bounded by system memory and file size limits.
    pub async fn validate_completeness(&self, expected_total_size: u64) -> ChunkResult<bool> {
        let range_files = self.discover_range_files().await?;
        
        if range_files.is_empty() {
            return Ok(false);
        }

        // Check for contiguous sequence starting from 0
        let mut total_size = 0u64;
        
        for (expected_index, range_info) in range_files.iter().enumerate() {
            let expected_index = u32::try_from(expected_index).expect("range index should fit in u32");
            if range_info.index != expected_index {
                debug!(
                    "Range sequence gap: expected {expected_index}, found {}",
                    range_info.index
                );
                return Ok(false);
            }
            
            total_size += range_info.file_size;
        }

        // Verify total size matches expectation (with some tolerance for edge cases)
        let size_matches = total_size == expected_total_size;
        
        debug!(
            "Range completeness check: {} ranges, {} bytes total (expected: {}), complete: {}",
            range_files.len(),
            total_size,
            expected_total_size,
            size_matches
        );

        Ok(size_matches)
    }
}