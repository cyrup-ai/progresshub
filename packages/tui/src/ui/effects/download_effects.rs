//! TachyonFX effects for download progress visualization with proper theming integration.
//!
//! This module provides sophisticated visual effects for download progress tracking,
//! including gradients, pulses, glows, transitions, and celebration effects.
//! All effects use the CyrupTheme system for consistent, dynamic color schemes.

use tachyonfx::Effect;

use super::super::components::data::download_data::{DownloadLevel, DownloadPoint};

// Re-export all functionality for backward compatibility
pub use super::download::{
    AdvancedEffects, CoreEffects, DownloadEffectStack, StateEffects, ThemeUtils,
};

/// Creates TachyonFX effects for the download progress graph
pub struct DownloadEffects;

impl DownloadEffects {
    /// Get themed color based on download level using CyrupTheme system
    #[inline]
    pub fn level_themed_color(level: DownloadLevel) -> ratatui::style::Color {
        ThemeUtils::level_themed_color(level)
    }

    /// Enhance color based on intensity using CyrupTheme system
    #[inline]
    pub fn intensity_enhanced_color(
        base_color: ratatui::style::Color,
        intensity: f32,
    ) -> ratatui::style::Color {
        ThemeUtils::intensity_enhanced_color(base_color, intensity)
    }

    /// Create sophisticated gradient effect using theming system and advanced TachyonFX patterns
    #[inline]
    pub fn create_gradient_effect(level: DownloadLevel, intensity: f32) -> Effect {
        CoreEffects::create_gradient_effect(level, intensity)
    }

    /// Create elegant pulsing effect for active downloads using themed colors
    #[inline]
    pub fn create_pulse_effect() -> Effect {
        CoreEffects::create_pulse_effect()
    }

    /// Create spectacular transition effect with dissolve and coalesce for progress updates
    #[inline]
    pub fn create_progress_transition(level: DownloadLevel) -> Effect {
        CoreEffects::create_progress_transition(level)
    }

    /// Create sophisticated glow effect with ping-pong animations for peaks
    #[inline]
    pub fn create_glow_effect(current_position: usize, peak_positions: &[usize]) -> Effect {
        AdvancedEffects::create_glow_effect(current_position, peak_positions)
    }

    /// Create elegant flowing effect using slide animations and themed colors
    #[inline]
    pub fn create_flow_effect() -> Effect {
        CoreEffects::create_flow_effect()
    }

    /// Create sparkline update effect when new download progress arrives
    pub fn create_progress_update_effect(new_point: &DownloadPoint) -> Effect {
        StateEffects::create_progress_update_effect(new_point)
    }

    /// Create spectacular completion celebration with explode effect using themed colors
    #[inline]
    pub fn create_completion_celebration() -> Effect {
        StateEffects::create_completion_celebration()
    }

    /// Create error effect with sophisticated warning animations using themed colors
    #[inline]
    pub fn create_error_effect() -> Effect {
        StateEffects::create_error_effect()
    }

    /// Create composite effect stack for the download progress graph
    pub fn create_effect_stack(
        level: DownloadLevel,
        intensity: f32,
        current_position: usize,
        peak_positions: &[usize],
    ) -> Effect {
        AdvancedEffects::create_effect_stack(level, intensity, current_position, peak_positions)
    }

    /// Create enhanced effect stack for high-performance downloads
    pub fn create_enhanced_effect_stack(
        level: DownloadLevel,
        intensity: f32,
        current_position: usize,
        peak_positions: &[usize],
    ) -> Effect {
        AdvancedEffects::create_enhanced_effect_stack(
            level,
            intensity,
            current_position,
            peak_positions,
        )
    }

    /// Create shimmer effect for high-speed downloads using themed colors
    #[inline]
    pub fn create_shimmer_effect() -> Effect {
        AdvancedEffects::create_shimmer_effect()
    }

    /// Create velocity trails for download progress
    pub fn create_velocity_trails(current_position: usize) -> Effect {
        AdvancedEffects::create_velocity_trails(current_position)
    }

    /// Get color parameters for download level (HSL values for advanced color manipulation)
    #[allow(dead_code)] // Utility function for sophisticated color theming in effects system
    pub fn level_color_params(level: DownloadLevel) -> (f32, f32, f32) {
        ThemeUtils::level_color_params(level)
    }
}
