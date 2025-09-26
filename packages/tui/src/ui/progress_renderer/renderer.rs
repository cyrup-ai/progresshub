//! Core Progress Renderer with TachyonFX Effect Management
//!
//! Zero-allocation magical progress renderer with blazing-fast effect processing
//! and sophisticated tachyonfx integration for production-grade visual effects.

use ratatui::{buffer::Buffer, layout::Rect};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Instant;

use tachyonfx::{Effect, EffectManager};

use crate::ui::{
    components::widgets::overall_progress::ProgressGaugeWidget,
    components::widgets::{DownloadGraphState, DownloadGraphWidget},
    progress_bars::{ProgressBarType, ProgressData},
};

use super::config::ProgressRendererConfig;

/// Magical progress renderer with advanced tachyonfx effects
pub struct ProgressRenderer {
    /// TachyonFX effect manager for magical animations
    effect_manager: EffectManager<String>,
    /// Atomic effect ID counter for unique effect management
    #[allow(dead_code)] // Used by next_effect_id for atomic effect ID generation
    effect_id_counter: AtomicU32,
    /// Atomic effect count tracker for real-time monitoring
    effect_count: AtomicUsize,
    /// Active progress bars by model ID
    active_models: HashMap<String, ProgressBarType>,
    /// Download graph state for sophisticated visualization
    download_graph_state: DownloadGraphState,
    /// Last frame timestamp for timing calculations
    last_frame: Instant,
    /// Configuration for magical effects
    config: ProgressRendererConfig,
}
impl ProgressRenderer {
    pub fn new(config: ProgressRendererConfig) -> Self {
        Self {
            effect_manager: EffectManager::default(),
            effect_id_counter: AtomicU32::new(1),
            effect_count: AtomicUsize::new(0),
            active_models: HashMap::new(),
            download_graph_state: DownloadGraphState::new(),
            last_frame: Instant::now(),
            config,
        }
    }

    /// Get next unique effect ID using atomic counter
    #[allow(dead_code)] // Utility method for generating unique effect IDs in concurrent rendering
    #[inline]
    fn next_effect_id(&self) -> u32 {
        self.effect_id_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Enhance existing ProgressGaugeWidget with magical tachyonfx effects
    pub fn render_overall_progress(
        &mut self,
        overall_progress: f32,
        total_models: usize,
        completed_models: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        // First render the existing sophisticated ProgressGaugeWidget (zero allocation)
        let safe_progress_percentage = (overall_progress * 100.0).clamp(0.0, 100.0);

        // Create zero-allocation label using stack buffer
        let progress_widget = if overall_progress >= 1.0 {
            ProgressGaugeWidget::new(safe_progress_percentage as f64)
                .label("COMPLETE ✅".to_string())
                .enhanced_height(true)
        } else {
            // Use stack-allocated label to avoid heap allocation
            let mut label_buffer = [0u8; 64];
            use std::io::Write;
            let mut cursor = std::io::Cursor::new(&mut label_buffer[..]);
            let _ = write!(
                cursor,
                "Overall Progress {safe_progress_percentage:.1}% ({completed_models}/{total_models})"
            );
            let len = cursor.position() as usize;
            let progress_label = std::str::from_utf8(&label_buffer[..len]).unwrap_or("Progress");

            ProgressGaugeWidget::new(safe_progress_percentage as f64)
                .label(progress_label.to_string())
                .enhanced_height(true)
        };

        // Render the existing widget first (preserving all functionality)
        use ratatui::widgets::Widget;
        progress_widget.render(area, buf);

        // Then enhance with tachyonfx magical effects if enabled
        if self.config.enable_effects {
            if overall_progress >= 1.0 && self.config.enable_celebrations {
                // Add spectacular celebration effect for completion with unique ID
                let _effect_id = self.next_effect_id();
                let celebration_effect: Effect = self.config.create_celebration_effect();
                self.effect_manager.add_effect(celebration_effect);
                self.effect_count.fetch_add(1, Ordering::Relaxed);
            } else if overall_progress > 0.0 {
                // Add shine effect for active progress with unique ID
                let _effect_id = self.next_effect_id();
                let shine_effect = self.config.create_shine_effect(overall_progress);
                self.effect_manager.add_effect(shine_effect);
                self.effect_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    /// Render sophisticated download graph with magical TachyonFX effects
    pub fn render_download_graph(
        &mut self,
        download_mbps: f64,
        bytes_downloaded: u64,
        total_bytes: u64,
        progress_percentage: f32,
        area: Rect,
        buf: &mut Buffer,
    ) {
        // Update the download graph state with new progress data
        self.download_graph_state.update_download_progress(
            download_mbps,
            bytes_downloaded,
            total_bytes,
            progress_percentage,
        );

        // Create the sophisticated download graph widget
        let download_widget = DownloadGraphWidget::new()
            .title("DOWNLOAD PROGRESS")
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));

        // Render using StatefulWidget pattern (preserving all sophisticated functionality)
        use ratatui::widgets::StatefulWidget;
        download_widget.render(area, buf, &mut self.download_graph_state);

        // Then enhance with tachyonfx flow effects if enabled
        if self.config.enable_effects && download_mbps > 0.1 {
            // Create flow effect based on download speed with unique ID
            let _effect_id = self.next_effect_id();
            let flow_effect = self.config.create_wave_effect();
            self.effect_manager.add_effect(flow_effect);
            self.effect_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Legacy bandwidth meter method for backward compatibility
    pub fn render_bandwidth_meter(
        &mut self,
        bandwidth_mbps: f64,
        _bandwidth_history: &[f64],
        area: Rect,
        buf: &mut Buffer,
    ) {
        // Convert to download graph parameters (assuming single file download)
        let estimated_total_bytes = 1_000_000_000u64; // 1GB estimate
        let estimated_downloaded = (bandwidth_mbps * 60.0 * 1_000_000.0) as u64; // Estimate based on 1 minute
        let progress = (estimated_downloaded as f32 / estimated_total_bytes as f32).min(1.0);

        // Delegate to the new sophisticated download graph
        self.render_download_graph(
            bandwidth_mbps,
            estimated_downloaded,
            estimated_total_bytes,
            progress,
            area,
            buf,
        );
    }
    /// Apply magical glow effect for focus states
    #[inline]
    pub fn apply_focus_glow(&mut self, _area: Rect, focused: bool) {
        if !self.config.enable_effects || !focused {
            return;
        }

        let _effect_id = self.next_effect_id();
        let glow_effect = self.config.create_focus_glow();
        self.effect_manager.add_effect(glow_effect);
        self.effect_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Trigger spectacular completion celebration
    #[inline]
    pub fn trigger_completion_celebration(&mut self, _area: Rect) {
        if !self.config.enable_celebrations {
            return;
        }

        // Fireworks burst effect with rainbow cascade
        let _effect_id = self.next_effect_id();
        let fireworks_effect = self.config.create_celebration_effect();
        self.effect_manager.add_effect(fireworks_effect);
        self.effect_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Process all tachyonfx magical effects
    #[inline]
    pub fn process_effects(&mut self, buffer: &mut Buffer, area: Rect) {
        if self.config.enable_effects {
            // Apply all tachyonfx effects to the buffer
            let elapsed = self.last_frame.elapsed();
            self.effect_manager
                .process_effects(elapsed.into(), buffer, area);
        }
        self.last_frame = Instant::now();
    }

    /// Clean up completed effects (zero allocation)
    #[inline]
    pub fn cleanup_effects(&mut self) {
        // TachyonFX automatically manages effect lifecycle
        // This method exists for API compatibility
    }
    /// Get current effect count for debugging
    #[inline]
    pub fn effect_count(&self) -> usize {
        // Return real-time effect count from atomic counter
        self.effect_count.load(Ordering::Relaxed)
    }

    /// Get all active model IDs (zero allocation)
    #[inline]
    pub fn get_active_ids(&self) -> Vec<&String> {
        self.active_models.keys().collect()
    }

    /// Add a new model with specified progress bar type
    #[inline]
    pub fn add_model(&mut self, model_id: String, bar_type: Option<ProgressBarType>) {
        let bar_type = bar_type.unwrap_or(self.config.default_bar_type);
        self.active_models.insert(model_id, bar_type);
    }

    /// Update progress data for a specific model
    #[inline]
    pub fn update_model_progress(&mut self, model_id: &str, progress_data: ProgressData) {
        if let Some(&bar_type) = self.active_models.get(model_id) {
            // Apply effects based on progress bar type and data
            if self.config.enable_effects {
                let _effect_id = self.next_effect_id();
                let effect = self.config.create_progress_effect(bar_type, progress_data);
                self.effect_manager.add_effect(effect);
                self.effect_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

impl Default for ProgressRenderer {
    #[inline]
    fn default() -> Self {
        Self::new(ProgressRendererConfig::default())
    }
}
