//! A high-performance, production-ready HTTP client for `ProgressHub`.
//! This client uses modern HTTP protocols with automatic version negotiation for all communications.
//! It is designed for reliability, efficiency, and ease of use, with built-in chunked downloads,
//! retry logic, progress reporting, and support for request timeouts and cancellation.
//!
//! # HTTP Protocol Implementation
//!
//! This client uses modern HTTP protocols with automatic negotiation - HTTP/3, HTTP/2, or HTTP/1.1
//! as supported by the server. Optimized for chunked parallel downloads with range requests.
//!
//! # Features
//!
//! - **HTTP/3 Protocol**: Exclusive use of HTTP/3 over QUIC transport with prior knowledge
//! - **Connection Pooling**: Efficiently manage multiple HTTP/3 connections
//! - **Automatic Retries**: Configurable exponential backoff for transient failures
//! - **Progress Reporting**: Built-in support for download progress tracking
//! - **Request Timeouts**: Configurable timeouts for connection and requests
//! - **Cancellation Support**: Gracefully cancel in-progress operations
//! - **Zero-Copy**: Efficient data handling with optimized allocations
//!
//! # Example
//!
//! ```no_run
//! use progresshub_client_http::{HttpClient, DownloadFileConfig};
//! use std::sync::Arc;
//! use std::time::Duration;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a new client with default settings
//!     let client = HttpClient::default()
//!         .with_request_timeout(Duration::from_secs(30))
//!         .with_connect_timeout(Duration::from_secs(10));
//!
//!     // Download a file with resumable support to HF cache directory
//!     let progress_handler = Arc::new(progresshub_common::NoOpProgressHandler);
//!     
//!     // Destination is computed based on HF cache environment variables
//!     let cache_dir = progresshub_config::environment::get_hf_hub_cache();
//!     let file_destination = cache_dir.join("models--example--model").join("file.bin");
//!     
//!     let config = DownloadFileConfig {
//!         path: "username/model-repo/pytorch_model.bin",
//!         destination: &file_destination,
//!         expected_file_size: 1024,
//!         expected_hash: None,
//!         manifest_version: "v1".to_string(),
//!         expected_chunk_size: 8192,
//!         progress_handler,
//!         model_id: Some("username/model-repo".to_string()),
//!         raw_event_sender: None,
//!         quantization: "F16".to_string(),
//!     };
//!     let bytes_downloaded = client.download_file_resumable(config).await?;
//!
//!     Ok(())
//! }
//! ```
//! High-performance HTTP client implementation for progresshub
//!
//! This package provides a focused HTTP/3 QUIC client implementation for `HuggingFace` downloads
//! to maintain proper separation of concerns. It handles HTTP protocol communication
//! with `HuggingFace` and other endpoints, providing superior performance
//! and reliability compared to traditional HTTP/1.1 implementations.

#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

// chunk_manager removed - use concurrent_manager directly
pub mod chunk_assembler;
pub mod chunk_fetcher;
pub mod chunk_state;
pub mod client;
pub mod concurrent_manager;
pub mod connection_manager;
pub mod error;
pub mod etag_parser;
pub mod range_combiner;
pub mod types;

pub use chunk_assembler::ChunkValidator;
pub use chunk_state::{ChunkError, ChunkResult, ChunkStrategy, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE};
pub use concurrent_manager::ConcurrentChunkManager;
pub use range_combiner::RangeCombiner;

// Backward compatibility alias
pub type ChunkManager = ConcurrentChunkManager;
pub use client::{DownloadFileConfig, HttpClient};
// Backward compatibility alias
pub type QuicClient = HttpClient;
pub use connection_manager::{ConnectionManager, ConnectionPoolConfig, ServerResolver};
pub use error::QuicError;
pub use etag_parser::{ETagParseError, ETagParseResult, ETagParser, MultipartInfo};
pub use types::ConnectionInfo;
