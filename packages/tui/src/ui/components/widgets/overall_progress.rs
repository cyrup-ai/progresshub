//! Overall progress widget with dynamic color-coded gauge

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Gauge, Widget},
};

use crate::ui::{
    state::AppState,
    theme::{CyrupTheme, Theme},
};
use progresshub_progress::ProgressPercentage;

/// Widget to display overall progress across all downloads
pub struct ProgressGaugeWidget {
    /// Progress percentage (0.0 - 100.0)
    progress_percentage: f64,
    /// Optional label to display on the gauge
    label: Option<String>,
    /// Optional custom block styling
    block: Option<Block<'static>>,
    /// Whether to show percentage text
    show_percentage: bool,
    /// Whether to use enhanced height for bottom bar
    enhanced_height: bool,
}

impl ProgressGaugeWidget {
    /// Create a new overall progress widget
    pub fn new(progress_percentage: f64) -> Self {
        Self {
            progress_percentage: progress_percentage.clamp(0.0, 100.0),
            label: None,
            block: None,
            show_percentage: true,
            enhanced_height: false,
        }
    }

    /// Set a custom label for the gauge
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a custom block for styling
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set whether to show percentage text
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Enable enhanced height for bottom bar usage
    pub fn enhanced_height(mut self, enhanced: bool) -> Self {
        self.enhanced_height = enhanced;
        self
    }

    /// Get the appropriate gauge style based on progress
    fn get_gauge_style(&self) -> ratatui::style::Style {
        let progress_ratio = self.progress_percentage / 100.0;
        CyrupTheme::gauge_style(progress_ratio)
    }

    /// Format the display label
    fn format_label(&self) -> String {
        // Validate progress percentage to prevent corrupted values
        let safe_percentage =
            if self.progress_percentage.is_finite() && self.progress_percentage >= 0.0 {
                self.progress_percentage.min(100.0)
            } else {
                0.0
            };

        let progress = ProgressPercentage {
            value: Some(safe_percentage),
            is_complete: safe_percentage >= 100.0,
            is_indeterminate: false,
        };

        match &self.label {
            Some(custom_label) => {
                if self.show_percentage {
                    // Format as "Label (45.2%)" style
                    let percentage_str = progress.formatted();
                    format!("{custom_label} ({percentage_str})")
                } else {
                    custom_label.clone()
                }
            }
            None => {
                if self.show_percentage {
                    progress.formatted()
                } else {
                    String::new()
                }
            }
        }
    }
}

impl Widget for ProgressGaugeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Extract needed values before consuming self
        let progress_percentage = self.progress_percentage;
        let label = self.format_label();
        let gauge_style = self.get_gauge_style();
        let has_custom_block = self.block.is_some();

        // Calculate the inner area if we have a block
        let render_area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Create the gauge with dynamic styling and progress-based rounded corners
        let gauge = if has_custom_block {
            // If we already have a custom block, don't add another one
            Gauge::default()
                .ratio(progress_percentage / 100.0)
                .label(label)
                .style(gauge_style)
        } else {
            // Add professional rounded corner styling with progress-based border colors
            Gauge::default()
                .ratio(progress_percentage / 100.0)
                .label(label)
                .style(gauge_style)
                .block(create_rounded_block(
                    Borders::ALL,
                    false,
                    Some(progress_percentage),
                ))
        };

        gauge.render(render_area, buf);
    }
}

/// Format bytes into human-readable string using progress crate
///
/// Calculate overall progress from download state using ProgressCalculator formatted methods
///
/// Returns Option to avoid fake formatting in views
pub fn calculate_overall_progress(app_state: &AppState) -> Option<(String, String, f64)> {
    app_state
        .total_progress_formatted()
        .map(|(downloaded_str, total_str, percentage_str)| {
            // Extract percentage value from formatted string like "76.4%"
            let progress_percentage = if percentage_str == "---" {
                0.0
            } else {
                percentage_str
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .min(100.0)
            };
            (downloaded_str, total_str, progress_percentage)
        })
}

/// Create a professional rounded corner block for widget styling with progress-based colors
fn create_rounded_block(borders: Borders, focused: bool, progress: Option<f64>) -> Block<'static> {
    let theme = Theme::default();
    let border_style = if let Some(progress_val) = progress {
        CyrupTheme::progress_border_style(progress_val, focused)
    } else if focused {
        ratatui::style::Style::default().fg(theme.border_focused)
    } else {
        ratatui::style::Style::default().fg(theme.border)
    };

    Block::default()
        .borders(borders)
        .border_style(border_style)
        .style(ratatui::style::Style::default())
}
