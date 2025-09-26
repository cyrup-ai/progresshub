//! Progress calculator module with immutable data structures and pure functions
//!
//! This module provides a complete progress calculation system with true
//! immutability using the `im` crate and blazing-fast performance.

pub mod converter;
pub mod core;
pub mod formatter;
pub mod progress_calculator;
pub mod quantization_analyzer;
pub mod snapshot;
pub mod types;

// Re-export core types for convenience
pub use types::{
    ImmutableFileProgress, ImmutableModelProgress, ImmutableProgressState, ProgressPercentage,
};

// Re-export ProgressCalculator (the event type)
pub use progress_calculator::{ModelData, ProgressCalculator, TimingData};

// Re-export core calculation functions (internal use only - displays must use ProgressCalculator accessor methods)
pub(crate) use core::{calculate_percentage, get_progress_ratio};

// Internal formatting functions (displays must use ProgressCalculator accessor methods)
pub(crate) use formatter::{format_bytes, format_percentage, format_speed_display};

// Re-export snapshot creation functions
pub use snapshot::{from_manifest_and_progress, with_updated_file};

// Re-export conversion utilities
pub use converter::{
    calculate_overall_from_models, extract_progress_ratios, from_raw_progress_data,
};

// Re-export quantization analysis (centralized intelligence)
pub use quantization_analyzer::{FileAnalysisInfo, QuantizationAnalysis, QuantizationAnalyzer};
