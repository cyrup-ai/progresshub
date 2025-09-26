//! Download result types and status enums
//!
//! This module contains all types related to download results, status tracking,
//! and backend information, used by the progress orchestrator.

use std::path::PathBuf;
use std::time::Duration;

/// ZeroOneOrMany type that represents 0, 1, or many results
#[derive(Debug, Clone, Default)]
pub enum ZeroOneOrMany<T> {
    #[default]
    Zero,
    One(T),
    Many(Vec<T>),
}

impl<T> ZeroOneOrMany<T> {
    pub fn is_empty(&self) -> bool {
        matches!(self, ZeroOneOrMany::Zero)
    }

    pub fn len(&self) -> usize {
        match self {
            ZeroOneOrMany::Zero => 0,
            ZeroOneOrMany::One(_) => 1,
            ZeroOneOrMany::Many(items) => items.len(),
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            ZeroOneOrMany::Zero => Vec::new(),
            ZeroOneOrMany::One(item) => vec![item],
            ZeroOneOrMany::Many(items) => items,
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            ZeroOneOrMany::Zero => Box::new(std::iter::empty()),
            ZeroOneOrMany::One(item) => Box::new(std::iter::once(item)),
            ZeroOneOrMany::Many(items) => Box::new(items.iter()),
        }
    }
}

impl<T> IntoIterator for ZeroOneOrMany<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

/// Result of a single file download
#[derive(Debug, Clone)]
pub struct FileResult {
    /// Original filename from the model repository
    pub filename: String,
    /// Path where the file was downloaded (full absolute path)
    pub path: PathBuf,
    /// Actual size of the downloaded file in bytes
    pub downloaded_size: u64,
    /// Expected size from manifest in bytes (for reconciliation)
    pub expected_size: u64,
    /// Hash of the file (if available)
    pub hash: Option<String>,
    /// Whether this file was served from cache vs downloaded fresh
    pub from_cache: bool,
}

/// Download status for each model
#[derive(Debug, Clone)]
pub enum DownloadStatus {
    /// Download completed successfully
    Complete,
    /// Partial download (some files failed)
    Partial {
        completed_files: usize,
        total_files: usize,
    },
    /// Download failed completely
    Failed { error: String },
}

/// Result for a single model download
#[derive(Debug, Clone)]
pub struct ModelResult {
    /// Model identifier (e.g., "meta-llama/Llama-2-7b")
    pub model_id: String,
    /// Status of the download
    pub status: DownloadStatus,
    /// Root path where the model was downloaded (HuggingFace cache directory)
    pub model_cache_path: PathBuf,
    /// Information about individual files within this model
    pub files: Vec<FileResult>,
    /// Total bytes actually downloaded for this model (sum of downloaded_size)
    pub total_downloaded_bytes: u64,
    /// Total bytes expected from manifest (sum of expected_size)
    pub total_expected_bytes: u64,
    /// Number of files downloaded vs expected
    pub files_downloaded: usize,
    /// Number of files expected from manifest
    pub files_expected: usize,
    /// Time taken to download this model
    pub duration: Duration,
    /// Whether all files match expected sizes (reconciliation status)
    pub size_reconciled: bool,
}

/// Comprehensive download result with statistics
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Results for each model (can be zero, one, or many models)
    pub models: ZeroOneOrMany<ModelResult>,
    /// Total bytes actually downloaded across all models
    pub total_downloaded_bytes: u64,
    /// Total bytes expected from all manifests
    pub total_expected_bytes: u64,
    /// Total time taken for all downloads
    pub total_duration: Duration,
    /// Average download speed in MB/s (based on actual downloaded bytes)
    pub average_speed_mbps: f64,
    /// Whether all models and files have reconciled sizes
    pub fully_reconciled: bool,
}
