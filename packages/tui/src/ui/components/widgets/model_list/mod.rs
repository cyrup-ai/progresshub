//! Focused modules for model list widget implementation.
//!
//! This module provides blazing-fast, zero-allocation model list rendering
//! with elegant modular decomposition for optimal maintainability and performance.

pub mod block_utils;
pub mod column_layout;
pub mod column_renderer;
pub mod file_renderer;
pub mod model_collapsible;

// Minimal re-exports required by dependent modules
pub use column_layout::calculate_column_distribution;
pub use column_renderer::{ModelListStateAccess, render_column};

// ModelListWidget functionality integrated into CLI display
