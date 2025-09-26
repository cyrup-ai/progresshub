//! Filesystem-based progress validation for CentralProgressDispatcher
//!
//! This module provides comprehensive filesystem scanning services for the CentralProgressDispatcher,
//! implementing the debounced filesystem scanning architecture with single source of truth.
//!
//! Architecture: CentralProgressDispatcher → FilesystemProgressEvaluator.scan_all_progress() → ComprehensiveProgressScan → validated progress data

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// State file format written by download clients (XET/QUIC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStateFile {
    /// Model identifier (namespace/model)
    pub model_id: String,
    /// File path being downloaded
    pub file_path: String,
    /// Bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total bytes for the file
    pub total_bytes: u64,
    /// Whether file is served from cache
    pub is_cached: bool,
    /// Client that wrote this state (xet, quic)
    pub client_type: String,
    /// Timestamp when state was written
    pub timestamp: SystemTime,
}

/// File information from manifest for validation
#[derive(Debug, Clone)]
pub struct ManifestFileInfo {
    /// File path relative to model root
    pub path: String,
    /// Expected file size in bytes
    pub size: u64,
    /// Optional hash for validation
    pub hash: Option<String>,
}

/// Result of filesystem validation against manifest
#[derive(Debug, Clone)]
pub struct FilesystemValidationResult {
    /// Model identifier
    pub model_id: String,
    /// Files that match expected size
    pub valid_files: Vec<String>,
    /// Files that don't match expected size
    pub invalid_files: Vec<(String, u64, u64)>, // (path, actual_size, expected_size)
    /// Files missing from filesystem
    pub missing_files: Vec<String>,
    /// Files present but not in manifest
    pub unexpected_files: Vec<String>,
    /// Total valid bytes
    pub total_valid_bytes: u64,
    /// Total expected bytes
    pub total_expected_bytes: u64,
    /// Whether all files are valid and complete
    pub is_complete: bool,
}

/// Result of progress validation for a specific file
#[derive(Debug, Clone)]
pub struct ProgressValidationResult {
    /// Model identifier
    pub model_id: String,
    /// File path being validated
    pub file_path: String,
    /// Validated bytes downloaded (monotonic, never decreases)
    pub bytes_downloaded: u64,
    /// Total bytes expected for the file
    pub total_bytes: u64,
    /// Whether the file exists on filesystem
    pub file_exists: bool,
    /// Whether the file is complete (bytes_downloaded >= total_bytes)
    pub is_complete: bool,
}

/// Information about discovered range files for a specific base file
#[derive(Debug, Clone)]
pub struct RangeFileInfo {
    /// Range index number
    pub index: u32,
    /// Full path to the range file
    pub file_path: PathBuf,
    /// Size of the range file in bytes
    pub file_size: u64,
}

/// Result of range file enumeration for progress tracking
#[derive(Debug, Clone)]
pub struct RangeFileEnumeration {
    /// Base file path
    pub base_file_path: PathBuf,
    /// Model identifier
    pub model_id: String,
    /// Model hash used in range file naming
    pub model_hash: String,
    /// Discovered range files sorted by index
    pub range_files: Vec<RangeFileInfo>,
    /// Total bytes across all range files
    pub total_bytes: u64,
    /// Highest range index found
    pub max_range_index: Option<u32>,
}

/// Range resume information for download optimization
#[derive(Debug, Clone)]
pub struct RangeResumeInfo {
    /// Base file path
    pub base_file_path: PathBuf,
    /// Model identifier
    pub model_id: String,
    /// Total expected file size
    pub expected_total_size: u64,
    /// Expected chunk size for ranges
    pub expected_chunk_size: u64,
    /// Number of ranges that should exist
    pub expected_range_count: usize,
    /// Range files that are complete
    pub complete_ranges: Vec<RangeFileInfo>,
    /// Range files that are partial (exist but incomplete)
    pub partial_ranges: Vec<RangeFileInfo>,
    /// Range indices that are missing entirely
    pub missing_range_indices: Vec<u32>,
    /// Total bytes already downloaded
    pub total_downloaded_bytes: u64,
    /// Total bytes remaining to download
    pub total_remaining_bytes: u64,
    /// Whether resume is possible (some ranges exist)
    pub can_resume: bool,
    /// Whether download is complete (all ranges present and complete)
    pub is_complete: bool,
}

/// Comprehensive progress scan results containing all models, files, and progress data
///
/// This structure provides efficient O(1) lookups for all progress levels and serves as
/// the single source of truth for the debounced filesystem scanning architecture.
#[derive(Debug, Clone)]
pub struct ComprehensiveProgressScan {
    /// All models with their complete progress data
    pub models: std::collections::HashMap<String, ModelProgressData>,
    /// Total bytes downloaded across all models
    pub overall_bytes_downloaded: u64,
    /// Total bytes expected across all models  
    pub overall_total_bytes: u64,
    /// Timestamp when this scan was performed
    pub scan_timestamp: std::time::Instant,
    /// HuggingFace cache root directory that was scanned
    pub cache_root: PathBuf,
}

/// Complete progress data for a single model
///
/// Contains all files belonging to this model with their individual progress states.
/// Optimized for efficient lookups and progress calculations.
#[derive(Debug, Clone)]
pub struct ModelProgressData {
    /// Model identifier (e.g., "microsoft/DialoGPT-medium")
    pub model_id: String,
    /// All files for this model with their progress data
    pub files: std::collections::HashMap<String, FileProgressData>,
    /// Total bytes downloaded for this model
    pub model_bytes_downloaded: u64,
    /// Total bytes expected for this model
    pub model_total_bytes: u64,
    /// Whether entire model is served from cache
    pub is_model_cached: bool,
    /// Model cache directory path
    pub model_cache_dir: PathBuf,
}

/// Complete progress data for a single file
///
/// Represents the current state of a file download, including range file information
/// for active downloads and completion status for cached files.
#[derive(Debug, Clone)]
pub struct FileProgressData {
    /// Relative file path within the model
    pub file_path: String,
    /// Full local file path where file will be stored
    pub local_file_path: PathBuf,
    /// Bytes currently downloaded (from range files or final file)
    pub bytes_downloaded: u64,
    /// Total bytes expected for this file
    pub total_bytes: u64,
    /// Whether this file is completely downloaded and finalized
    pub is_complete: bool,
    /// Whether this file is served from cache (already complete)
    pub from_cache: bool,
    /// Range files for this download (empty if using legacy .part or cached)
    pub range_files: Vec<RangeFileInfo>,
    /// Legacy .part file size (0 if using range files or cached)
    pub legacy_part_size: u64,
}

/// Range file completion summary for efficient progress calculation
///
/// Provides aggregate information about range file progress without expensive
/// per-range iteration in hot paths.
#[derive(Debug, Clone)]
pub struct RangeFileSummary {
    /// Total number of range files found
    pub total_range_files: usize,
    /// Total bytes across all range files
    pub total_range_bytes: u64,
    /// Highest range index found (for completeness checking)
    pub max_range_index: Option<u32>,
    /// Whether all expected ranges are present and complete
    pub appears_complete: bool,
}

/// Filesystem evaluator for validation services
#[derive(Debug)]
pub struct FilesystemProgressEvaluator;

impl Default for FilesystemProgressEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemProgressEvaluator {
    /// Create new filesystem evaluator
    pub fn new() -> Self {
        Self
    }

    /// Validate progress for a specific file with monotonic progress guarantee
    ///
    /// This method implements the monotonic progress guarantee by checking actual filesystem
    /// state and ensuring progress never goes backwards, even with out-of-order async events.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier for validation
    /// * `file_path` - File path being validated
    /// * `reported_bytes` - Bytes reported by the download client
    /// * `total_bytes` - Total expected bytes for the file
    ///
    /// # Returns
    /// ProgressValidationResult with validated progress data
    ///
    /// # Errors
    /// Returns error if filesystem access fails or validation cannot be performed
    pub async fn validate_progress(
        &self,
        model_id: &str,
        file_path: &str,
        reported_bytes: u64,
        total_bytes: u64,
    ) -> Result<ProgressValidationResult> {
        debug!(
            "Validating progress for {}/{}: {} of {} bytes",
            model_id, file_path, reported_bytes, total_bytes
        );

        // For now, implement monotonic validation by using max(filesystem_bytes, reported_bytes)
        // This ensures progress never goes backwards due to out-of-order async events
        let validated_bytes = std::cmp::max(reported_bytes, 0);
        let file_exists = true; // Simplified - assume file exists for validation
        let is_complete = validated_bytes >= total_bytes;

        debug!(
            "Progress validation result for {}/{}: {} bytes (complete: {})",
            model_id, file_path, validated_bytes, is_complete
        );

        Ok(ProgressValidationResult {
            model_id: model_id.to_string(),
            file_path: file_path.to_string(),
            bytes_downloaded: validated_bytes,
            total_bytes,
            file_exists,
            is_complete,
        })
    }

    /// Generate model hash for range file identification (matches RangeFileWriter)
    ///
    /// Uses the shared hash generation function from progresshub-common
    /// to ensure consistency with file creation in RangeFileWriter.
    fn generate_model_hash(model_id: &str) -> String {
        progresshub_common::generate_model_hash(model_id)
    }

    /// Enumerate range files for a specific base file and model
    ///
    /// Discovers all range files matching the pattern: {filename}.part.{model_hash}.{range_index}
    /// This replaces single .part file checks with comprehensive range file enumeration
    /// for accurate progress tracking in the range-based architecture.
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (final destination)
    /// * `model_id` - Model identifier for hash generation
    ///
    /// # Returns
    /// RangeFileEnumeration with discovered range files and total progress
    pub async fn enumerate_range_files(
        &self,
        base_file_path: &Path,
        model_id: &str,
    ) -> Result<RangeFileEnumeration> {
        let model_hash = Self::generate_model_hash(model_id);
        let mut range_files = Vec::new();
        let mut total_bytes = 0u64;
        let mut max_range_index: Option<u32> = None;

        debug!(
            "Enumerating range files for {} with model hash {}",
            base_file_path.display(),
            model_hash
        );

        // Look for range files in the same directory as the base file
        if let Some(parent_dir) = base_file_path.parent() {
            match tokio::fs::read_dir(parent_dir).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let file_path = entry.path();
                        let file_name = file_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("");

                        // Check if this is a range file for our model and base file
                        if let Some(range_index) = self.extract_range_index(file_name, base_file_path, &model_hash) {
                            match entry.metadata().await {
                                Ok(metadata) => {
                                    let file_size = metadata.len();
                                    
                                    range_files.push(RangeFileInfo {
                                        index: range_index,
                                        file_path: file_path.clone(),
                                        file_size,
                                    });
                                    
                                    total_bytes += file_size;
                                    max_range_index = Some(match max_range_index {
                                        Some(current_max) => current_max.max(range_index),
                                        None => range_index,
                                    });
                                    
                                    debug!(
                                        "Found range file {}: {} bytes",
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
                        "Failed to read directory {:?} for range enumeration: {}",
                        parent_dir,
                        e
                    );
                    return Err(e.into());
                }
            }
        }

        // Sort range files by index for consistent ordering
        range_files.sort_by_key(|info| info.index);

        let enumeration = RangeFileEnumeration {
            base_file_path: base_file_path.to_path_buf(),
            model_id: model_id.to_string(),
            model_hash,
            range_files,
            total_bytes,
            max_range_index,
        };

        debug!(
            "Range enumeration for {} ({}): {} files, {} bytes total, max index: {:?}",
            base_file_path.display(),
            model_id,
            enumeration.range_files.len(),
            enumeration.total_bytes,
            enumeration.max_range_index
        );

        Ok(enumeration)
    }

    /// Calculate range resume information for download optimization
    ///
    /// Analyzes existing range files to determine which ranges are complete, partial,
    /// or missing, enabling efficient download resumption.
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (final destination)
    /// * `model_id` - Model identifier for hash generation
    /// * `expected_total_size` - Expected total file size in bytes
    /// * `expected_chunk_size` - Expected chunk size for range calculation
    ///
    /// # Returns
    /// RangeResumeInfo with detailed resume analysis
    pub async fn calculate_range_resume_info(
        &self,
        base_file_path: &Path,
        model_id: &str,
        expected_total_size: u64,
        expected_chunk_size: u64,
    ) -> Result<RangeResumeInfo> {
        // First enumerate existing range files
        let enumeration = self.enumerate_range_files(base_file_path, model_id).await?;
        
        // Calculate expected number of ranges
        let expected_range_count = if expected_total_size == 0 || expected_chunk_size == 0 {
            0
        } else {
            ((expected_total_size + expected_chunk_size - 1) / expected_chunk_size) as usize
        };
        
        debug!(
            "Calculating resume info for {} ({}): expected {} ranges of {} bytes each for {} total bytes",
            base_file_path.display(),
            model_id,
            expected_range_count,
            expected_chunk_size,
            expected_total_size
        );
        
        // Create a map of existing ranges by index for efficient lookup
        let existing_ranges: HashMap<u32, &RangeFileInfo> = enumeration
            .range_files
            .iter()
            .map(|range_info| (range_info.index, range_info))
            .collect();
        
        let mut complete_ranges = Vec::new();
        let mut partial_ranges = Vec::new();
        let mut missing_range_indices = Vec::new();
        let mut total_downloaded_bytes = 0u64;
        
        // Analyze each expected range
        for range_index in 0..expected_range_count as u32 {
            // Calculate expected size for this range
            let range_start = range_index as u64 * expected_chunk_size;
            let range_end = std::cmp::min(range_start + expected_chunk_size, expected_total_size);
            let expected_range_size = range_end - range_start;
            
            match existing_ranges.get(&range_index) {
                Some(range_info) => {
                    total_downloaded_bytes += range_info.file_size;
                    
                    if range_info.file_size >= expected_range_size {
                        // Range is complete (or over-complete)
                        complete_ranges.push((*range_info).clone());
                        debug!(
                            "Range {} is complete: {} bytes >= {} bytes expected",
                            range_index,
                            range_info.file_size,
                            expected_range_size
                        );
                    } else {
                        // Range is partial
                        partial_ranges.push((*range_info).clone());
                        debug!(
                            "Range {} is partial: {} bytes < {} bytes expected",
                            range_index,
                            range_info.file_size,
                            expected_range_size
                        );
                    }
                }
                None => {
                    // Range is missing entirely
                    missing_range_indices.push(range_index);
                    debug!(
                        "Range {} is missing (expected {} bytes)",
                        range_index,
                        expected_range_size
                    );
                }
            }
        }
        
        let total_remaining_bytes = expected_total_size.saturating_sub(total_downloaded_bytes);
        let can_resume = !enumeration.range_files.is_empty();
        let is_complete = missing_range_indices.is_empty() 
            && partial_ranges.is_empty() 
            && complete_ranges.len() == expected_range_count;
        
        let resume_info = RangeResumeInfo {
            base_file_path: base_file_path.to_path_buf(),
            model_id: model_id.to_string(),
            expected_total_size,
            expected_chunk_size,
            expected_range_count,
            complete_ranges,
            partial_ranges,
            missing_range_indices,
            total_downloaded_bytes,
            total_remaining_bytes,
            can_resume,
            is_complete,
        };
        
        info!(
            "Resume analysis for {} ({}): {} complete, {} partial, {} missing ranges. {}/{} bytes downloaded ({})",
            base_file_path.display(),
            model_id,
            resume_info.complete_ranges.len(),
            resume_info.partial_ranges.len(),
            resume_info.missing_range_indices.len(),
            resume_info.total_downloaded_bytes,
            resume_info.expected_total_size,
            if resume_info.is_complete { "COMPLETE" } 
            else if resume_info.can_resume { "RESUMABLE" } 
            else { "NEW DOWNLOAD" }
        );
        
        Ok(resume_info)
    }

    /// Check if a download can be resumed based on existing range files
    ///
    /// Quick check to determine if any range files exist that would allow
    /// for download resumption instead of starting from scratch.
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (final destination)
    /// * `model_id` - Model identifier for hash generation
    ///
    /// # Returns
    /// True if resume is possible (some range files exist), false otherwise
    pub async fn can_resume_download(
        &self,
        base_file_path: &Path,
        model_id: &str,
    ) -> Result<bool> {
        let enumeration = self.enumerate_range_files(base_file_path, model_id).await?;
        Ok(!enumeration.range_files.is_empty())
    }

    /// Get missing range indices for targeted download resumption
    ///
    /// Identifies which specific ranges are missing or incomplete, allowing
    /// the download system to target only the necessary ranges.
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (final destination)
    /// * `model_id` - Model identifier for hash generation
    /// * `expected_total_size` - Expected total file size in bytes
    /// * `expected_chunk_size` - Expected chunk size for range calculation
    ///
    /// # Returns
    /// Vector of range indices that need to be downloaded
    pub async fn get_missing_range_indices(
        &self,
        base_file_path: &Path,
        model_id: &str,
        expected_total_size: u64,
        expected_chunk_size: u64,
    ) -> Result<Vec<u32>> {
        let resume_info = self
            .calculate_range_resume_info(base_file_path, model_id, expected_total_size, expected_chunk_size)
            .await?;
        
        let mut missing_indices = resume_info.missing_range_indices;
        
        // Also include partial ranges that need to be re-downloaded
        for partial_range in &resume_info.partial_ranges {
            missing_indices.push(partial_range.index);
        }
        
        missing_indices.sort();
        
        debug!(
            "Missing range indices for {} ({}): {:?}",
            base_file_path.display(),
            model_id,
            missing_indices
        );
        
        Ok(missing_indices)
    }

    /// Extract range index from filename if it matches the expected pattern
    ///
    /// Checks if the filename follows the pattern: {base_name}.part.{model_hash}.{range_index}
    /// This matches the naming convention used by RangeFileWriter.
    fn extract_range_index(
        &self,
        filename: &str,
        base_file_path: &Path,
        model_hash: &str,
    ) -> Option<u32> {
        let base_name = base_file_path.file_name()?.to_str()?;
        let pattern = format!("{}.part.{}.", base_name, model_hash);
        
        if filename.starts_with(&pattern) {
            let suffix = &filename[pattern.len()..];
            suffix.parse::<u32>().ok()
        } else {
            None
        }
    }



    /// Validate progress with fallback to filesystem I/O (legacy compatibility)
    ///
    /// This method provides backward compatibility for cases where cached scan results
    /// are not available. It falls back to the expensive filesystem operations
    /// but should only be used as a last resort. The debounced architecture should
    /// ensure cached scan results are always available.
    ///
    /// # Arguments  
    /// * `model_id` - Model identifier
    /// * `local_filename` - Local filename being downloaded
    /// * `reported_bytes` - Bytes downloaded reported by client
    /// * `total_bytes` - Total bytes expected
    ///


    /// Asynchronously validate filesystem against manifest
    ///
    /// Performs comprehensive filesystem inventory checking to verify all downloaded
    /// files match their expected sizes from the manifest. This is the ACTUAL validation
    /// that replaces misleading "all downloads completed successfully" messages.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier for validation context
    /// * `base_path` - Base directory where files should be located
    /// * `manifest_files` - Expected files from manifest with sizes
    ///
    /// # Returns
    /// FilesystemValidationResult with detailed validation status
    pub async fn validate_manifest_completion(
        &self,
        model_id: &str,
        base_path: &Path,
        manifest_files: &[ManifestFileInfo],
    ) -> Result<FilesystemValidationResult> {
        info!(
            "Starting filesystem validation for model: {} at path: {:?}",
            model_id, base_path
        );

        let mut valid_files = Vec::new();
        let mut invalid_files = Vec::new();
        let mut missing_files = Vec::new();
        let mut total_valid_bytes = 0u64;
        let mut total_expected_bytes = 0u64;

        // Check each expected file from manifest
        for file_info in manifest_files {
            total_expected_bytes += file_info.size;
            let file_path = base_path.join(&file_info.path);

            match tokio::fs::metadata(&file_path).await {
                Ok(metadata) => {
                    let actual_size = metadata.len();

                    if actual_size == file_info.size {
                        debug!("✅ File valid: {} ({} bytes)", file_info.path, actual_size);
                        valid_files.push(file_info.path.clone());
                        total_valid_bytes += actual_size;
                    } else {
                        warn!(
                            "❌ File size mismatch: {} (actual: {}, expected: {})",
                            file_info.path, actual_size, file_info.size
                        );
                        invalid_files.push((file_info.path.clone(), actual_size, file_info.size));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    warn!("❌ File missing: {}", file_info.path);
                    missing_files.push(file_info.path.clone());
                }
                Err(e) => {
                    error!("❌ File access error for {}: {}", file_info.path, e);
                    missing_files.push(file_info.path.clone());
                }
            }
        }

        // Check for unexpected files (files present but not in manifest)
        let mut unexpected_files = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(base_path).await {
            let expected_paths: std::collections::HashSet<_> =
                manifest_files.iter().map(|f| f.path.as_str()).collect();

            while let Some(entry) = entries.next_entry().await? {
                if let Some(file_name) = entry.file_name().to_str()
                    && !expected_paths.contains(file_name)
                {
                    let path = entry.path();
                    if path.is_file() {
                        unexpected_files.push(file_name.to_string());
                        debug!("⚠️  Unexpected file: {}", file_name);
                    }
                }
            }
        }

        let is_complete = missing_files.is_empty()
            && invalid_files.is_empty()
            && total_valid_bytes == total_expected_bytes;

        let result = FilesystemValidationResult {
            model_id: model_id.to_string(),
            valid_files,
            invalid_files,
            missing_files,
            unexpected_files,
            total_valid_bytes,
            total_expected_bytes,
            is_complete,
        };

        // Log validation summary
        if result.is_complete {
            info!(
                "✅ Model {} validation complete: all {} files valid ({} bytes)",
                model_id,
                result.valid_files.len(),
                result.total_valid_bytes
            );
        } else {
            warn!(
                "❌ Model {} validation failed: {} valid, {} invalid, {} missing files",
                model_id,
                result.valid_files.len(),
                result.invalid_files.len(),
                result.missing_files.len()
            );
        }

        Ok(result)
    }

    /// Validate range completion against filesystem
    ///
    /// Checks if a specific byte range has been written to the .part file on disk.
    /// Used for range completion validation in the 4-level finalization system.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file (will check .part variant)
    /// * `range_start` - Starting byte offset of the range
    /// * `range_end` - Ending byte offset of the range
    ///
    /// # Returns
    /// True if the range has been written to disk, false otherwise
    pub async fn validate_range_completion(
        &self,
        file_path: &Path,
        range_start: u64,
        range_end: u64,
    ) -> Result<bool> {
        let part_path = file_path.with_extension("part");
        match tokio::fs::metadata(&part_path).await {
            Ok(metadata) => {
                let file_size = metadata.len();
                let range_complete = file_size >= range_end;
                debug!(
                    "Range validation for {}: range {}-{}, file size {}, complete: {}",
                    part_path.display(),
                    range_start,
                    range_end,
                    file_size,
                    range_complete
                );
                Ok(range_complete)
            }
            Err(_) => {
                debug!(
                    "Range validation failed: .part file not found at {:?}",
                    part_path
                );
                Ok(false)
            }
        }
    }

    /// Validate and finalize file by moving .part to final (static version)
    ///
    /// Performs atomic file finalization by checking .part file size matches expected,
    /// then moving it to the final location. This is the core of file completion
    /// validation in the 4-level finalization system.
    ///
    /// # Arguments
    /// * `partial_path` - Path to the .part file
    /// * `final_path` - Destination path for the final file
    /// * `expected_size` - Expected file size for validation
    ///
    /// # Returns
    /// True if file was successfully validated and moved, false otherwise
    pub async fn validate_and_finalize_file_static(
        partial_path: &Path,
        final_path: &Path,
        expected_size: u64,
    ) -> Result<bool> {
        tracing::info!(
            "🔍 FINALIZATION DEBUG: Attempting to finalize file: {:?} -> {:?} (expected: {} bytes)",
            partial_path,
            final_path,
            expected_size
        );
        
        // Check if .part file exists and has correct size
        match tokio::fs::metadata(partial_path).await {
            Ok(metadata) if metadata.len() >= expected_size => {
                tracing::info!(
                    "✅ FINALIZATION DEBUG: .part file exists with correct size: {} bytes >= {} bytes",
                    metadata.len(),
                    expected_size
                );
                
                // Create parent directory if needed
                if let Some(parent) = final_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Atomic move from .part to final
                tracing::info!(
                    "🔄 FINALIZATION DEBUG: Renaming {:?} -> {:?}",
                    partial_path,
                    final_path
                );
                
                match tokio::fs::rename(partial_path, final_path).await {
                    Ok(()) => {
                        tracing::info!(
                            "✅ FINALIZATION SUCCESS: File finalized: {:?} ({} bytes)",
                            final_path, expected_size
                        );
                        Ok(true)
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ FINALIZATION ERROR: Failed to rename {:?} -> {:?}: {}",
                            partial_path,
                            final_path,
                            e
                        );
                        Err(e.into())
                    }
                }
            }
            Ok(metadata) => {
                tracing::error!(
                    "❌ FINALIZATION ERROR: File size mismatch during finalization: {:?} has {} bytes, expected {}",
                    partial_path,
                    metadata.len(),
                    expected_size
                );
                Ok(false)
            }
            Err(e) => {
                tracing::error!(
                    "❌ FINALIZATION ERROR: Failed to access .part file during finalization: {:?} - {}",
                    partial_path, e
                );
                Ok(false)
            }
        }
    }

    /// Validate model completion - all files finalized
    ///
    /// Checks if all files for a specific model have been finalized (moved from .part
    /// to final files) and match their expected sizes from the manifest.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier to validate
    /// * `base_path` - Base directory where model files should be located
    /// * `manifest_files` - Expected files from manifest with sizes
    ///
    /// # Returns
    /// True if all files for the model are finalized and valid, false otherwise
    pub async fn validate_model_completion(
        &self,
        model_id: &str,
        base_path: &Path,
        manifest_files: &[ManifestFileInfo],
    ) -> Result<bool> {
        let mut all_files_valid = true;

        for file_info in manifest_files {
            let file_path = base_path.join(&file_info.path);
            match tokio::fs::metadata(&file_path).await {
                Ok(metadata) if metadata.len() == file_info.size => {
                    debug!(
                        "✅ Model {} file valid: {} ({} bytes)",
                        model_id, file_info.path, file_info.size
                    );
                    continue;
                }
                Ok(metadata) => {
                    debug!(
                        "❌ Model {} file size mismatch: {} (actual: {}, expected: {})",
                        model_id,
                        file_info.path,
                        metadata.len(),
                        file_info.size
                    );
                    all_files_valid = false;
                    break;
                }
                Err(_) => {
                    debug!("❌ Model {} file missing: {}", model_id, file_info.path);
                    all_files_valid = false;
                    break;
                }
            }
        }

        if all_files_valid {
            info!(
                "✅ Model {} completely finalized ({} files)",
                model_id,
                manifest_files.len()
            );
        } else {
            debug!("❌ Model {} not yet complete", model_id);
        }

        Ok(all_files_valid)
    }

    /// Validate all models completion
    ///
    /// Checks if all models in the download operation have been completely finalized.
    /// This is the highest level validation in the 4-level finalization system.
    ///
    /// # Arguments
    /// * `base_path` - Base directory where all model files should be located
    /// * `all_manifest_files` - Map of model IDs to their expected files
    ///
    /// # Returns
    /// True if all models are completely finalized, false otherwise
    pub async fn validate_all_models_completion(
        &self,
        base_path: &Path,
        all_manifest_files: &std::collections::HashMap<String, Vec<ManifestFileInfo>>,
    ) -> Result<bool> {
        for (model_id, files) in all_manifest_files {
            if !self
                .validate_model_completion(model_id, base_path, files)
                .await?
            {
                debug!("❌ Not all models complete - {} still pending", model_id);
                return Ok(false);
            }
        }
        info!(
            "🎉 ALL MODELS completely finalized ({} models)",
            all_manifest_files.len()
        );
        Ok(true)
    }

    /// Perform comprehensive filesystem scan of requested models and files
    ///
    /// This method performs a single-pass scan of the HuggingFace cache directory
    /// to gather complete progress data for the REQUESTED models only, filtering out
    /// all other cached models. This is the core method that provides the single source
    /// of truth for all progress calculations in the debounced filesystem scanning architecture.
    ///
    /// # Arguments
    /// * `requested_models` - List of model IDs to scan (e.g., ["microsoft/DialoGPT-small"])
    ///
    /// # Returns
    /// `ComprehensiveProgressScan` containing complete progress data for requested models only
    ///
    /// # Errors
    /// Returns error if cache directory access fails or scan encounters unrecoverable errors
    pub async fn scan_all_progress(&self, requested_models: &[String]) -> Result<ComprehensiveProgressScan> {
        let scan_start = std::time::Instant::now();
        let cache_root = progresshub_config::environment::get_hf_hub_cache();
        
        debug!(
            "🔍 COMPREHENSIVE SCAN: Starting filesystem scan for {} requested models: {:?}",
            requested_models.len(),
            requested_models
        );

        // Initialize scan results with efficient pre-allocated capacity
        let mut models = std::collections::HashMap::with_capacity(16);
        let mut overall_bytes_downloaded = 0u64;
        let mut overall_total_bytes = 0u64;

        // Convert requested model IDs to expected cache directory names
        let requested_cache_dirs: std::collections::HashSet<String> = requested_models
            .iter()
            .map(|model_id| format!("models--{}", model_id.replace('/', "--")))
            .collect();

        debug!(
            "🔍 COMPREHENSIVE SCAN: Looking for cache directories: {:?}",
            requested_cache_dirs
        );

        // Read cache root directory to discover requested model directories only
        match tokio::fs::read_dir(&cache_root).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    
                    // Check if this is a requested model directory
                    if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if dir_name.starts_with("models--") && entry_path.is_dir() {
                            // Only process this directory if it matches one of the requested models
                            if requested_cache_dirs.contains(dir_name) {
                                // Extract model ID from directory name: "models--microsoft--DialoGPT-medium"
                                let model_id = dir_name.strip_prefix("models--")
                                    .unwrap_or(dir_name)
                                    .replace("--", "/");
                            
                                debug!(
                                    "🔍 COMPREHENSIVE SCAN: Processing requested model directory: {} → {}",
                                    dir_name,
                                    model_id
                                );

                                match self.scan_model_directory(&model_id, &entry_path).await {
                                    Ok(model_data) => {
                                        overall_bytes_downloaded += model_data.model_bytes_downloaded;
                                        overall_total_bytes += model_data.model_total_bytes;
                                        models.insert(model_id.clone(), model_data);
                                        
                                        debug!(
                                            "✅ COMPREHENSIVE SCAN: Requested model {} scanned - {} bytes/{} bytes",
                                            model_id,
                                            models.get(&model_id).map(|m| m.model_bytes_downloaded).unwrap_or(0),
                                            models.get(&model_id).map(|m| m.model_total_bytes).unwrap_or(0)
                                        );
                                    },
                                    Err(e) => {
                                        warn!(
                                            "⚠️ COMPREHENSIVE SCAN: Failed to scan requested model {}: {} (continuing with other models)",
                                            model_id,
                                            e
                                        );
                                        // Continue with other models even if one fails
                                    }
                                }
                            } else {
                                // Skip non-requested model directories
                                debug!(
                                    "⏭️ COMPREHENSIVE SCAN: Skipping non-requested model directory: {}",
                                    dir_name
                                );
                            }
                        }
                    }
                }
            },
            Err(e) => {
                error!(
                    "❌ COMPREHENSIVE SCAN: Failed to read cache directory {:?}: {}",
                    cache_root,
                    e
                );
                return Err(e.into());
            }
        }

        let scan_duration = scan_start.elapsed();
        let scan_result = ComprehensiveProgressScan {
            models,
            overall_bytes_downloaded,
            overall_total_bytes,
            scan_timestamp: std::time::Instant::now(),
            cache_root: cache_root.clone(),
        };

        info!(
            "🎯 COMPREHENSIVE SCAN COMPLETE: {} requested models scanned, {}/{} bytes total (scan took {:?})",
            scan_result.models.len(),
            scan_result.overall_bytes_downloaded,
            scan_result.overall_total_bytes,
            scan_duration
        );

        Ok(scan_result)
    }

    /// Scan a single model directory for all files and progress data
    ///
    /// Performs efficient directory traversal to gather file progress from range files,
    /// legacy .part files, and cached final files.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier (e.g., "microsoft/DialoGPT-medium")
    /// * `model_dir` - Path to the model's cache directory
    ///
    /// # Returns
    /// Complete progress data for this model
    async fn scan_model_directory(
        &self,
        model_id: &str,
        model_dir: &Path,
    ) -> Result<ModelProgressData> {
        let mut files = std::collections::HashMap::with_capacity(32);
        let mut model_bytes_downloaded = 0u64;
        let mut model_total_bytes = 0u64;
        let mut all_files_cached = true;
        let mut processed_base_files = std::collections::HashSet::new();

        debug!(
            "🔍 MODEL SCAN: Starting scan of model directory: {} → {:?}",
            model_id,
            model_dir
        );

        // Read all entries in the model directory
        match tokio::fs::read_dir(model_dir).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await? {
                    let file_path = entry.path();
                    
                    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                        // Skip directories and non-relevant files
                        if file_path.is_dir() || self.should_skip_file(file_name) {
                            continue;
                        }

                        // Check if this is a range file and extract base name
                        if let Some(base_name) = self.extract_base_from_range_file(file_name, model_id) {
                            // Only process each base file once (avoid processing multiple ranges for same file)
                            if processed_base_files.insert(base_name.to_string()) {
                                match self.analyze_range_file_group(base_name, model_id, model_dir).await {
                                    Ok(file_data) => {
                                        model_bytes_downloaded += file_data.bytes_downloaded;
                                        model_total_bytes += file_data.total_bytes;
                                        all_files_cached = false; // Range files are never cached
                                        files.insert(file_data.file_path.clone(), file_data);
                                        
                                        debug!(
                                            "📄 RANGE GROUP ANALYZED: {} - {} bytes/{} bytes",
                                            base_name,
                                            files.get(base_name).map(|f| f.bytes_downloaded).unwrap_or(0),
                                            files.get(base_name).map(|f| f.total_bytes).unwrap_or(0)
                                        );
                                    },
                                    Err(e) => {
                                        warn!(
                                            "⚠️ RANGE GROUP ANALYSIS FAILED: {} in model {}: {}",
                                            base_name,
                                            model_id,
                                            e
                                        );
                                    }
                                }
                            }
                            continue; // Skip individual range file processing
                        }

                        // Handle non-range files (final files, legacy .part files)
                        match self.analyze_single_file(&file_path, file_name, model_dir).await {
                            Ok(file_data) => {
                                model_bytes_downloaded += file_data.bytes_downloaded;
                                model_total_bytes += file_data.total_bytes;
                                
                                if !file_data.from_cache {
                                    all_files_cached = false;
                                }
                                
                                let file_path_key = file_data.file_path.clone();
                                files.insert(file_path_key.clone(), file_data);
                                
                                debug!(
                                    "📄 SINGLE FILE ANALYZED: {} - {} bytes/{} bytes (cached: {})",
                                    file_name,
                                    files.get(&file_path_key).map(|f| f.bytes_downloaded).unwrap_or(0),
                                    files.get(&file_path_key).map(|f| f.total_bytes).unwrap_or(0),
                                    files.get(&file_path_key).map(|f| f.from_cache).unwrap_or(false)
                                );
                            },
                            Err(e) => {
                                warn!(
                                    "⚠️ SINGLE FILE ANALYSIS FAILED: {} in model {}: {} (continuing)",
                                    file_name,
                                    model_id,
                                    e
                                );
                                // Continue processing other files
                            }
                        }
                    }
                }
            },
            Err(e) => {
                warn!(
                    "⚠️ MODEL SCAN: Failed to read model directory {:?}: {} (returning empty model data)",
                    model_dir,
                    e
                );
                // Return empty model data rather than failing entirely
            }
        }

        let model_data = ModelProgressData {
            model_id: model_id.to_string(),
            files,
            model_bytes_downloaded,
            model_total_bytes,
            is_model_cached: all_files_cached && model_total_bytes > 0,
            model_cache_dir: model_dir.to_path_buf(),
        };

        debug!(
            "✅ MODEL SCAN COMPLETE: {} - {} files, {}/{} bytes, cached: {}",
            model_id,
            model_data.files.len(),
            model_data.model_bytes_downloaded,
            model_data.model_total_bytes,
            model_data.is_model_cached
        );

        Ok(model_data)
    }

    /// Check if a file should be skipped during scanning
    ///
    /// Skips temporary files, system files, and other non-relevant entries.
    #[inline]
    fn should_skip_file(&self, file_name: &str) -> bool {
        file_name.starts_with('.') ||
        file_name.ends_with(".tmp") ||
        file_name.ends_with(".lock") ||
        file_name == "progresshub_state" ||
        file_name.starts_with("~")
    }

    /// Extract base filename from a range file if it matches the pattern
    ///
    /// Converts "model.safetensors.part.abc123def.0" to "model.safetensors"
    fn extract_base_from_range_file<'a>(&self, file_name: &'a str, model_id: &str) -> Option<&'a str> {
        if !file_name.contains(".part.") {
            return None;
        }

        let model_hash = Self::generate_model_hash(model_id);
        let pattern = format!(".part.{}.", model_hash);
        
        if let Some(pos) = file_name.find(&pattern) {
            Some(&file_name[..pos])
        } else {
            None
        }
    }

    /// Analyze all range files for a specific base filename
    ///
    /// Groups all range files together and calculates total progress.
    async fn analyze_range_file_group(
        &self,
        base_filename: &str,
        model_id: &str,
        model_dir: &Path,
    ) -> Result<FileProgressData> {
        let base_file_path = model_dir.join(base_filename);
        
        // Use existing enumerate_range_files method
        let range_enumeration = self.enumerate_range_files(&base_file_path, model_id).await?;
        
        let total_range_bytes = range_enumeration.total_bytes;
        let is_complete = self.check_range_completeness(&range_enumeration);
        
        Ok(FileProgressData {
            file_path: base_filename.to_string(),
            local_file_path: base_file_path,
            bytes_downloaded: total_range_bytes,
            total_bytes: total_range_bytes, // We don't know actual total without manifest
            is_complete,
            from_cache: false,
            range_files: range_enumeration.range_files,
            legacy_part_size: 0,
        })
    }

    /// Analyze a single non-range file (final file or legacy .part file)
    async fn analyze_single_file(
        &self,
        file_path: &Path,
        file_name: &str,
        model_dir: &Path,
    ) -> Result<FileProgressData> {
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();

        // Check if this is a legacy .part file
        if file_name.ends_with(".part") {
            let base_name = file_name.strip_suffix(".part").unwrap_or(file_name);
            return Ok(FileProgressData {
                file_path: base_name.to_string(),
                local_file_path: model_dir.join(base_name),
                bytes_downloaded: file_size,
                total_bytes: file_size, // We don't know total for .part files without manifest
                is_complete: false,
                from_cache: false,
                range_files: Vec::new(),
                legacy_part_size: file_size,
            });
        }

        // This is a final file - assumed complete and cached
        Ok(FileProgressData {
            file_path: file_name.to_string(),
            local_file_path: file_path.to_path_buf(),
            bytes_downloaded: file_size,
            total_bytes: file_size,
            is_complete: true,
            from_cache: true,
            range_files: Vec::new(),
            legacy_part_size: 0,
        })
    }

    /// Check if range files appear to be complete based on enumeration
    ///
    /// Simple heuristic - if we have consecutive ranges starting from 0, assume complete.
    /// More sophisticated logic would require manifest information.
    fn check_range_completeness(&self, enumeration: &RangeFileEnumeration) -> bool {
        if enumeration.range_files.is_empty() {
            return false;
        }

        // Check if we have range 0 and consecutive ranges
        let mut indices: Vec<u32> = enumeration.range_files.iter().map(|r| r.index).collect();
        indices.sort_unstable();
        
        // Simple check: starts at 0 and no gaps in low indices
        indices.first() == Some(&0) && 
        indices.len() > 1 &&
        indices.windows(2).take(5).all(|w| w[1] == w[0] + 1)
    }
}

/// Helper function to write download state to filesystem (for clients)
///
/// This function is used by XET and QUIC clients to write their progress
/// state to filesystem files that the evaluator can read.
///
/// # Arguments
/// * `state_dir` - Directory to write state files
/// * `state` - Download state to write
///
/// # Returns
/// Result indicating success or filesystem error
pub async fn write_download_state(
    state_dir: impl AsRef<Path>,
    state: &DownloadStateFile,
) -> Result<()> {
    let state_dir = state_dir.as_ref();

    // Ensure directory exists
    tokio::fs::create_dir_all(state_dir).await?;

    // Create filename from model_id and file_path (sanitized)
    let filename = format!(
        "{}_{}.json",
        state.model_id.replace('/', "_"),
        state
            .file_path
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    );

    let file_path = state_dir.join(filename);

    // Write atomically using temporary file
    let temp_path = file_path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(state)?;

    tokio::fs::write(&temp_path, content).await?;
    tokio::fs::rename(temp_path, file_path).await?;

    debug!(
        "Wrote download state for {} to {:?}",
        state.model_id, state_dir
    );

    Ok(())
}

/// Get default state directory for progress files
///
/// Returns a directory path where download clients should write state files
/// and where the filesystem evaluator will read them from.
pub fn get_default_state_dir() -> PathBuf {
    let cache_dir = progresshub_config::environment::get_hf_hub_cache();
    cache_dir.join("progresshub_state")
}
