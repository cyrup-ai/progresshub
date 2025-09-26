//! HTTP client modules
//!
//! Decomposed HTTP client implementation with focused, zero-allocation modules:
//! - `client_core`: Core client struct, error types, authentication, and builder patterns  
//! - `url_builder`: Zero-allocation URL construction with stack-allocated buffers
//! - `download_methods`: Download operations and HTTP request handling

pub mod client_core;
pub mod download_methods;
pub mod url_builder;

// Re-export core types for backward compatibility
pub use client_core::{
    CancelableFuture, CancellationToken, DownloadFileConfig, HttpAuth, HttpClient, HttpClientError,
};
// Legacy alias for backward compatibility
pub type QuicClient = HttpClient;
