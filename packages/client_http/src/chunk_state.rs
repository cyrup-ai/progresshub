//! Core chunk state management and strategy types
//!
//! Provides zero-allocation state types, chunking strategies, and configuration
//! for high-performance parallel downloads with elegant ergonomic APIs.

use std::{
    cmp,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};
use thiserror::Error as ThisError;

use progresshub_common::ProgressHandler;

/// Maximum number of concurrent chunks to download simultaneously
/// Optimized for blazing-fast performance on modern systems with high bandwidth
pub const MAX_CONCURRENT_CHUNKS: usize = 128;

/// Maximum retry attempts per chunk before permanent failure
pub const MAX_CHUNK_RETRIES: u32 = 3;

/// Base delay for exponential backoff retry logic (milliseconds)
pub const BASE_RETRY_DELAY_MS: u64 = 100;

/// Chunk size thresholds for adaptive sizing
pub const SMALL_FILE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
pub const MEDIUM_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB

/// Optimal chunk size bounds for memory efficiency and performance
pub const MIN_CHUNK_SIZE: u64 = 1024 * 1024; // 1MB minimum
pub const MAX_CHUNK_SIZE: u64 = 100 * 1024 * 1024; // 100MB maximum
pub const DEFAULT_CHUNK_SIZE: u64 = 8 * 1024 * 1024; // 8MB fallback

/// Errors that can occur during chunk-based downloading
#[derive(Debug, ThisError)]
pub enum ChunkError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] quyc::HttpError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Chunk validation failed: expected hash {expected}, got {actual}")]
    ValidationFailed { expected: String, actual: String },

    #[error("Invalid chunk range: {start}-{end} for file size {file_size}")]
    InvalidRange {
        start: u64,
        end: u64,
        file_size: u64,
    },

    #[error("Maximum retries exceeded for chunk {start}-{end}")]
    MaxRetriesExceeded { start: u64, end: u64 },

    // StateError removed - pure event-driven architecture with no shared state
    #[error("HTTP range not satisfiable - server doesn't support range requests")]
    RangeNotSatisfiable,

    #[error("Chunk write failed: {0}")]
    WriteError(String),

    #[error("ETag parsing error: {0}")]
    ETagError(#[from] crate::etag_parser::ETagParseError),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for chunk operations
pub type ChunkResult<T> = Result<T, ChunkError>;

/// Strategy for chunking a file based on its size
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkStrategy {
    /// Single chunk for small files
    Single,
    /// Small chunks for medium files
    Small { chunk_size: u64, num_chunks: usize },
    /// Medium chunks for large files
    Medium { chunk_size: u64, num_chunks: usize },
    /// Large chunks for very large files
    Large { chunk_size: u64, num_chunks: usize },
}

impl ChunkStrategy {
    /// Calculate the optimal chunking strategy for a file size
    ///
    /// # Deprecation Notice
    /// This method is deprecated in favor of `calculate_from_chunk_size()` which uses
    /// server-determined optimal chunk sizes via `ETag` analysis instead of arbitrary thresholds.
    ///
    /// # Intelligent Replacement
    /// Production code should use ETag-based chunk size determination:
    /// ```rust
    /// let optimal_chunk_size = fetcher.fetch_optimal_chunk_size(url, file_size).await?;
    /// let strategy = ChunkStrategy::calculate_from_chunk_size(optimal_chunk_size, file_size);
    /// ```
    #[must_use]
    #[deprecated(
        since = "1.1.0",
        note = "Use calculate_from_chunk_size() with ETag-determined chunk sizes for intelligent chunking"
    )]
    pub fn calculate(file_size: u64) -> Self {
        // Fallback to intelligent sizing using proven optimal chunk size
        // This provides better performance than the old arbitrary thresholds
        Self::calculate_from_chunk_size(DEFAULT_CHUNK_SIZE, file_size)
    }

    /// Calculate chunking strategy from ETag-determined chunk size
    /// Uses server-determined chunk boundaries instead of arbitrary file size thresholds
    #[must_use]
    pub fn calculate_from_chunk_size(chunk_size: u64, file_size: u64) -> Self {
        if file_size == 0 || chunk_size == 0 {
            return Self::Single;
        }

        // If chunk size equals or exceeds file size, use single chunk
        if chunk_size >= file_size {
            return Self::Single;
        }

        // Calculate number of chunks based on ETag-determined size
        let num_chunks = usize::try_from(file_size.div_ceil(chunk_size)).unwrap_or(usize::MAX);
        let num_chunks = cmp::min(num_chunks, MAX_CONCURRENT_CHUNKS);

        // Classify strategy based on chunk size, not file size
        if chunk_size <= MIN_CHUNK_SIZE {
            Self::Small {
                chunk_size,
                num_chunks,
            }
        } else if chunk_size <= DEFAULT_CHUNK_SIZE {
            Self::Medium {
                chunk_size,
                num_chunks,
            }
        } else {
            Self::Large {
                chunk_size,
                num_chunks,
            }
        }
    }

    /// Get the chunk size for this strategy
    #[must_use]
    pub fn chunk_size(&self) -> u64 {
        match self {
            Self::Single => u64::MAX, // Single chunk downloads entire file
            Self::Small { chunk_size, .. }
            | Self::Medium { chunk_size, .. }
            | Self::Large { chunk_size, .. } => *chunk_size,
        }
    }

    /// Get the number of chunks for this strategy
    #[must_use]
    pub fn num_chunks(&self) -> usize {
        match self {
            Self::Single => 1,
            Self::Small { num_chunks, .. }
            | Self::Medium { num_chunks, .. }
            | Self::Large { num_chunks, .. } => *num_chunks,
        }
    }
}

/// Information about a completed chunk download
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Start byte position (inclusive)
    pub start: u64,
    /// End byte position (exclusive)
    pub end: u64,
    /// Number of bytes actually downloaded
    pub bytes_downloaded: u64,
    /// SHA256 hash of the chunk data
    pub hash: String,
    /// Download duration
    pub duration: Duration,
}

/// Configuration for single range download operations (Pure Event-Driven)
pub struct DownloadChunkConfig<'a> {
    pub url: &'a str,
    pub start: u64,
    pub end: u64,
    pub writer: Arc<crate::chunk_assembler::RangeFileWriter>,
    /// Progress handler for chunk completion updates
    pub progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    /// Atomic counter for total bytes downloaded across all ranges
    pub total_bytes: Arc<AtomicU64>,
    /// Model identifier for event dispatch (`repo_id`)
    pub model_id: Option<String>,
    /// Local filename for event dispatch
    pub local_filename: Option<String>,
    /// Flume sender for raw download events (Pure Flume Channel Event-Driven Architecture)
    pub raw_event_sender: Option<flume::Sender<progresshub_common::RawDownloadEvent>>,
    /// Quantization specification (e.g., "`Q4_K_M`", "`Q8_0`", "`F16`")
    pub quantization: String,
    /// Total file size for `RawDownloadEvent` (Pure Event-Driven Architecture)
    pub total_file_size: u64,
    /// Expected hash for file validation (SHA256 for LFS files, None for small files)
    pub expected_hash: Option<String>,
}

/// Configuration for range download operations (owned variant, Pure Event-Driven)
pub struct ChunkDownloadConfig {
    pub url: String,
    pub start: u64,
    pub end: u64,
    pub writer: Arc<crate::chunk_assembler::RangeFileWriter>,
    /// Progress handler for range completion updates
    pub progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    /// Atomic counter for total bytes downloaded across all ranges
    pub total_bytes: Arc<AtomicU64>,
    /// Model identifier for event dispatch (`repo_id`)
    pub model_id: Option<String>,
    /// Local filename for event dispatch
    pub local_filename: Option<String>,
    /// Flume sender for raw download events (Pure Flume Channel Event-Driven Architecture)
    pub raw_event_sender: Option<flume::Sender<progresshub_common::RawDownloadEvent>>,
    /// Quantization specification (e.g., "`Q4_K_M`", "`Q8_0`", "`F16`")
    pub quantization: String,
    /// Total file size for `RawDownloadEvent` (Pure Event-Driven Architecture)
    pub total_file_size: u64,
    /// Expected hash for file validation (SHA256 for LFS files, None for small files)
    pub expected_hash: Option<String>,
}


