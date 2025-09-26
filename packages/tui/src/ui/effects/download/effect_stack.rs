//! Effect stack management for download progress visualization.
//!
//! This module provides sophisticated effect stack management for combining
//! and orchestrating multiple TachyonFX effects in download progress displays.
//! All effects use zero-allocation patterns and production-ready state management.

use std::time::{Duration, Instant};
use tachyonfx::{Effect, Shader};

use super::super::super::components::data::download_data::DownloadLevel;
use super::advanced_effects::AdvancedEffects;
use super::core_effects::CoreEffects;
use super::state_effects::StateEffects;

/// Helper to combine multiple download effects efficiently
#[derive(Debug)]
pub struct DownloadEffectStack {
    effects: Vec<Effect>,
    last_update: Instant,
    celebration_mode: bool,
    error_mode: bool,
}

impl Default for DownloadEffectStack {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            last_update: Instant::now(),
            celebration_mode: false,
            error_mode: false,
        }
    }
}

impl DownloadEffectStack {
    /// Create a new download effect stack
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an effect to the stack
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Add celebration effects for completed downloads
    pub fn add_celebration(&mut self) {
        self.celebration_mode = true;
        self.effects
            .push(StateEffects::create_completion_celebration());
    }

    /// Add error effects for failed downloads
    pub fn add_error_effect(&mut self) {
        self.error_mode = true;
        self.effects.push(StateEffects::create_error_effect());
    }

    /// Update all effects with frame duration
    pub fn update(&mut self, _frame_duration: Duration) {
        let now = Instant::now();
        self.last_update = now;

        // Remove completed effects
        self.effects.retain(|effect| !effect.done());

        // Reset modes if no effects are running
        if self.effects.is_empty() {
            self.celebration_mode = false;
            self.error_mode = false;
        }
    }

    /// Check if any effects are still running
    pub fn has_running_effects(&self) -> bool {
        self.effects.iter().any(|effect| effect.running())
    }

    /// Check if in celebration mode
    pub fn is_celebrating(&self) -> bool {
        self.celebration_mode
    }

    /// Check if in error mode
    pub fn is_showing_error(&self) -> bool {
        self.error_mode
    }

    /// Get the primary effect for rendering
    pub fn primary_effect(&self) -> Option<&Effect> {
        self.effects.first()
    }

    /// Get all active effects
    pub fn active_effects(&self) -> &[Effect] {
        &self.effects
    }

    /// Clear all effects and reset modes
    pub fn clear(&mut self) {
        self.effects.clear();
        self.celebration_mode = false;
        self.error_mode = false;
    }

    /// Create base effects for normal download progress
    pub fn create_base_effects(&mut self, level: DownloadLevel, intensity: f32) {
        self.clear();
        self.effects
            .push(CoreEffects::create_gradient_effect(level, intensity));
        self.effects.push(CoreEffects::create_pulse_effect());
        self.effects.push(CoreEffects::create_flow_effect());
    }

    /// Create enhanced effects for high-speed downloads
    pub fn create_enhanced_effects(
        &mut self,
        level: DownloadLevel,
        intensity: f32,
        current_position: usize,
    ) {
        self.clear();
        self.effects
            .push(CoreEffects::create_gradient_effect(level, intensity));
        self.effects.push(CoreEffects::create_pulse_effect());
        self.effects.push(CoreEffects::create_flow_effect());
        self.effects.push(AdvancedEffects::create_shimmer_effect());
        self.effects
            .push(AdvancedEffects::create_velocity_trails(current_position));
    }
}
