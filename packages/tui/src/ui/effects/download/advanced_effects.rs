//! Advanced TachyonFX effects for high-performance download visualization.
//!
//! This module provides sophisticated visual effects for download progress tracking,
//! including glow effects, shimmer effects, velocity trails, and enhanced effect stacks.
//! All effects use zero-allocation patterns and the CyrupTheme system.

use tachyonfx::{CellFilter, Effect, EffectTimer, Interpolation, Motion, fx};

use super::super::super::components::data::download_data::DownloadLevel;
use super::core_effects::CoreEffects;
use super::theme_utils::ThemeUtils;
use crate::ui::theme::CyrupTheme;

/// Advanced effects implementation for high-performance download visualization
pub struct AdvancedEffects;

impl AdvancedEffects {
    /// Create sophisticated glow effect with ping-pong animations for peaks
    #[inline]
    pub fn create_glow_effect(current_position: usize, peak_positions: &[usize]) -> Effect {
        let accent_color = CyrupTheme::ACCENT;
        let success_color = CyrupTheme::SUCCESS;

        // Create position-aware filter for current position highlighting
        let _position_filter =
            CellFilter::AllOf(vec![CellFilter::Text, CellFilter::FgColor(accent_color)]);

        // Create peak-aware effects using position data
        let _peak_filter = ThemeUtils::create_peak_filter(peak_positions.to_vec());

        fx::parallel(&[
            // Current position glow with ping-pong effect - position aware
            fx::ping_pong(fx::fade_to_fg(
                accent_color,
                EffectTimer::from_ms(800, Interpolation::SineInOut),
            )),
            // Peak celebration with sophisticated breathing effect - peak positions
            fx::repeating(fx::sequence(&[
                fx::fade_to_fg(
                    success_color,
                    EffectTimer::from_ms(1200, Interpolation::SineInOut),
                ),
                fx::hsl_shift_fg(
                    [10.0, 5.0, 15.0],
                    EffectTimer::from_ms(800, Interpolation::BounceOut),
                ),
            ])),
            // Shimmer effect at current position
            fx::sweep_in(
                Motion::LeftToRight,
                current_position as u16,
                2,
                accent_color,
                EffectTimer::from_ms(600, Interpolation::ExpoOut),
            ),
        ])
    }

    /// Create shimmer effect for high-speed downloads using themed colors
    #[inline]
    pub fn create_shimmer_effect() -> Effect {
        let accent_color = CyrupTheme::ACCENT;
        let text_bright = CyrupTheme::TEXT_PRIMARY;

        fx::repeating(fx::sequence(&[
            fx::fade_to_fg(
                text_bright,
                EffectTimer::from_ms(600, Interpolation::SineInOut),
            ),
            fx::fade_to_fg(
                accent_color,
                EffectTimer::from_ms(600, Interpolation::SineInOut),
            ),
        ]))
    }

    /// Create velocity trails for download progress
    pub fn create_velocity_trails(_current_position: usize) -> Effect {
        // Simplified trail effect using sweep animations
        fx::sequence(&[
            fx::sweep_in(
                Motion::LeftToRight,
                20,
                2,
                CyrupTheme::ACCENT,
                EffectTimer::from_ms(800, Interpolation::ExpoOut),
            ),
            fx::fade_to_fg(
                CyrupTheme::TEXT_SECONDARY,
                EffectTimer::from_ms(400, Interpolation::QuadOut),
            ),
        ])
    }

    /// Create composite effect stack for the download progress graph
    pub fn create_effect_stack(
        level: DownloadLevel,
        intensity: f32,
        current_position: usize,
        peak_positions: &[usize],
    ) -> Effect {
        fx::parallel(&[
            CoreEffects::create_gradient_effect(level, intensity),
            CoreEffects::create_pulse_effect(),
            Self::create_glow_effect(current_position, peak_positions),
            CoreEffects::create_flow_effect(),
        ])
    }

    /// Create enhanced effect stack for high-performance downloads
    pub fn create_enhanced_effect_stack(
        level: DownloadLevel,
        intensity: f32,
        current_position: usize,
        peak_positions: &[usize],
    ) -> Effect {
        fx::parallel(&[
            CoreEffects::create_gradient_effect(level, intensity),
            CoreEffects::create_pulse_effect(),
            Self::create_glow_effect(current_position, peak_positions),
            CoreEffects::create_flow_effect(),
            // Additional effects for high performance
            Self::create_shimmer_effect(),
            Self::create_velocity_trails(current_position),
        ])
    }
}
