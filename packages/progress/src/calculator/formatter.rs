//! String formatting functions for progress display
//!
//! This module provides consistent, ergonomic string formatting for all
//! progress display permutations used throughout the TUI system.

use crate::calculator::{core, types::ProgressPercentage};

/// Format percentage for display with consistent precision
///
/// Provides clean, user-friendly percentage formatting with right alignment.
///
/// # Arguments
/// * `progress` - ProgressPercentage to format
///
/// # Returns
/// Formatted string for display (e.g., " 45.0%", "100.0%", "  ---")
pub fn format_percentage(progress: &ProgressPercentage) -> String {
    match progress.value {
        Some(percentage) => format!("{percentage:>5.1}%"),
        None => "  ---".to_string(),
    }
}

/// Get percentage as rounded string (e.g., "45%", "100%", "---")
pub fn get_percentage_rounded_str(downloaded: u64, total: u64) -> String {
    let progress = core::calculate_percentage(downloaded, total);
    match progress.value {
        Some(percentage) => format!("{:.0}%", percentage.round()),
        None => "---".to_string(),
    }
}

/// Get percentage with parentheses format (e.g., "Overall Progress ( 45.2%)")
pub fn get_percentage_with_parens_str(label: &str, downloaded: u64, total: u64) -> String {
    let progress = core::calculate_percentage(downloaded, total);
    match progress.value {
        Some(percentage) => format!("{label} ({percentage:>5.1}%)"),
        None => format!("{label} (  ---)"),
    }
}

/// Format remaining models and files display
pub fn format_remaining_models_and_files(models_count: usize, files_count: usize) -> String {
    format!("Remaining: {models_count} ■ Models | {files_count} ▫ Files")
}

/// Format speed display (e.g., "3.8 GB/s", "156 MB/s")
pub fn format_speed_display(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{bytes_per_sec:.0} B/s")
    } else if bytes_per_sec < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else if bytes_per_sec < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB/s", bytes_per_sec / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format bytes for display with appropriate units
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let unit_index = (bytes_f.log(THRESHOLD).floor() as usize).min(UNITS.len() - 1);
    let value = bytes_f / THRESHOLD.powi(unit_index as i32);

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else if value >= 100.0 {
        format!("{:.0} {}", value, UNITS[unit_index])
    } else if value >= 10.0 {
        format!("{:.1} {}", value, UNITS[unit_index])
    } else {
        format!("{:.2} {}", value, UNITS[unit_index])
    }
}
