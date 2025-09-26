//! Common types and traits for progresshub
//!
//! This crate provides shared types that are used across multiple packages
//! to avoid circular dependencies between client packages and progress package.

pub mod events;
pub mod hash;
pub mod noop_handler;
pub mod traits;

pub use events::{FinalizationLevel, ManifestEvent, ProgressEvent, RawDownloadEvent, SimpleFileInfo, SimpleRepoManifest};
pub use hash::generate_model_hash;
pub use noop_handler::NoOpProgressHandler;
pub use traits::{DownloadProgress, FileStatus, ProgressHandler};
