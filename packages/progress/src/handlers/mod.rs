//! Progress handler implementations with specialized functionality.
//!
//! This module provides focused handler implementations optimized for different
//! use cases, from no-op testing handlers to atomic state management.

pub mod atomic_state;
pub mod noop_handler;

// Re-export main handler types for convenience
pub use atomic_state::{AtomicFileState, FileState};
pub use noop_handler::NoOpProgressHandler;
