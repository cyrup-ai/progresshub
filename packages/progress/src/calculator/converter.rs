//! Data conversion utilities for progress tracking
//!
//! This module provides conversion functions between different progress data
//! formats and generic calculation utilities.

use crate::calculator::types::{ImmutableProgressState, RawProgressData};

/// Calculate overall progress from aggregated model data
///
/// Stateless function that operates purely on input data.
/// Returns (total_downloaded, total_expected, progress_percentage).
///
/// # Arguments  
/// * `model_data` - Iterator of (bytes_downloaded, total_bytes) tuples
///
/// # Returns
/// Tuple of (bytes_downloaded, total_bytes, percentage_value)
pub fn calculate_overall_from_models<I>(model_data: I) -> (u64, u64, f64)
where
    I: Iterator<Item = (u64, u64)>,
{
    let (total_downloaded, total_expected) = crate::calculator::core::aggregate_bytes(model_data);
    let progress_result =
        crate::calculator::core::calculate_percentage(total_downloaded, total_expected);
    let progress_percentage = progress_result.value.unwrap_or(0.0);

    (total_downloaded, total_expected, progress_percentage)
}

/// Convert raw progress data into immutable progress state
///
/// Generic conversion function that creates immutable state from raw data.
///
/// # Arguments
/// * `model_data` - Collection of raw model progress data
///
/// # Returns
/// New ImmutableProgressState
pub fn from_raw_progress_data(model_data: RawProgressData) -> ImmutableProgressState {
    crate::calculator::snapshot::from_manifest_and_progress(
        &std::collections::HashMap::new(),
        &model_data,
    )
}

/// Extract progress ratios from immutable state
///
/// Utility function for converting progress data into ratios for UI widgets.
///
/// # Arguments
/// * `state` - Immutable progress state
///
/// # Returns
/// Vector of (model_id, progress_ratio) tuples
pub fn extract_progress_ratios(state: &ImmutableProgressState) -> Vec<(String, f64)> {
    state
        .models
        .iter()
        .map(|model| {
            let ratio = crate::calculator::core::get_progress_ratio(
                model.bytes_downloaded,
                model.total_bytes,
            );
            (model.model_id.clone(), ratio)
        })
        .collect()
}
