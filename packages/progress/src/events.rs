//! Event types for Pure Flume Channel Event-Driven Architecture
//!
//! Defines the core event types that flow through the flume channels:
//! - RawDownloadEvent: Raw progress data from XET/QUIC clients (6 fields including quantization)
//!
//! ProgressCalculator snapshots ARE the events flowing to displays - no custom event types needed.
//! This architecture ensures zero duplicate calculations and clean separation of concerns.

use serde::{Deserialize, Serialize};

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
}

impl RawDownloadEvent {
    /// Create new raw download event
    #[must_use]
    pub fn new(
        model_id: String,
        quant: String,
        remote_url: String,
        local_filepath: String,
        bytes_downloaded: u64,
        total_bytes: u64,
    ) -> Self {
        Self {
            model_id,
            quant,
            remote_url,
            local_filepath,
            bytes_downloaded,
            total_bytes,
        }
    }

    /// Get unique file identifier for this event
    #[must_use]
    pub fn file_key(&self) -> String {
        format!("{}:{}", self.model_id, self.local_filepath)
    }
}
