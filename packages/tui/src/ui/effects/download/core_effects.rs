//! Core TachyonFX effects for download progress visualization.
//!
//! This module provides the fundamental visual effects for download progress tracking,
//! including gradients, pulses, transitions, and flowing animations.
//! All effects use zero-allocation patterns and the CyrupTheme system.

use tachyonfx::{Effect, EffectTimer, Interpolation, Motion, fx};

use super::super::super::components::data::download_data::DownloadLevel;
use super::theme_utils::ThemeUtils;

/// Core effects implementation for download progress visualization
pub struct CoreEffects;

impl CoreEffects {
    /// Create sophisticated gradient effect using theming system and advanced TachyonFX patterns
    #[inline]
    pub fn create_gradient_effect(level: DownloadLevel, intensity: f32) -> Effect {
        let base_color = ThemeUtils::level_themed_color(level);
        let enhanced_color = ThemeUtils::intensity_enhanced_color(base_color, intensity);
        let (hue, saturation, lightness) = ThemeUtils::level_color_params(level);

        // Use sophisticated TachyonFX combination with HSL parameters
        fx::parallel(&[
            fx::fade_to_fg(
                enhanced_color,
                EffectTimer::from_ms(1200, Interpolation::CubicInOut),
            ),
            fx::sweep_in(
                Motion::LeftToRight,
                25,
                3,
                base_color,
                EffectTimer::from_ms(800, Interpolation::ExpoOut),
            ),
            fx::hsl_shift_fg(
                [
                    hue * 0.1,
                    saturation * intensity * 0.2,
                    lightness * intensity * 0.15,
                ],
                EffectTimer::from_ms(2000, Interpolation::SineInOut),
            ),
        ])
    }

    /// Create elegant pulsing effect for active downloads using themed colors
    #[inline]
    pub fn create_pulse_effect() -> Effect {
        use crate::ui::theme::CyrupTheme;

        let accent_color = CyrupTheme::ACCENT;
        let success_color = CyrupTheme::SUCCESS;

        fx::repeating(fx::sequence(&[
            fx::fade_to_fg(
                accent_color,
                EffectTimer::from_ms(1000, Interpolation::SineInOut),
            ),
            fx::fade_to_fg(
                success_color,
                EffectTimer::from_ms(1000, Interpolation::SineInOut),
            ),
        ]))
    }

    /// Create spectacular transition effect with dissolve and coalesce for progress updates
    #[inline]
    pub fn create_progress_transition(level: DownloadLevel) -> Effect {
        let target_color = ThemeUtils::level_themed_color(level);

        fx::sequence(&[
            fx::dissolve(EffectTimer::from_ms(150, Interpolation::ExpoOut)),
            fx::parallel(&[
                fx::coalesce_from(
                    ratatui::style::Style::default().fg(target_color),
                    EffectTimer::from_ms(400, Interpolation::BounceOut),
                ),
                fx::sweep_in(
                    Motion::LeftToRight,
                    30,
                    5,
                    target_color,
                    EffectTimer::from_ms(350, Interpolation::CubicOut),
                ),
            ]),
        ])
    }

    /// Create elegant flowing effect using slide animations and themed colors
    #[inline]
    pub fn create_flow_effect() -> Effect {
        use crate::ui::theme::CyrupTheme;

        let info_color = CyrupTheme::INFO;
        let accent_color = CyrupTheme::ACCENT;

        fx::repeating(fx::sequence(&[
            fx::slide_in(
                Motion::LeftToRight,
                15,
                2,
                info_color,
                EffectTimer::from_ms(2000, Interpolation::SineInOut),
            ),
            fx::slide_in(
                Motion::RightToLeft,
                15,
                2,
                accent_color,
                EffectTimer::from_ms(2000, Interpolation::SineInOut),
            ),
        ]))
    }
}
