//! Basic tachyonfx effect creation for progress visualization
//!
//! Zero-allocation effect builders with blazing-fast inline operations
//! for fundamental visual effects.

use super::super::config::ProgressRendererConfig;
use tachyonfx::{Effect, EffectTimer, Interpolation, Motion, fx};

/// Basic effect creation functionality
impl ProgressRendererConfig {
    /// Create shine effect for progress animation
    #[inline]
    pub fn create_shine_effect(&self, progress: f32) -> Effect {
        let intensity = progress * self.animation_intensity;
        let timer = EffectTimer::from_ms(1000, Interpolation::SineInOut);
        fx::hsl_shift(None, Some([0.0, 0.0, 20.0 * intensity]), timer)
    }

    /// Create celebration effect for completion
    #[inline]
    pub fn create_celebration_effect(&self) -> Effect {
        let timer = EffectTimer::from_ms(2000, Interpolation::BounceOut);
        fx::parallel(&[
            fx::hsl_shift(
                Some([360.0, 0.0, 0.0]),
                None,
                EffectTimer::from_ms(2000, Interpolation::Linear),
            ),
            fx::fade_to_fg(self.color_scheme.complete, timer),
        ])
    }

    /// Create pulse effect for active downloads
    #[inline]
    pub fn create_pulse_effect(&self, intensity: f32) -> Effect {
        let duration = (1000.0 / intensity.max(0.1)) as u32;
        let timer = EffectTimer::from_ms(duration, Interpolation::SineInOut);
        fx::hsl_shift(None, Some([0.0, 0.0, 30.0 * intensity]), timer)
    }

    /// Create wave effect for progress animation
    #[inline]
    pub fn create_wave_effect(&self) -> Effect {
        let timer = EffectTimer::from_ms(800, Interpolation::CircOut);
        fx::slide_in(
            Motion::LeftToRight,
            5,
            0,
            self.color_scheme.background,
            timer,
        )
    }

    /// Apply magical glow effect for focus states
    #[inline]
    pub fn create_focus_glow(&self) -> Effect {
        fx::hsl_shift(
            None,
            Some([0.0, 20.0, 40.0]), // Saturation and lightness boost for glow
            EffectTimer::from_ms(1200, Interpolation::SineInOut),
        )
    }
}
