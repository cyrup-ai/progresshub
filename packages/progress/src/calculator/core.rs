//! Pure calculation functions for progress computations
//!
//! This module contains stateless, pure functions for calculating progress
//! percentages and aggregations with zero allocations and blazing performance.

use crate::calculator::types::ProgressPercentage;

/// Calculate progress percentage from bytes downloaded and total bytes
///
/// Pure function with specification-compliant calculation using decimal precision
/// for smooth updates. Formula: (downloaded/total * 1000.0).round() / 10.0
///
/// # Arguments
/// * `downloaded` - Number of bytes downloaded
/// * `total` - Total bytes to download
///
/// # Returns
/// ProgressPercentage with value 0.0-100.0, or None if total is 0
#[inline]
pub fn calculate_percentage(downloaded: u64, total: u64) -> ProgressPercentage {
    if total == 0 {
        return ProgressPercentage {
            value: None,
            is_complete: false,
            is_indeterminate: true,
        };
    }

    // Specification-compliant calculation with decimal precision and exact 100.0 for completion
    let (percentage, is_complete) = if downloaded >= total && total > 0 {
        // Ensure exactly 100.0 for completed downloads to prevent floating-point precision issues
        (100.0, true)
    } else {
        let calc_percentage =
            ((downloaded as f64 / total as f64 * 1000.0).round() / 10.0).min(100.0);
        (calc_percentage, false)
    };

    ProgressPercentage {
        value: Some(percentage),
        is_complete,
        is_indeterminate: false,
    }
}

/// Get progress as ratio (0.0 to 1.0) for gauge widgets
///
/// Pure function providing normalized progress value for visual progress bars.
/// Handles edge cases and ensures valid range.
///
/// # Arguments
/// * `downloaded` - Number of bytes downloaded
/// * `total` - Total bytes to download
///
/// # Returns
/// Progress ratio between 0.0 and 1.0
#[inline]
pub fn get_progress_ratio(downloaded: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (downloaded as f64 / total as f64).clamp(0.0, 1.0)
    }
}

/// Check if progress is complete
///
/// Pure function for defensive completion check that handles edge cases.
/// Used throughout the system for state management.
///
/// # Arguments
/// * `downloaded` - Number of bytes downloaded
/// * `total` - Total bytes to download
///
/// # Returns
/// True if download is complete, false otherwise
#[inline]
pub fn is_progress_complete(downloaded: u64, total: u64) -> bool {
    total > 0 && downloaded >= total
}

/// Aggregate bytes from multiple (downloaded, total) pairs
///
/// Pure function for summing progress across multiple items.
/// Used for model-level and overall-level aggregations.
///
/// # Arguments
/// * `items` - Iterator of (bytes_downloaded, total_bytes) tuples
///
/// # Returns
/// Tuple of (total_downloaded, total_expected)
pub fn aggregate_bytes<I>(items: I) -> (u64, u64)
where
    I: Iterator<Item = (u64, u64)>,
{
    items.fold((0u64, 0u64), |(acc_down, acc_total), (down, total)| {
        (acc_down + down, acc_total + total)
    })
}
