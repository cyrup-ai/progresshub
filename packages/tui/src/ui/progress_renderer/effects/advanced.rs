//! Advanced tachyonfx effects with mathematical calculations
//!
//! Zero-allocation sophisticated visual effects using custom effect functions
//! and complex mathematical transformations for magical progress visualization.

use super::super::config::ProgressRendererConfig;
use crate::ui::progress_bars::{ProgressBarType, ProgressData};
use std::time::Instant;
use tachyonfx::{Effect, EffectTimer, Interpolation, Motion, fx};

/// Advanced effect creation functionality
impl ProgressRendererConfig {
    /// Create advanced magical effects based on progress bar type and data
    #[inline]
    pub fn create_progress_effect(
        &self,
        bar_type: ProgressBarType,
        progress_data: ProgressData,
    ) -> Effect {
        match bar_type {
            ProgressBarType::Flow => self.create_flow_effect(progress_data),
            ProgressBarType::Pulse => self.create_pulse_effect_advanced(progress_data),
            ProgressBarType::Wave => self.create_wave_effect_advanced(progress_data),
            ProgressBarType::Gradient => self.create_gradient_effect(progress_data),
            ProgressBarType::Particle => self.create_particle_effect(progress_data),
            ProgressBarType::Celebration => self.create_celebration_effect_advanced(progress_data),
        }
    }

    /// Create sophisticated flow effect with data packet visualization
    #[inline]
    pub fn create_flow_effect(&self, progress_data: ProgressData) -> Effect {
        let intensity = progress_data.normalized_speed();
        let timer =
            EffectTimer::from_ms((1000.0 / intensity.max(0.1)) as u32, Interpolation::Linear);

        fx::sequence(&[
            fx::slide_in(Motion::LeftToRight, 10, 0, self.color_scheme.active, timer),
            fx::hsl_shift(None, Some([0.0, 10.0 * intensity, 10.0 * intensity]), timer),
        ])
    }

    /// Create advanced pulse effect with rhythmic pulsing
    #[inline]
    pub fn create_pulse_effect_advanced(&self, progress_data: ProgressData) -> Effect {
        let intensity = progress_data.normalized_speed();
        let duration = (800.0 / intensity.max(0.1)) as u32;
        let timer = EffectTimer::from_ms(duration, Interpolation::SineInOut);

        fx::ping_pong(fx::hsl_shift(
            None,
            Some([0.0, 20.0 * intensity, 30.0 * intensity]),
            timer,
        ))
    }

    /// Create complex wave effect using custom effect function with sine waves
    #[inline]
    pub fn create_wave_effect_advanced(&self, progress_data: ProgressData) -> Effect {
        let intensity = progress_data.normalized_speed();
        let start_time = Instant::now();
        let timer = EffectTimer::from_ms(1200, Interpolation::Linear);

        fx::effect_fn(start_time, timer, move |state, _ctx, cell_iter| {
            let elapsed_ms = state.elapsed().as_millis() as f32;
            let wave_frequency = 0.02 * intensity;
            let wave_amplitude = 30.0 * intensity;

            cell_iter
                .filter(|(_, cell)| cell.symbol() != " ")
                .enumerate()
                .for_each(|(i, (_pos, cell))| {
                    let wave_offset = (i as f32 * wave_frequency + elapsed_ms * 0.005).sin();
                    let brightness_shift = wave_amplitude * wave_offset;

                    // Apply wave-based brightness modulation
                    let current_color = cell.fg;
                    let hsl = tachyonfx::color_to_hsl(&current_color);
                    let new_lightness = (hsl.2 + brightness_shift).clamp(0.0, 100.0);
                    let new_color = tachyonfx::color_from_hsl(hsl.0, hsl.1, new_lightness);
                    cell.set_fg(new_color);
                });
        })
    }

    /// Create gradient effect with smooth color transitions
    #[inline]
    pub fn create_gradient_effect(&self, progress_data: ProgressData) -> Effect {
        let progress = progress_data.progress;
        let timer = EffectTimer::from_ms(2000, Interpolation::CircOut);

        let start_color = self.color_scheme.starting;
        let end_color = if progress >= 1.0 {
            self.color_scheme.complete
        } else {
            self.color_scheme.active
        };

        fx::sequence(&[
            fx::fade_from_fg(start_color, timer),
            fx::fade_to_fg(end_color, timer),
        ])
    }

    /// Create particle effect with discrete data packet visualization
    #[inline]
    pub fn create_particle_effect(&self, progress_data: ProgressData) -> Effect {
        let speed = progress_data.normalized_speed();
        let start_time = Instant::now();
        let timer = EffectTimer::from_ms(600, Interpolation::Linear);

        fx::effect_fn(start_time, timer, move |state, _ctx, cell_iter| {
            let elapsed_ms = state.elapsed().as_millis() as f32;
            let particle_speed = speed * 0.1;

            cell_iter
                .filter(|(_, cell)| cell.symbol() != " ")
                .enumerate()
                .for_each(|(i, (_pos, cell))| {
                    // Create moving particle effect
                    let particle_offset = (elapsed_ms * particle_speed + i as f32 * 0.3) % 1.0;

                    if particle_offset < 0.1 {
                        // Particle is "passing through" this cell
                        let particle_intensity = (0.1 - particle_offset) * 10.0;
                        let brightness_boost = 50.0 * particle_intensity;

                        let current_color = cell.fg;
                        let hsl = tachyonfx::color_to_hsl(&current_color);
                        let new_lightness = (hsl.2 + brightness_boost).clamp(0.0, 100.0);
                        let particle_color = tachyonfx::color_from_hsl(hsl.0, 100.0, new_lightness);
                        cell.set_fg(particle_color);
                    }
                });
        })
    }

    /// Create spectacular completion celebration with layered effects
    #[inline]
    pub fn create_celebration_effect_advanced(&self, _progress_data: ProgressData) -> Effect {
        let timer_fast = EffectTimer::from_ms(1000, Interpolation::BounceOut);
        let timer_slow = EffectTimer::from_ms(3000, Interpolation::Linear);
        let start_time = Instant::now();

        fx::parallel(&[
            // Rainbow color shift
            fx::hsl_shift(Some([360.0, 0.0, 0.0]), None, timer_slow),
            // Burst effect
            fx::sequence(&[
                fx::fade_to_fg(self.color_scheme.complete, timer_fast),
                fx::dissolve(timer_fast),
                fx::coalesce(timer_fast),
            ]),
            // Sparkle effect using custom particle bursts
            fx::effect_fn(start_time, timer_slow, |state, _ctx, cell_iter| {
                let elapsed_ms = state.elapsed().as_millis() as f32;
                let sparkle_frequency = 0.05;

                cell_iter
                    .filter(|(_, cell)| cell.symbol() != " ")
                    .enumerate()
                    .for_each(|(i, (_pos, cell))| {
                        let sparkle_phase =
                            (i as f32 * sparkle_frequency + elapsed_ms * 0.01) % 1.0;

                        if sparkle_phase < 0.05 {
                            // Create sparkle burst
                            let sparkle_hue = (elapsed_ms * 0.5 + i as f32 * 30.0) % 360.0;
                            let sparkle_color = tachyonfx::color_from_hsl(sparkle_hue, 100.0, 80.0);
                            cell.set_fg(sparkle_color);
                        }
                    });
            }),
        ])
    }
}
