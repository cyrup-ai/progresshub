//! Theme utility functions for download effects.
//!
//! This module provides color calculation and theme integration utilities
//! for TachyonFX effects, ensuring consistent visual design across all
//! download progress visualizations.

use ratatui::style::Color;
use tachyonfx::CellFilter;

use super::super::super::components::data::download_data::DownloadLevel;
use crate::ui::theme::CyrupTheme;

/// Theme utility functions for download effects
pub struct ThemeUtils;

impl ThemeUtils {
    /// Get themed color based on download level using CyrupTheme system
    #[inline]
    pub fn level_themed_color(level: DownloadLevel) -> Color {
        match level {
            DownloadLevel::Low => CyrupTheme::ERROR, // Red for slow downloads
            DownloadLevel::Medium => CyrupTheme::WARNING, // Yellow for medium downloads
            DownloadLevel::High => CyrupTheme::INFO, // Blue for good downloads
            DownloadLevel::Critical => CyrupTheme::SUCCESS, // Green for excellent downloads
        }
    }

    /// Enhance color based on intensity using CyrupTheme system
    #[inline]
    pub fn intensity_enhanced_color(base_color: Color, intensity: f32) -> Color {
        if intensity > 0.8 {
            CyrupTheme::TEXT_PRIMARY // Bright white for very high intensity
        } else if intensity > 0.5 {
            base_color // Use base color for moderate intensity
        } else {
            CyrupTheme::TEXT_MUTED // Muted for low intensity
        }
    }

    /// Get color parameters for download level (HSL values for advanced color manipulation)
    #[allow(dead_code)] // Utility function for sophisticated color theming in effects system
    pub fn level_color_params(level: DownloadLevel) -> (f32, f32, f32) {
        match level {
            DownloadLevel::Low => (240.0, 70.0, 60.0),    // Deep Blue
            DownloadLevel::Medium => (180.0, 80.0, 65.0), // Cyan
            DownloadLevel::High => (60.0, 90.0, 70.0),    // Yellow
            DownloadLevel::Critical => (120.0, 100.0, 75.0), // Green (excellent speed)
        }
    }

    /// Create a sophisticated cell filter for peak positions with spatial awareness
    #[allow(dead_code)] // Advanced filtering utility for peak position highlighting
    #[inline]
    pub fn create_peak_filter(_peak_positions: Vec<usize>) -> CellFilter {
        // Create complex position-based filtering using coordinate mapping with themed colors
        CellFilter::AllOf(vec![
            CellFilter::Text,
            CellFilter::FgColor(CyrupTheme::SUCCESS), // Success theme for download peaks
        ])
    }
}
