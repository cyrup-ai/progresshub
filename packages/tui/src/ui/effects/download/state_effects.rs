//! State-driven TachyonFX effects for download events.
//!
//! This module provides event-driven visual effects for download state changes,
//! including completion celebrations, error effects, and progress update animations.
//! All effects use zero-allocation patterns and the CyrupTheme system.

use tachyonfx::{Effect, EffectTimer, Interpolation, Motion, fx};

use super::super::super::components::data::download_data::DownloadPoint;
use super::theme_utils::ThemeUtils;
use crate::ui::theme::CyrupTheme;

/// State-driven effects implementation for download event visualization
pub struct StateEffects;

impl StateEffects {
    /// Create sparkline update effect when new download progress arrives
    pub fn create_progress_update_effect(new_point: &DownloadPoint) -> Effect {
        let level = new_point.level();
        let base_color = ThemeUtils::level_themed_color(level);

        fx::sequence(&[
            fx::sweep_in(
                Motion::LeftToRight,
                15,
                0,
                base_color,
                EffectTimer::from_ms(250, Interpolation::ExpoOut),
            ),
            fx::parallel(&[
                fx::fade_to_fg(
                    base_color,
                    EffectTimer::from_ms(150, Interpolation::QuadOut),
                ),
                fx::hsl_shift_fg(
                    [0.0, 10.0, 10.0],
                    EffectTimer::from_ms(300, Interpolation::SineOut),
                ),
            ]),
        ])
    }

    /// Create spectacular completion celebration with explode effect using themed colors
    #[inline]
    pub fn create_completion_celebration() -> Effect {
        let success_color = CyrupTheme::SUCCESS;
        let warning_color = CyrupTheme::WARNING;

        fx::sequence(&[
            // Spectacular explosion effect
            fx::parallel(&[
                fx::explode(20.0, 3.0, EffectTimer::from_ms(600, Interpolation::ExpoOut)),
                fx::sweep_in(
                    Motion::LeftToRight,
                    40,
                    8,
                    success_color,
                    EffectTimer::from_ms(500, Interpolation::BounceOut),
                ),
                fx::sweep_in(
                    Motion::RightToLeft,
                    40,
                    8,
                    warning_color,
                    EffectTimer::from_ms(550, Interpolation::BounceOut),
                ),
            ]),
            // Celebratory pulse sequence
            fx::sequence(&[
                fx::ping_pong(fx::fade_to_fg(
                    success_color,
                    EffectTimer::from_ms(400, Interpolation::BounceOut),
                )),
                fx::hsl_shift_fg(
                    [15.0, 10.0, 20.0],
                    EffectTimer::from_ms(300, Interpolation::SineOut),
                ),
            ]),
        ])
    }

    /// Create error effect with sophisticated warning animations using themed colors
    #[inline]
    pub fn create_error_effect() -> Effect {
        let error_color = CyrupTheme::ERROR;

        fx::sequence(&[
            // Warning flash
            fx::fade_to_fg(
                error_color,
                EffectTimer::from_ms(200, Interpolation::ExpoOut),
            ),
            // Pulsing error indication
            fx::repeating(fx::ping_pong(fx::hsl_shift_fg(
                [0.0, 15.0, -10.0],
                EffectTimer::from_ms(800, Interpolation::SineInOut),
            ))),
        ])
    }
}
