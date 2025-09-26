//! Core immutable data types for progress tracking
//!
//! This module defines immutable data structures using the `im` crate for
//! zero-allocation, structural sharing and thread-safe progress tracking.

/// Raw file progress data: (filename, bytes_downloaded, total_bytes, is_cached)
pub type RawFileProgress = (String, u64, u64, bool);

/// Collection of raw file progress data for a model
pub type RawModelFiles = Vec<RawFileProgress>;

/// Raw model progress data: (model_id, files)
pub type RawModelProgress = (String, RawModelFiles);

/// Collection of raw model progress data
pub type RawProgressData = Vec<RawModelProgress>;

/// Progress percentage result with formatting options
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressPercentage {
    /// Raw percentage value (0.0-100.0), None if indeterminate
    pub value: Option<f64>,
    /// Whether progress is complete (100%)
    pub is_complete: bool,
    /// Whether progress is indeterminate (unknown total)
    pub is_indeterminate: bool,
}

impl ProgressPercentage {
    /// Check if progress is at 0%
    pub fn is_zero(&self) -> bool {
        matches!(self.value, Some(0.0))
    }

    /// Get percentage value or default
    pub fn value_or(&self, default: f64) -> f64 {
        self.value.unwrap_or(default)
    }

    /// Get pre-formatted percentage string
    /// Returns: "76.4%", "100.0%", "---"
    /// This method uses internal progress crate formatting functions
    pub fn formatted(&self) -> String {
        use crate::calculator::format_percentage;
        format_percentage(self)
    }
}

/// Immutable file progress representation
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImmutableFileProgress {
    /// File metadata as immutable map (id, path, etc.)
    pub file_id: im::OrdMap<String, String>,
    /// Display name of the file
    pub file_name: String,
    /// Number of bytes downloaded
    pub bytes_downloaded: u64,
    /// Total bytes for the file
    pub total_bytes: u64,
    /// Whether file was served from cache
    pub is_cached: bool,
    /// Chunk tracking for extensibility
    pub chunks_downloaded: im::Vector<u64>,
}

impl ImmutableFileProgress {
    /// Check if file download is complete
    pub fn is_complete(&self) -> bool {
        self.bytes_downloaded >= self.total_bytes && self.total_bytes > 0
    }

    /// Get pre-formatted percentage string for this file
    /// Returns: "76.4%", "100.0%", "---"
    /// This method uses internal progress crate formatting functions
    pub fn percentage_formatted(&self) -> String {
        use crate::calculator::{calculate_percentage, format_percentage};

        if self.total_bytes == 0 {
            return "---".to_string();
        }

        let percentage = calculate_percentage(self.bytes_downloaded, self.total_bytes);
        format_percentage(&percentage)
    }

    /// Get progress ratio for styling (0.0-1.0)
    /// Returns: 0.0-1.0 for progress, 0.0 if indeterminate
    /// This method uses internal progress crate calculation functions
    pub fn progress_ratio(&self) -> f64 {
        use crate::calculator::get_progress_ratio;

        if self.total_bytes == 0 {
            0.0
        } else {
            get_progress_ratio(self.bytes_downloaded, self.total_bytes)
        }
    }

    /// Get progress percentage struct for this file
    /// Returns: ProgressPercentage with value 0.0-100.0 or None if indeterminate
    /// This method uses internal progress crate calculation functions
    pub fn percentage(&self) -> ProgressPercentage {
        use crate::calculator::calculate_percentage;
        calculate_percentage(self.bytes_downloaded, self.total_bytes)
    }

    /// Get formatted total size: "786MB", "128KB", "4.25GB"
    /// Uses centralized format_bytes function to maintain consistency
    #[inline]
    pub fn total_size_formatted(&self) -> String {
        use crate::calculator::format_bytes;
        format_bytes(self.total_bytes)
    }

    /// Get formatted downloaded size: "456MB", "87KB", "3.2GB"
    /// Uses centralized format_bytes function to maintain consistency
    #[inline]
    pub fn downloaded_formatted(&self) -> String {
        use crate::calculator::format_bytes;
        format_bytes(self.bytes_downloaded)
    }

    /// Get formatted combined display: "456MB / 786MB"
    /// Eliminates need for raw byte access in display components
    #[inline]
    pub fn bytes_formatted(&self) -> String {
        format!(
            "{} / {}",
            self.downloaded_formatted(),
            self.total_size_formatted()
        )
    }
}

impl std::cmp::Ord for ImmutableFileProgress {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Primary sort: incomplete files first (is_complete() == false first)
        let self_complete = self.is_complete();
        let other_complete = other.is_complete();
        
        match self_complete.cmp(&other_complete) {
            std::cmp::Ordering::Equal => {
                // Secondary sort: largest files first (total_bytes descending)
                other.total_bytes.cmp(&self.total_bytes)
            }
            other_order => other_order,
        }
    }
}

impl std::cmp::PartialOrd for ImmutableFileProgress {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Immutable model progress representation
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImmutableModelProgress {
    /// Unique model identifier
    pub model_id: String,
    /// Total bytes downloaded for this model
    pub bytes_downloaded: u64,
    /// Total bytes expected for this model
    pub total_bytes: u64,
    /// Immutable vector of file progress
    pub files: im::Vector<ImmutableFileProgress>,
    /// Whether model is served from cache
    pub is_cached: bool,
    /// Model metadata (manifest data, etc.)
    pub metadata: im::OrdMap<String, String>,
}

impl ImmutableModelProgress {
    /// Check if model download is complete
    pub fn is_complete(&self) -> bool {
        self.bytes_downloaded >= self.total_bytes && self.total_bytes > 0
    }

    /// Get count of completed files
    pub fn completed_files(&self) -> usize {
        self.files.iter().filter(|f| f.is_complete()).count()
    }

    /// Get pre-formatted percentage string for this model
    /// Returns: "76.4%", "100.0%", "---"
    /// This method uses internal progress crate formatting functions
    pub fn percentage_formatted(&self) -> String {
        use crate::calculator::{calculate_percentage, format_percentage};

        if self.total_bytes == 0 {
            return "---".to_string();
        }

        let percentage = calculate_percentage(self.bytes_downloaded, self.total_bytes);
        format_percentage(&percentage)
    }

    /// Get progress ratio for styling (0.0-1.0)
    /// Returns: 0.0-1.0 for progress, 0.0 if indeterminate
    /// This method uses internal progress crate calculation functions
    pub fn progress_ratio(&self) -> f64 {
        use crate::calculator::get_progress_ratio;

        if self.total_bytes == 0 {
            0.0
        } else {
            get_progress_ratio(self.bytes_downloaded, self.total_bytes)
        }
    }

    /// Get progress percentage struct for this model
    /// Returns: ProgressPercentage with value 0.0-100.0 or None if indeterminate
    /// This method uses internal progress crate calculation functions
    pub fn percentage(&self) -> ProgressPercentage {
        use crate::calculator::calculate_percentage;
        calculate_percentage(self.bytes_downloaded, self.total_bytes)
    }

    /// Get formatted total size: "4.25GB", "786MB", "128KB"
    /// Uses centralized format_bytes function to maintain consistency
    #[inline]
    pub fn total_size_formatted(&self) -> String {
        use crate::calculator::format_bytes;
        format_bytes(self.total_bytes)
    }

    /// Get formatted downloaded size: "3.2GB", "456MB", "87KB"  
    /// Uses centralized format_bytes function to maintain consistency
    #[inline]
    pub fn downloaded_formatted(&self) -> String {
        use crate::calculator::format_bytes;
        format_bytes(self.bytes_downloaded)
    }

    /// Get formatted combined display: "3.2GB / 4.25GB"
    /// Eliminates need for raw byte access in display components
    #[inline]
    pub fn bytes_formatted(&self) -> String {
        format!(
            "{} / {}",
            self.downloaded_formatted(),
            self.total_size_formatted()
        )
    }
}

/// Complete immutable progress state representation
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImmutableProgressState {
    /// Immutable vector of model progress
    pub models: im::Vector<ImmutableModelProgress>,
    /// Overall bytes downloaded across all models
    pub overall_bytes_downloaded: u64,
    /// Overall total bytes across all models
    pub overall_total_bytes: u64,
    /// Global manifest data
    pub manifest_data: im::OrdMap<String, String>,
    /// Timestamp of this state snapshot
    pub timestamp: std::time::SystemTime,
}

impl ImmutableProgressState {
    /// Check if all downloads are complete
    pub fn is_complete(&self) -> bool {
        self.overall_bytes_downloaded >= self.overall_total_bytes && self.overall_total_bytes > 0
    }

    /// Get count of completed models
    pub fn completed_models(&self) -> usize {
        self.models.iter().filter(|m| m.is_complete()).count()
    }
}
