//! Error types specific to QUIC operations

use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuicError {
    #[error("QUIC connection error: {0}")]
    Connection(String),

    #[error("QUIC stream error: {0}")]
    Stream(String),

    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("Operation timed out after {:?}", .0)]
    Timeout(Duration),

    #[error("Operation was cancelled")]
    Canceled,

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Timeout after {0} seconds")]
    TimeoutAfter(u64),
}
