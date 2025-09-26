//! Event types for Pure Flume Channel Event-Driven Architecture
//!
//! Defines the core event types that flow through the flume channels:
//! - RawDownloadEvent: Raw progress data from XET/QUIC clients (6 fields including quantization)
//!
//! ProgressCalculator snapshots ARE the events flowing to displays - no custom event types needed.
//! This architecture ensures zero duplicate calculations and clean separation of concerns.

use serde::{Deserialize, Serialize};
use ystream::prelude::*;

/// Simplified file information for progress calculation
///
/// Contains only the essential data needed by CentralProgressDispatcher
/// to calculate accurate per-file progress percentages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFileInfo {
    /// File path relative to repository root
    pub path: String,
    /// File size in bytes (essential for progress calculation)
    pub size: u64,
    /// Optional hash for validation
    pub hash: Option<String>,
    /// Remote URL for downloading
    pub remote_url: String,
}

/// Simplified repository manifest for progress calculation
///
/// Contains per-file size information that CentralProgressDispatcher needs
/// to calculate accurate progress percentages for each individual file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleRepoManifest {
    /// Repository identifier (e.g., "microsoft/DialoGPT-medium")
    pub repo_id: String,
    /// List of files with their sizes for progress calculation
    pub files: Vec<SimpleFileInfo>,
    /// Total size across all files
    pub total_size: u64,
    /// Quantization specification for this manifest
    pub quant: String,
}

impl SimpleRepoManifest {
    /// Create new simple repo manifest
    #[must_use]
    pub fn new(repo_id: String, files: Vec<SimpleFileInfo>, total_size: u64, quant: String) -> Self {
        Self {
            repo_id,
            files,
            total_size,
            quant,
        }
    }

    /// Find file info by path
    pub fn find_file(&self, file_path: &str) -> Option<&SimpleFileInfo> {
        self.files.iter().find(|f| f.path == file_path)
    }

    /// Get expected size for a specific file
    pub fn get_file_size(&self, file_path: &str) -> Option<u64> {
        self.find_file(file_path).map(|f| f.size)
    }
}

/// Manifest initialization event to set total expected download information
///
/// Sent by orchestration before any RawDownloadEvents to establish the total
/// expected files and bytes. This enables accurate progress calculation across
/// all files in the download operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEvent {
    /// Model identifier for this download operation
    pub model_id: String,
    /// Total number of files expected to be downloaded
    pub total_files: usize,
    /// Total bytes expected across all files
    pub total_bytes: u64,
    /// Quantization specification for this download
    pub quant: String,
}

impl ManifestEvent {
    /// Create new manifest event
    #[must_use]
    pub fn new(model_id: String, total_files: usize, total_bytes: u64, quant: String) -> Self {
        Self {
            model_id,
            total_files,
            total_bytes,
            quant,
        }
    }
}

/// Raw download progress event dispatched by XET/QUIC clients
///
/// Contains unformatted progress data that CentralProgressDispatcher will process
/// into ProgressCalculator snapshots. XET/QUIC clients send these via flume channels
/// with NO calculations or formatting - just raw progress numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDownloadEvent {
    /// Model identifier (e.g., "meta-llama/Llama-2-7b")
    pub model_id: String,
    /// Quantization specification (e.g., "Q4_K_M", "Q8_0", "F16")
    /// Enables selective downloading - only requested quantization
    pub quant: String,
    /// Remote URL being downloaded
    pub remote_url: String,
    /// Local filepath where file is being written
    pub local_filepath: String,
    /// Bytes downloaded so far for this file
    pub bytes_downloaded: u64,
    /// Total bytes expected for this file
    pub total_bytes: u64,
    /// Optional finalization event indicating completion level
    pub finalization: Option<FinalizationLevel>,
    /// Expected hash for file validation (SHA256 for LFS files, None for small files)
    pub expected_hash: Option<String>,
    /// Optional error message for ystream MessageChunk compatibility
    pub error_message: Option<String>,
}

/// Finalization event levels for 4-tier completion system
///
/// Enables tracking completion at multiple granularity levels:
/// - Range: Individual chunk/range downloaded
/// - File: Entire file completed and .part moved to final
/// - Model: All files for one model finalized
/// - AllModels: Everything finished
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinalizationLevel {
    /// Range/chunk completion event
    RangeComplete {
        /// Starting byte offset of completed range
        range_start: u64,
        /// Ending byte offset of completed range
        range_end: u64,
    },
    /// File completion event - .part file moved to final
    FileComplete {
        /// Final file path after .part is moved
        final_file_path: String,
    },
    /// Model completion event - all files for model finalized
    ModelComplete {
        /// Model identifier that completed
        model_id: String,
    },
    /// All models completion event - entire download operation finished
    AllModelsComplete,
}

impl RawDownloadEvent {
    /// Create new raw download event
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_id: String,
        quant: String,
        remote_url: String,
        local_filepath: String,
        bytes_downloaded: u64,
        total_bytes: u64,
        finalization: Option<FinalizationLevel>,
        expected_hash: Option<String>,
    ) -> Self {
        Self {
            model_id,
            quant,
            remote_url,
            local_filepath,
            bytes_downloaded,
            total_bytes,
            finalization,
            expected_hash,
            error_message: None,
        }
    }

    /// Get unique file identifier for this event
    #[must_use]
    pub fn file_key(&self) -> String {
        format!("{}:{}", self.model_id, self.local_filepath)
    }
}

/// MessageChunk implementation for RawDownloadEvent to enable ystream compatibility
///
/// This allows RawDownloadEvent to be used in ystream AsyncStream<RawDownloadEvent, CAP>
/// for zero-allocation streaming with crossbeam-based concurrency instead of tokio runtimes.
impl MessageChunk for RawDownloadEvent {
    /// Create error chunk from error message
    fn bad_chunk(error: String) -> Self {
        Self {
            model_id: "ERROR".to_string(),
            quant: "UNKNOWN".to_string(),
            remote_url: "".to_string(),
            local_filepath: "".to_string(),
            bytes_downloaded: 0,
            total_bytes: 0,
            finalization: None,
            expected_hash: None,
            error_message: Some(error),
        }
    }

    /// Return error message if this is an error chunk
    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Check if this chunk represents an error
    fn is_error(&self) -> bool {
        self.error_message.is_some()
    }
}

impl Default for RawDownloadEvent {
    /// Default implementation for ystream compatibility
    fn default() -> Self {
        Self {
            model_id: "".to_string(),
            quant: "".to_string(),
            remote_url: "".to_string(),
            local_filepath: "".to_string(),
            bytes_downloaded: 0,
            total_bytes: 0,
            finalization: None,
            expected_hash: None,
            error_message: None,
        }
    }
}

/// Unified event type for the central dispatcher
///
/// Allows the central dispatcher to receive manifest initialization,
/// detailed repository manifests, and download progress events through a single channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    /// Manifest initialization with total expected information
    Manifest(ManifestEvent),
    /// Detailed repository manifest with per-file sizes for accurate progress calculation
    RepoManifest(SimpleRepoManifest),
    /// Raw download progress from HTTP clients
    Download(RawDownloadEvent),
}

impl From<ManifestEvent> for ProgressEvent {
    fn from(event: ManifestEvent) -> Self {
        ProgressEvent::Manifest(event)
    }
}

impl From<SimpleRepoManifest> for ProgressEvent {
    fn from(manifest: SimpleRepoManifest) -> Self {
        ProgressEvent::RepoManifest(manifest)
    }
}

impl From<RawDownloadEvent> for ProgressEvent {
    fn from(event: RawDownloadEvent) -> Self {
        ProgressEvent::Download(event)
    }
}
