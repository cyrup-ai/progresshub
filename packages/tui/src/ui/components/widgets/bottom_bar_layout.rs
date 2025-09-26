//! Bottom bar layout widget that combines Remaining, Progress, and Bandwidth widgets

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Widget},
};

use super::{
    bandwidth::BandwidthWidget,
    overall_progress::{OverallProgressWidget, calculate_overall_progress},
    remaining::{CounterWidget, calculate_remaining_counts},
};
use crate::ui::{state::AppState, theme::{CyrupTheme, Theme}};

/// Widget that renders the complete bottom bar with all three components
pub struct BottomBarLayout<'a> {
    /// Reference to the app state
    app_state: &'a AppState,
    /// Bandwidth history for sparkline
    bandwidth_history: Vec<u64>,
    /// Whether to show detailed information
    show_detailed: bool,
    /// Whether to use enhanced styling
    enhanced_styling: bool,
}

impl<'a> BottomBarLayout<'a> {
    /// Create a new bottom bar layout
    pub fn new(app_state: &'a AppState) -> Self {
        Self {
            app_state,
            bandwidth_history: Vec::new(),
            show_detailed: true,
            enhanced_styling: true,
        }
    }

    /// Set bandwidth history for sparkline
    pub fn bandwidth_history(mut self, history: Vec<u64>) -> Self {
        self.bandwidth_history = history;
        self
    }

    /// Set whether to show detailed information
    pub fn show_detailed(mut self, show: bool) -> Self {
        self.show_detailed = show;
        self
    }

    /// Set whether to use enhanced styling
    pub fn enhanced_styling(mut self, enhanced: bool) -> Self {
        self.enhanced_styling = enhanced;
        self
    }
}

impl<'a> Widget for BottomBarLayout<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Split the bottom bar into three equal sections with padding
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33), // Remaining widget (left)
                Constraint::Percentage(34), // Progress widget (center)
                Constraint::Percentage(33), // Bandwidth widget (right)
            ])
            .split(area);

        // Calculate data for each widget
        let (remaining_models, remaining_files) = calculate_remaining_counts(&self.app_state.downloads);
        let (total_downloaded, total_expected, progress_percentage) = calculate_overall_progress(&self.app_state.downloads);
        let bandwidth_stats = &self.app_state.bandwidth_stats;
        let current_speed = bandwidth_stats.current_speed;

        // Create and render remaining widget (left)
        let remaining_widget = CounterWidget::new(remaining_models, remaining_files)
            .show_detailed(self.show_detailed);

        if self.enhanced_styling {
            let remaining_block = create_progress_block(None);
            remaining_widget.block(remaining_block).render(chunks[0], buf);
            render_shadow(chunks[0], buf, create_shadow_style());
        } else {
            remaining_widget.render(chunks[0], buf);
        }

        // Create and render overall progress widget (center)
        // ARCHITECTURAL FIX: Use progress crate formatting instead of manual formatting
        // This maintains the Pure Flume Channel Event-Driven Architecture constraint
        let progress_label = if total_expected > 0 {
            // Using progress crate formatting function to maintain consistency
            use progresshub_progress::calculator::format_bytes;
            format!(
                "{} / {}",
                format_bytes(total_downloaded),
                format_bytes(total_expected)
            )
        } else {
            "No downloads".to_string()
        };

        let progress_widget = OverallProgressWidget::new(progress_percentage)
            .label(progress_label)
            .enhanced_height(true)
            .show_percentage(true);

        if self.enhanced_styling {
            let progress_block = create_progress_block(Some(progress_percentage));
            progress_widget.block(progress_block).render(chunks[1], buf);
            render_shadow(chunks[1], buf, create_shadow_style());
        } else {
            progress_widget.render(chunks[1], buf);
        }

        // Create and render bandwidth widget (right)
        let bandwidth_widget = if !self.bandwidth_history.is_empty() {
            BandwidthWidget::with_history(current_speed, self.bandwidth_history)
                .show_sparkline(true)
                .show_units(true)
        } else {
            BandwidthWidget::new(current_speed)
                .show_units(true)
        };

        if self.enhanced_styling {
            let bandwidth_block = create_progress_block(None);
            bandwidth_widget.block(bandwidth_block).render(chunks[2], buf);
            render_shadow(chunks[2], buf, create_shadow_style());
        } else {
            bandwidth_widget.render(chunks[2], buf);
        }
    }
}

// REMOVED: format_bytes_compact function - ARCHITECTURAL VIOLATION
// All byte formatting must use ProgressCalculator::bytes_formatted() 
// This maintains the Pure Flume Channel Event-Driven Architecture constraint:
// "ZERO FORMATTING LOGIC IN DISPLAYS - Progress crate ONLY location for any calculations or formatting"

/// Render a professional shadow effect using Unicode block characters
fn render_shadow(area: Rect, buf: &mut Buffer, shadow_style: ratatui::style::Style) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    
    // Right shadow (vertical bar)
    for y in area.y + 1..area.y + area.height {
        if let Some(cell) = buf.cell_mut((area.x + area.width, y)) {
            cell.set_symbol("▐").set_style(shadow_style);
        }
    }
    
    // Bottom shadow (horizontal bar)
    for x in area.x + 1..area.x + area.width + 1 {
        if let Some(cell) = buf.cell_mut((x, area.y + area.height)) {
            cell.set_symbol("▄").set_style(shadow_style);
        }
    }
}

/// Create a shadow style with appropriate transparency and color
fn create_shadow_style() -> ratatui::style::Style {
    let theme = Theme::default();
    ratatui::style::Style::default()
        .fg(theme.border)
}

/// Create a professional block with progress-based styling
fn create_progress_block(progress: Option<f64>) -> Block<'static> {
    let theme = Theme::default();
    let border_style = if let Some(progress_val) = progress {
        if progress_val >= 100.0 {
            ratatui::style::Style::default().fg(theme.success)
        } else if progress_val >= 70.0 {
            ratatui::style::Style::default().fg(theme.accent)
        } else if progress_val >= 30.0 {
            ratatui::style::Style::default().fg(theme.warning)
        } else {
            ratatui::style::Style::default().fg(theme.error)
        }
    } else {
        ratatui::style::Style::default().fg(theme.border)
    };
    
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default())
}