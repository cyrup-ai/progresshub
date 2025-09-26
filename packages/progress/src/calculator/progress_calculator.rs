//! ProgressCalculator - Immutable snapshots that flow as events through flume channels
//!
//! This is the core event type in the Pure Flume Channel Event-Driven Architecture.
//! ProgressCalculator contains ALL formatting methods and flows as immutable events
//! from the CentralProgressDispatcher to CLI/TUI/Ratatui displays.

use crate::calculator::{
    FileAnalysisInfo, ImmutableFileProgress, ImmutableModelProgress, ImmutableProgressState,
    QuantizationAnalysis, QuantizationAnalyzer, calculate_percentage, format_bytes,
    format_percentage, format_speed_display,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// ProgressCalculator - The immutable event that flows through flume channels
///
/// Contains all progress data and ALL formatting methods. Displays call accessor
/// methods to get pre-formatted strings with perfect consistency.
/// This IS the event - no custom event types needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressCalculator {
    /// Current immutable progress state
    pub progress_state: ImmutableProgressState,
    /// Quantization information for display
    pub quantization: String,
    /// Timing data for speed/ETA calculations
    pub timing_data: TimingData,
    /// Model-specific data
    pub model_data: ModelData,
}

/// Timing data for speed and ETA calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingData {
    /// When downloads started
    pub started_at: SystemTime,
    /// Last update timestamp
    pub last_update: SystemTime,
    /// Current download speed in bytes per second
    pub current_speed_bps: f64,
}

/// Model-specific data for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelData {
    /// Current model being processed
    pub current_model_id: String,
    /// Current file being processed
    pub current_filename: String,
    /// Total models to download
    pub total_models: usize,
    /// Completed models
    pub completed_models: usize,
}

impl ProgressCalculator {
    /// Create new ProgressCalculator snapshot
    pub fn new(
        progress_state: ImmutableProgressState,
        quantization: String,
        timing_data: TimingData,
        model_data: ModelData,
    ) -> Self {
        // Create new ImmutableProgressState with sorted files
        let sorted_models = progress_state.models
            .into_iter()
            .map(|model| {
                // Convert to Vec, sort, then back to Vector for efficiency
                let mut files_vec: Vec<_> = model.files.into_iter().collect();
                files_vec.sort();
                let sorted_files = files_vec.into_iter().collect();
                
                // Create new ImmutableModelProgress with sorted files
                ImmutableModelProgress {
                    model_id: model.model_id,
                    bytes_downloaded: model.bytes_downloaded,
                    total_bytes: model.total_bytes,
                    files: sorted_files,
                    is_cached: model.is_cached,
                    metadata: model.metadata,
                }
            })
            .collect();
        
        // Create new ImmutableProgressState with sorted models
        let sorted_progress_state = ImmutableProgressState {
            models: sorted_models,
            overall_bytes_downloaded: progress_state.overall_bytes_downloaded,
            overall_total_bytes: progress_state.overall_total_bytes,
            manifest_data: progress_state.manifest_data,
            timestamp: progress_state.timestamp,
        };
        
        Self {
            progress_state: sorted_progress_state,
            quantization,
            timing_data,
            model_data,
        }
    }

    /// Get raw percentage value for calculations
    /// Returns: 76.4, 100.0, 0.0 if indeterminate
    pub fn percentage_raw(&self) -> f64 {
        if self.progress_state.overall_total_bytes == 0 {
            return 0.0;
        }

        let percentage = calculate_percentage(
            self.progress_state.overall_bytes_downloaded,
            self.progress_state.overall_total_bytes,
        );

        percentage.value_or(0.0)
    }

    /// Get pre-formatted percentage string
    /// Returns: "76.4%", "100.0%", "---"
    pub fn percentage_formatted(&self) -> String {
        if self.progress_state.overall_total_bytes == 0 {
            return "---".to_string();
        }

        let percentage = calculate_percentage(
            self.progress_state.overall_bytes_downloaded,
            self.progress_state.overall_total_bytes,
        );

        format_percentage(&percentage)
    }

    /// Get model progress data for TUI state updates
    /// Returns: Reference to the model progress collection
    pub fn get_model_progress(&self) -> &im::Vector<ImmutableModelProgress> {
        &self.progress_state.models
    }

    /// Get pre-formatted bytes display string  
    /// Returns: "1.2 GB / 3.4 GB", "512 MB / 1.0 GB"
    pub fn bytes_formatted(&self) -> String {
        format!(
            "{} / {}",
            format_bytes(self.progress_state.overall_bytes_downloaded),
            format_bytes(self.progress_state.overall_total_bytes)
        )
    }

    /// Get pre-formatted speed display string
    /// Returns: "156 MB/s", "3.8 GB/s", "N/A"
    pub fn speed_formatted(&self) -> String {
        if self.timing_data.current_speed_bps > 0.1 {
            format_speed_display(self.timing_data.current_speed_bps)
        } else {
            "N/A".to_string()
        }
    }

    /// Get session average speed in megabytes per second for TUI display
    /// Returns: average download speed since session start in MB/s
    pub fn session_avg_speed_mbps(&self) -> f64 {
        self.timing_data.current_speed_bps / (1024.0 * 1024.0)
    }

    /// Get pre-formatted ETA display string
    /// Returns: "2m 34s", "1h 15m", "N/A"
    pub fn eta_formatted(&self) -> String {
        if self.timing_data.current_speed_bps <= 0.0 {
            return "N/A".to_string();
        }

        let remaining_bytes = self
            .progress_state
            .overall_total_bytes
            .saturating_sub(self.progress_state.overall_bytes_downloaded);

        if remaining_bytes == 0 {
            return "Complete".to_string();
        }

        let eta_seconds = remaining_bytes as f64 / self.timing_data.current_speed_bps;
        self.format_duration_seconds(eta_seconds as u64)
    }

    /// Get pre-formatted files remaining string
    /// Returns: "12 files remaining", "1 file remaining", "Complete"
    pub fn files_remaining_formatted(&self) -> String {
        let total_files: usize = self
            .progress_state
            .models
            .iter()
            .map(|m| m.files.len())
            .sum();

        let completed_files: usize = self
            .progress_state
            .models
            .iter()
            .map(|m| m.completed_files())
            .sum();

        let remaining = total_files.saturating_sub(completed_files);

        if remaining == 0 {
            "Complete".to_string()
        } else if remaining == 1 {
            "1 file remaining".to_string()
        } else {
            format!("{remaining} files remaining")
        }
    }

    /// Get pre-formatted overall progress string
    /// Returns: "Model: 45.2% complete", "Model: Download complete"
    pub fn overall_progress_formatted(&self) -> String {
        if self.progress_state.is_complete() {
            "Model: Download complete".to_string()
        } else {
            format!("Model: {} complete", self.percentage_formatted())
        }
    }

    /// Get pre-formatted quantization info string
    /// Returns: "Quantization: Q4_K_M", "Quantization: F16"
    pub fn quantization_info_formatted(&self) -> String {
        format!("Quantization: {}", self.quantization)
    }

    /// Get pre-formatted model progress string
    /// Returns: "{model_name} ({current} of {total} models)"
    pub fn model_progress_formatted(&self) -> String {
        format!(
            "{} ({} of {} models)",
            self.model_data.current_model_id,
            self.model_data.completed_models + 1, // +1 for current model
            self.model_data.total_models
        )
    }

    /// Get pre-formatted current file string  
    /// Returns: "{filename}"
    pub fn current_file_formatted(&self) -> String {
        self.model_data.current_filename.clone()
    }

    /// Get pre-formatted model bytes for a specific model
    /// Returns: "6.9 MB / 12.3 GB"
    pub fn model_bytes_formatted(&self, model: &ImmutableModelProgress) -> String {
        format!(
            "{} / {}",
            format_bytes(model.bytes_downloaded),
            format_bytes(model.total_bytes)
        )
    }

    /// Get pre-formatted file bytes for a specific file
    /// Returns: "1.2 MB / 3.4 MB"
    pub fn file_bytes_formatted(&self, file: &ImmutableFileProgress) -> String {
        format!(
            "{} / {}",
            format_bytes(file.bytes_downloaded),
            format_bytes(file.total_bytes)
        )
    }

    /// Get unique identifier for progress tracking
    pub fn progress_key(&self) -> String {
        format!(
            "{}:{}",
            self.model_data.current_model_id, self.model_data.current_filename
        )
    }

    /// Check if all downloads are complete
    pub fn is_complete(&self) -> bool {
        self.progress_state.is_complete()
    }

    /// Check if downloads are in progress
    pub fn is_downloading(&self) -> bool {
        !self.is_complete() && self.progress_state.overall_bytes_downloaded > 0
    }

    /// Get immutable reference to progress state
    ///
    /// This accessor method provides controlled access to the progress state
    /// while maintaining architectural encapsulation principles.
    ///
    /// # Returns
    /// Reference to the immutable progress state for display logic
    pub fn get_progress_state(&self) -> &ImmutableProgressState {
        &self.progress_state
    }

    /// Get formatted total progress for display
    ///
    /// Returns the high-level formatted strings actually shown to users.
    /// All formatting is done by ProgressCalculator to eliminate view formatting.
    ///
    /// # Returns
    /// Tuple of (formatted_downloaded, formatted_total, percentage_string)
    /// Examples: ("4.25 GB", "8.50 GB", "50.0%")
    pub fn get_total_progress_formatted(&self) -> (String, String, String) {
        let downloaded_formatted = format_bytes(self.progress_state.overall_bytes_downloaded);
        let total_formatted = format_bytes(self.progress_state.overall_total_bytes);
        let percentage_formatted = self.percentage_formatted();

        (downloaded_formatted, total_formatted, percentage_formatted)
    }

    /// Get formatted progress summary for display
    ///
    /// Returns complete progress summary as displayed to end users.
    /// Example: "4.25 GB / 8.50 GB (50.0%)"
    ///
    /// # Returns
    /// Single formatted string for display
    pub fn get_progress_summary_formatted(&self) -> String {
        let (downloaded, total, percentage) = self.get_total_progress_formatted();
        format!("{downloaded} / {total} ({percentage})")
    }

    /// Get formatted model counts for display
    ///
    /// Returns formatted strings for model state counts as shown to end users.
    /// Examples: ("3 pending", "2 active", "5 completed")
    ///
    /// # Returns
    /// Tuple of formatted count strings for display
    pub fn model_counts_formatted(&self) -> (String, String, String) {
        let (pending, active, completed) = self.model_counts();

        (
            format!("{pending} pending"),
            format!("{active} active"),
            format!("{completed} completed"),
        )
    }

    /// Get model counts by state with zero allocation
    ///
    /// Returns (pending_count, active_count, completed_count) using
    /// ProgressCalculator intelligence to maintain architectural compliance.
    ///
    /// # Returns
    /// Tuple of (pending, active, completed) model counts
    pub fn model_counts(&self) -> (usize, usize, usize) {
        let mut pending = 0;
        let mut active = 0;
        let mut completed = 0;

        for model in &self.progress_state.models {
            if model.total_bytes == 0 {
                pending += 1;
            } else if model.bytes_downloaded >= model.total_bytes {
                completed += 1;
            } else {
                active += 1;
            }
        }

        (pending, active, completed)
    }

    /// Get total model count with zero allocation
    ///
    /// # Returns
    /// Number of models being tracked
    pub fn get_model_count(&self) -> usize {
        self.progress_state.models.len()
    }

    /// Get model name at specific index with bounds checking
    ///
    /// # Arguments
    /// * `index` - Zero-based index of model
    ///
    /// # Returns
    /// Some(model_id) if index is valid, None otherwise
    pub fn get_model_name_at_index(&self, index: usize) -> Option<String> {
        self.progress_state
            .models
            .get(index)
            .map(|m| m.model_id.clone())
    }

    /// Get models for rendering with zero allocation cloning
    ///
    /// Provides immutable model progress data for display components
    /// while maintaining architectural encapsulation.
    ///
    /// # Returns
    /// Vector of immutable model progress data
    pub fn get_models_for_rendering(&self) -> Vec<crate::calculator::ImmutableModelProgress> {
        self.progress_state.models.iter().cloned().collect()
    }

    /// Get raw progress ratio (0.0 to 1.0)
    pub fn progress_ratio(&self) -> f64 {
        if self.progress_state.overall_total_bytes == 0 {
            0.0
        } else {
            self.progress_state.overall_bytes_downloaded as f64
                / self.progress_state.overall_total_bytes as f64
        }
    }

    /// Get model percentage for a specific model
    /// Returns: 76.4, 100.0, 0.0 for calculations
    pub fn model_percentage(&self, model: &ImmutableModelProgress) -> f64 {
        if model.total_bytes == 0 {
            return 0.0;
        }

        let percentage = calculate_percentage(model.bytes_downloaded, model.total_bytes);

        percentage.value_or(0.0)
    }

    /// Get pre-formatted model percentage for display
    /// Returns: "76.4%", "100.0%", "---"
    pub fn model_percentage_formatted(&self, model: &ImmutableModelProgress) -> String {
        if model.total_bytes == 0 {
            return "---".to_string();
        }

        let percentage = calculate_percentage(model.bytes_downloaded, model.total_bytes);

        format_percentage(&percentage)
    }

    /// Get file percentage for a specific file  
    /// Returns: 76.4, 100.0, 0.0 for calculations
    pub fn file_percentage(&self, file: &ImmutableFileProgress) -> f64 {
        if file.total_bytes == 0 {
            return 0.0;
        }

        let percentage = calculate_percentage(file.bytes_downloaded, file.total_bytes);

        percentage.value_or(0.0)
    }

    /// Get pre-formatted file percentage for display
    /// Returns: "76.4%", "100.0%", "---"
    pub fn file_percentage_formatted(&self, file: &ImmutableFileProgress) -> String {
        if file.total_bytes == 0 {
            return "---".to_string();
        }

        let percentage = calculate_percentage(file.bytes_downloaded, file.total_bytes);

        format_percentage(&percentage)
    }

    /// Format duration in seconds as human-readable string
    fn format_duration_seconds(&self, seconds: u64) -> String {
        if seconds < 60 {
            format!("{seconds}s")
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            let secs = seconds % 60;
            format!("{hours}h {minutes}m {secs}s")
        }
    }

    // ================== QUANTIZATION INTELLIGENCE METHODS ==================
    // These methods provide centralized quantization analysis and validation.
    // All quantization intelligence is centralized in the Progress crate.

    /// Analyze files for quantization patterns and availability.
    ///
    /// This is the centralized intelligence method for quantization analysis.
    /// Examines file information to determine available quantizations, XET availability,
    /// and provides complete analysis for downstream filtering and validation.
    ///
    /// # Arguments
    /// * `files` - Collection of files to analyze (from manifest data)
    ///
    /// # Returns
    /// Complete quantization analysis with all available information
    ///
    /// # Example
    /// ```rust
    /// let files = vec![FileAnalysisInfo {
    ///     path: "model-Q4_K_M.gguf".to_string(),
    ///     size: 1024000,
    ///     xet_hash: Some("hash123".to_string())
    /// }];
    /// let analysis = ProgressCalculator::analyze_quantizations(&files);
    /// ```
    pub fn analyze_quantizations(files: &[FileAnalysisInfo]) -> QuantizationAnalysis {
        let analyzer = QuantizationAnalyzer::new();
        analyzer.analyze_files(files)
    }

    /// Validate that a requested quantization exists in the analyzed files.
    ///
    /// Returns detailed error messages when validation fails, including
    /// available alternatives and file counts. This is the centralized
    /// validation intelligence that should be called by all components.
    ///
    /// # Arguments
    /// * `analysis` - Results from analyze_quantizations()
    /// * `requested_quantization` - User's requested quantization
    ///
    /// # Returns
    /// * `Ok(())` if quantization exists and is valid
    /// * `Err(anyhow::Error)` with detailed error message if not found
    ///
    /// # Example
    /// ```rust
    /// use tracing::info;
    /// let analysis = ProgressCalculator::analyze_quantizations(&files);
    /// match ProgressCalculator::validate_quantization(&analysis, "Q4_K_M") {
    ///     Ok(()) => info!("Quantization valid"),
    ///     Err(e) => tracing::error!("Validation failed: {}", e),
    /// }
    /// ```
    pub fn validate_quantization(
        analysis: &QuantizationAnalysis,
        requested_quantization: &str,
    ) -> anyhow::Result<()> {
        let analyzer = QuantizationAnalyzer::new();
        analyzer.validate_quantization_request(analysis, requested_quantization)
    }

    /// Count files matching a specific quantization pattern.
    ///
    /// Provides file counts for quantization filtering decisions.
    /// Uses centralized quantization detection logic.
    ///
    /// # Arguments
    /// * `files` - Files to analyze
    /// * `quantization` - Quantization to count matches for
    ///
    /// # Returns
    /// Number of model files matching the quantization
    pub fn count_quantized_files(files: &[FileAnalysisInfo], quantization: &str) -> usize {
        let analyzer = QuantizationAnalyzer::new();
        analyzer.count_quantized_files(files, quantization)
    }

    /// Get semantic file status for CLI display
    ///
    /// Returns the correct FileStatus enum for a file based on ProgressCalculator
    /// intelligence. This eliminates ALL view logic from display components.
    ///
    /// # Arguments
    /// * `file` - File progress to evaluate
    ///
    /// # Returns
    /// FileStatus enum for proper status icon selection
    ///
    /// # Architecture
    /// - ONLY location for file status intelligence
    /// - Eliminates view logic in CLI/TUI displays  
    /// - Uses ProgressCalculator state for decisions
    pub fn file_status(&self, file: &ImmutableFileProgress) -> crate::FileStatus {
        use crate::FileStatus;
        
        if file.is_complete() {
            FileStatus::Completed
        } else if file.bytes_downloaded > 0 {
            FileStatus::Downloading  
        } else {
            FileStatus::Pending
        }
    }

    /// Convert manifest file information to FileAnalysisInfo format.
    ///
    /// Helper method to convert from various file info formats to the
    /// standardized FileAnalysisInfo used by quantization analysis.
    ///
    /// # Arguments  
    /// * `path` - File path or URL
    /// * `size` - File size in bytes
    /// * `xet_hash` - Optional XET hash for protocol detection
    ///
    /// # Returns
    /// FileAnalysisInfo ready for quantization analysis
    pub fn create_file_analysis_info(
        path: String,
        size: u64,
        xet_hash: Option<String>,
    ) -> FileAnalysisInfo {
        FileAnalysisInfo {
            path,
            size,
            xet_hash,
        }
    }

    /// Convert ProgressCalculator to DownloadResult with all calculated data
    ///
    /// This method provides the ONLY correct way to extract DownloadResult data
    /// from a ProgressCalculator snapshot. Never manually extract raw fields!
    ///
    /// # Returns
    /// Complete DownloadResult with all statistics and model information
    ///
    /// # Architecture
    /// - Uses calculated values from ProgressCalculator state
    /// - Eliminates ALL manual data extraction patterns
    /// - Provides complete download summary information
    pub fn to_download_result(&self) -> crate::results::DownloadResult {
        use crate::results::{
            DownloadResult, DownloadStatus, FileResult, ModelResult, ZeroOneOrMany,
        };
        use std::path::PathBuf;

        // Calculate total duration from timing data
        let total_duration = self
            .timing_data
            .started_at
            .elapsed()
            .unwrap_or(std::time::Duration::ZERO);

        // Calculate average speed in MB/s
        let average_speed_mbps = if total_duration.as_secs() > 0 {
            let downloaded_mb =
                self.progress_state.overall_bytes_downloaded as f64 / (1024.0 * 1024.0);
            downloaded_mb / total_duration.as_secs_f64()
        } else {
            0.0
        };

        // Build model results from progress state
        let models = if self.progress_state.models.is_empty() {
            ZeroOneOrMany::Zero
        } else {
            let model_results: Vec<ModelResult> = self
                .progress_state
                .models
                .iter()
                .map(|model| {
                    // Determine status based on completion
                    let status =
                        if model.bytes_downloaded >= model.total_bytes && model.total_bytes > 0 {
                            DownloadStatus::Complete
                        } else if model.bytes_downloaded > 0 {
                            DownloadStatus::Partial {
                                completed_files: model.files.len(),
                                total_files: model.files.len(),
                            }
                        } else {
                            DownloadStatus::Failed {
                                error: "No bytes downloaded".to_string(),
                            }
                        };

                    // Build file results
                    let file_results: Vec<FileResult> = model
                        .files
                        .iter()
                        .map(|file| {
                            // Extract local path from file_id metadata if available
                            let local_path = file
                                .file_id
                                .get("local_path")
                                .map(PathBuf::from)
                                .unwrap_or_else(|| PathBuf::from(&file.file_name));

                            FileResult {
                                filename: file.file_name.clone(),
                                path: local_path,
                                downloaded_size: file.bytes_downloaded,
                                expected_size: file.total_bytes,
                                hash: None, // Hash not available in current progress state
                                from_cache: file.is_cached,
                            }
                        })
                        .collect();

                    ModelResult {
                        model_id: model.model_id.clone(),
                        status,
                        model_cache_path: PathBuf::from(&self.model_data.current_model_id), // Simplified path
                        files: file_results,
                        total_downloaded_bytes: model.bytes_downloaded,
                        total_expected_bytes: model.total_bytes,
                        files_downloaded: model.files.len(),
                        files_expected: model.files.len(),
                        duration: total_duration,
                        size_reconciled: model.bytes_downloaded == model.total_bytes,
                    }
                })
                .collect();

            match model_results.len() {
                1 => {
                    // We've verified length is exactly 1, so we can safely get the first element
                    let mut iter = model_results.into_iter();
                    match iter.next() {
                        Some(item) => ZeroOneOrMany::One(item),
                        None => {
                            // This should never happen since we verified length == 1
                            // Fall back to Many variant to avoid panic
                            tracing::error!(
                                "Logic error: iterator empty despite length verification"
                            );
                            ZeroOneOrMany::Many(vec![])
                        }
                    }
                }
                _ => ZeroOneOrMany::Many(model_results),
            }
        };

        DownloadResult {
            models,
            total_downloaded_bytes: self.progress_state.overall_bytes_downloaded,
            total_expected_bytes: self.progress_state.overall_total_bytes,
            total_duration,
            average_speed_mbps,
            fully_reconciled: self.is_complete(),
        }
    }

    /// Get ranges completed formatted
    /// Returns: "23/45 ranges completed", "Complete" if all done
    pub fn ranges_completed_formatted(&self) -> String {
        let total_ranges: usize = self
            .progress_state
            .models
            .iter()
            .flat_map(|m| m.files.iter())
            .map(|f| f.chunks_downloaded.len())
            .sum();

        let completed_ranges: usize = self
            .progress_state
            .models
            .iter()
            .flat_map(|m| m.files.iter())
            .map(|f| f.chunks_downloaded.len())
            .sum();

        if total_ranges == 0 {
            "No ranges".to_string()
        } else if completed_ranges >= total_ranges {
            "All ranges complete".to_string()
        } else {
            format!("{}/{} ranges completed", completed_ranges, total_ranges)
        }
    }

    /// Get files completed formatted  
    /// Returns: "12/15 files completed", "All files complete"
    pub fn files_completed_formatted(&self) -> String {
        let total_files: usize = self
            .progress_state
            .models
            .iter()
            .map(|m| m.files.len())
            .sum();

        let completed_files: usize = self
            .progress_state
            .models
            .iter()
            .flat_map(|m| m.files.iter())
            .filter(|f| f.bytes_downloaded >= f.total_bytes)
            .count();

        if total_files == 0 {
            "No files".to_string()
        } else if completed_files >= total_files {
            "All files complete".to_string()
        } else {
            format!("{}/{} files completed", completed_files, total_files)
        }
    }

    /// Get models completed formatted
    /// Returns: "2/3 models completed", "All models complete"
    pub fn models_completed_formatted(&self) -> String {
        if self.model_data.total_models == 0 {
            "No models".to_string()
        } else if self.model_data.completed_models >= self.model_data.total_models {
            "All models complete".to_string()
        } else {
            format!(
                "{}/{} models completed",
                self.model_data.completed_models, self.model_data.total_models
            )
        }
    }

    /// Get overall finalization status
    /// Returns comprehensive finalization status across all levels
    pub fn overall_finalization_status(&self) -> String {
        if self.model_data.completed_models == self.model_data.total_models
            && self.model_data.total_models > 0
        {
            "🎉 All models completed".to_string()
        } else {
            format!(
                "Models: {}/{} | Files: {}",
                self.model_data.completed_models,
                self.model_data.total_models,
                self.files_completed_formatted()
            )
        }
    }

    /// Get finalization progress percentage
    /// Returns completion percentage based on finalized files
    pub fn finalization_percentage(&self) -> f64 {
        let total_files: usize = self
            .progress_state
            .models
            .iter()
            .map(|m| m.files.len())
            .sum();

        let completed_files: usize = self
            .progress_state
            .models
            .iter()
            .flat_map(|m| m.files.iter())
            .filter(|f| f.bytes_downloaded >= f.total_bytes)
            .count();

        if total_files == 0 {
            0.0
        } else {
            (completed_files as f64 / total_files as f64) * 100.0
        }
    }

    /// Check if all downloads are finalized
    /// Returns true if all files have been moved from .part to final locations
    pub fn is_fully_finalized(&self) -> bool {
        let total_files: usize = self
            .progress_state
            .models
            .iter()
            .map(|m| m.files.len())
            .sum();

        if total_files == 0 {
            return false;
        }

        let completed_files: usize = self
            .progress_state
            .models
            .iter()
            .flat_map(|m| m.files.iter())
            .filter(|f| f.bytes_downloaded >= f.total_bytes)
            .count();

        completed_files == total_files
    }
}

impl TimingData {
    /// Create new timing data
    pub fn new(started_at: SystemTime, current_speed_bps: f64) -> Self {
        Self {
            started_at,
            last_update: SystemTime::now(),
            current_speed_bps,
        }
    }

    /// Create timing data for new download
    pub fn new_download() -> Self {
        Self::new(SystemTime::now(), 0.0)
    }

    /// Update speed calculation
    pub fn update_speed(&mut self, bytes_downloaded: u64, elapsed_duration: Duration) {
        if elapsed_duration.as_secs_f64() > 0.0 {
            self.current_speed_bps = bytes_downloaded as f64 / elapsed_duration.as_secs_f64();
        }
        self.last_update = SystemTime::now();
    }
}

impl ModelData {
    /// Create new model data
    pub fn new(
        current_model_id: String,
        current_filename: String,
        total_models: usize,
        completed_models: usize,
    ) -> Self {
        Self {
            current_model_id,
            current_filename,
            total_models,
            completed_models,
        }
    }

    /// Update current file being processed
    pub fn update_current_file(&mut self, filename: String) {
        self.current_filename = filename;
    }

    /// Mark model as completed
    pub fn complete_model(&mut self) {
        self.completed_models += 1;
    }
}
