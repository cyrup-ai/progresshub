//! Configuration structures for magical progress renderer
//!
//! Zero-allocation configuration management with elegant defaults
//! and blazing-fast access patterns.

use crate::ui::{progress_bars::ProgressBarType, theme::CyrupTheme};
use ratatui::style::Color;

/// Configuration for tachyonfx effect enhancement
#[derive(Debug, Clone)]
pub struct ProgressRendererConfig {
    /// Enable magical tachyonfx effects
    pub enable_effects: bool,
    /// Default progress bar type for effect selection
    pub default_bar_type: ProgressBarType,
    /// Effect intensity (0.0 to 1.0)
    pub animation_intensity: f32,
    /// Enable spectacular celebration effects on completion
    pub enable_celebrations: bool,
    /// Color scheme for enhanced effects
    pub color_scheme: ProgressColorScheme,
}

/// Color scheme configuration for progress bars
#[derive(Debug, Clone)]
pub struct ProgressColorScheme {
    pub queued: Color,
    pub starting: Color,
    pub active: Color,
    pub completing: Color,
    pub complete: Color,
    pub paused: Color,
    pub error: Color,
    pub background: Color,
    pub border: Color,
}

impl Default for ProgressColorScheme {
    #[inline]
    fn default() -> Self {
        Self {
            queued: CyrupTheme::TEXT_MUTED,
            starting: CyrupTheme::INFO,
            active: CyrupTheme::ACCENT,
            completing: CyrupTheme::WARNING,
            complete: CyrupTheme::SUCCESS,
            paused: CyrupTheme::TEXT_SECONDARY,
            error: CyrupTheme::ERROR,
            background: CyrupTheme::BG_PRIMARY,
            border: CyrupTheme::BORDER,
        }
    }
}

impl Default for ProgressRendererConfig {
    #[inline]
    fn default() -> Self {
        Self {
            enable_effects: true,
            default_bar_type: ProgressBarType::Flow,
            animation_intensity: 0.8,
            enable_celebrations: true,
            color_scheme: ProgressColorScheme::default(),
        }
    }
}
