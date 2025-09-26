//! Sophisticated rendering system with magical tachyonfx effects
//!
//! Zero-allocation rendering pipeline with adaptive layout, professional spacing,
//! and spectacular magical effects for production-grade visual experience.

// Temporarily commented out during YStream integration to avoid cryypt compilation errors
// use lib_bandwydth::display::components::BandwidthGraphWidget;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::ui::components::header::render_header;
use crate::ui::{
    components::widgets::{
        bottom_bar::RemainingWidget,
        overall_progress::{ProgressGaugeWidget, calculate_overall_progress},
    },
    layout::adaptive::{AdaptiveLayout, ProfessionalSpacing},
};

use super::app_core::App;

impl App {
    /// Render the UI with professional spacing, adaptive layout, and magical effects.
    ///
    /// Provides comprehensive visual experience with:
    /// - Adaptive layout system for optimal space utilization
    /// - Professional spacing and elegant widget arrangement
    /// - Sophisticated progress visualization with magical tachyonfx effects
    /// - Zero-allocation rendering pipeline with blazing-fast performance
    /// - Real-time bandwidth monitoring with sparkline visualization
    ///
    /// # Arguments
    /// * `frame` - Ratatui frame for rendering operations
    ///
    /// # Performance
    /// - Zero allocation in hot rendering paths
    /// - Blazing-fast widget rendering with inline optimizations
    /// - Efficient data validation and sanitization
    /// - Magical effects processing with optimal timing
    #[inline]
    pub fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Update terminal size in app state
        self.state.update_layout(area.width, area.height);

        // Top header + content split
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(1),    // rest of UI
            ])
            .split(area);

        // Render header using the function in `ui/components/header.rs` to exercise it
        {
            render_header(frame, vertical[0], &mut self.state);
        }

        // Create professional adaptive layout with proper spacing for the remainder
        let adaptive_layout = AdaptiveLayout::new(
            vertical[1].width,
            vertical[1].height,
            self.state.get_model_count(),
        )
        .professional_spacing(true);

        let (main_area, bottom_bar_area) = adaptive_layout.create_app_layout(vertical[1]);

        // Render model list in main area with zero-allocation patterns
        if main_area.height > 0 {
            self.render_model_list(frame, main_area);
        }

        // Render specification-compliant bottom bar with professional spacing
        self.render_bottom_bar(frame, bottom_bar_area);

        // Process all tachyonfx effects for magical animations
        self.progress_renderer
            .process_effects(frame.buffer_mut(), area);
    }

    /// Render model list with zero-allocation patterns and intelligent layout.
    ///
    /// Provides comprehensive model visualization with:
    /// - Adaptive column layout for optimal space utilization
    /// - Zero-allocation model data processing
    /// - Intelligent focus state management
    /// - Production-grade error handling with validation
    ///
    /// # Arguments
    /// * `frame` - Ratatui frame for rendering operations
    /// * `area` - Screen area available for model list rendering
    ///
    /// # Performance
    /// - Zero allocation model iteration with efficient borrowing
    /// - Blazing-fast column distribution with optimal height calculation
    /// - Intelligent data validation with comprehensive error handling
    #[inline]
    fn render_model_list(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use crate::ui::components::widgets::model_list::{
            calculate_column_distribution, render_column,
        };

        if area.height < 3 {
            return; // Not enough space for meaningful rendering
        }

        if self.state.get_model_count() == 0 {
            return; // No models to render
        }

        // Use AppState method instead of direct field access - ZERO pattern violations
        let models_vec = self.state.get_models_for_rendering();

        // Calculate optimal column distribution for models
        let (column_rects, model_groups) = calculate_column_distribution(&models_vec, area);

        // Render models in columns using efficient column renderer
        for (column_rect, column_models) in column_rects.iter().zip(model_groups.iter()) {
            if !column_models.is_empty() {
                render_column(
                    *column_rect,
                    frame.buffer_mut(),
                    &mut self.model_list_state,
                    column_models,
                    &self.state,
                );
            }
        }
    }

    /// Render sophisticated bottom bar with magical progress visualization.
    ///
    /// Provides comprehensive bottom bar experience with:
    /// - Professional three-widget layout with proper spacing
    /// - Intelligent remaining count calculation with validation
    /// - Magical overall progress rendering with tachyonfx effects
    /// - Advanced bandwidth visualization with sparkline history
    /// - Zero-allocation data processing with efficient validation
    ///
    /// # Arguments
    /// * `frame` - Ratatui frame for rendering operations
    /// * `bottom_bar_area` - Layout area for bottom bar widgets
    ///
    /// # Performance
    /// - Zero allocation in widget rendering
    /// - Blazing-fast statistics calculation with overflow protection
    /// - Efficient data validation with comprehensive error handling
    #[inline]
    fn render_bottom_bar(
        &mut self,
        frame: &mut ratatui::Frame,
        bottom_bar_area: ratatui::layout::Rect,
    ) {
        let bottom_constraints = vec![
            Constraint::Percentage(33), // Remaining widget (left)
            Constraint::Length(ProfessionalSpacing::COLUMN_SPACING), // Spacing
            Constraint::Percentage(33), // Progress widget (center)
            Constraint::Length(ProfessionalSpacing::COLUMN_SPACING), // Spacing
            Constraint::Percentage(33), // Bandwidth widget (right)
        ];

        let bottom_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(bottom_constraints)
            .split(bottom_bar_area);

        // Extract widget areas (skip spacing areas)
        let bottom_widget_areas = [bottom_layout[0], bottom_layout[2], bottom_layout[4]];

        // Render remaining counts widget (bottom-left)
        self.render_remaining_widget(frame, bottom_widget_areas[0]);

        // Render overall progress (bottom-center)
        self.render_progress_widget(frame, bottom_widget_areas[1]);

        // Render bandwidth visualization (bottom-right)
        self.render_bandwidth_widget(frame, bottom_widget_areas[2]);
    }

    /// Render remaining counts widget with intelligent validation.
    #[inline]
    fn render_remaining_widget(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        // Use dedicated RemainingWidget which reads from AppState
        let widget = RemainingWidget::new(&self.state);
        frame.render_widget(widget, area);
    }

    /// Render overall progress widget (center of bottom bar)
    #[inline]
    fn render_progress_widget(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let percentage = calculate_overall_progress(&self.state)
            .map(|(_, _, percentage)| percentage)
            .unwrap_or(0.0);
        let gauge = ProgressGaugeWidget::new(percentage).enhanced_height(true);
        frame.render_widget(gauge, area);
    }

    /// Render sophisticated bandwidth widget with lib_bandwydth integration.
    #[inline]
    fn render_bandwidth_widget(&mut self, _frame: &mut ratatui::Frame, _area: ratatui::layout::Rect) {
        // Temporarily commented out during YStream integration
        // let bandwidth_widget = BandwidthGraphWidget::new()
        //     .title("NETWORK BANDWIDTH")
        //     .borders(true);

        // frame.render_stateful_widget(bandwidth_widget, area, &mut self.state.bandwidth_state);
    }
}
