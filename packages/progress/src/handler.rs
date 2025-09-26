//! Progress handler implementations for download progress tracking.
//!  
//! This module provides the main progress handler interface and re-exports
//! specialized handler implementations for different use cases.

// Re-export all handler types from the focused modules
pub use crate::handlers::{
    AtomicFileState, FileState, NoOpProgressHandler,
};

// Keep the main handler exports for backward compatibility
pub use crate::handlers::noop_handler::NoOpProgressHandler as NoOpHandler;
