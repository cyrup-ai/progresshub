use crate::ui::{
    icons::{FileIcons, ModelIcons, NetworkIcons},
    state::AppState,
    theme::{CyrupTheme, Theme},
};
// Removed formatting function imports - TUI should use ProgressCalculator accessor methods only
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

/// Widget showing remaining models and files to download
pub struct RemainingWidget<'a> {
    app_state: &'a AppState,
}

impl<'a> RemainingWidget<'a> {
    pub fn new(app_state: &'a AppState) -> Self {
        Self { app_state }
    }
}

impl<'a> Widget for RemainingWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (remaining_models, remaining_files) =
            self.app_state
                .progress_state
                .models
                .iter()
                .fold((0, 0), |(models, files), model| {
                    let model_complete = model.is_complete();
                    let incomplete_files = model.files.iter().filter(|f| !f.is_complete()).count();

                    if !model_complete {
                        (models + 1, files + incomplete_files)
                    } else {
                        (models, files + incomplete_files)
                    }
                });

        // Beautiful themed block with rounded corners
        let block = create_rounded_block(Borders::RIGHT, false, None);

        let inner = block.inner(area).inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        block.render(area, buf);

        let theme = Theme::default();

        // Check completion state and show appropriate message
        use crate::ui::state::CompletionState;
        let content = match &self.app_state.completion_state {
            CompletionState::VisualCompletion => {
                // Show immediate completion confirmation
                Line::from(vec![
                    Span::styled("󰄬 ", CyrupTheme::progress_completed()),
                    Span::styled("Download Complete!", CyrupTheme::progress_completed()),
                    Span::styled(
                        " All files downloaded successfully.",
                        ratatui::style::Style::default().fg(theme.text_bright),
                    ),
                ])
            }
            CompletionState::Complete => {
                // Show completion with countdown during 2-second timer
                let remaining_time = if let Some(start_time) = self.app_state.completion_timer {
                    let elapsed = start_time.elapsed().as_secs();
                    2_u64.saturating_sub(elapsed)
                } else {
                    2
                };

                Line::from(vec![
                    Span::styled("󰄬 ", CyrupTheme::progress_completed()),
                    Span::styled("Download Complete!", CyrupTheme::progress_completed()),
                    Span::styled(
                        format!(
                            " Exiting in {} second{}...",
                            remaining_time,
                            if remaining_time == 1 { "" } else { "s" }
                        ),
                        ratatui::style::Style::default().fg(theme.text_dim),
                    ),
                ])
            }
            CompletionState::Exiting => {
                // Show final exit message
                Line::from(vec![
                    Span::styled("󰄬 ", CyrupTheme::progress_completed()),
                    Span::styled("Goodbye!", CyrupTheme::progress_completed()),
                ])
            }
            CompletionState::Active => {
                // Normal operation - show remaining counts
                Line::from(vec![
                    Span::styled(
                        "Remaining: ",
                        ratatui::style::Style::default().fg(theme.text_dim), // Dim for labels
                    ),
                    Span::styled(
                        ModelIcons::HUGGINGFACE,
                        ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                    ),
                    Span::styled(
                        format!(" {remaining_models} Models"),
                        ratatui::style::Style::default()
                            .fg(theme.text_bright)
                            .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                    ),
                    Span::styled(
                        " │ ",
                        ratatui::style::Style::default().fg(theme.border), // Border color for separators
                    ),
                    Span::styled(
                        FileIcons::FILE,
                        ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                    ),
                    Span::styled(
                        format!(" {remaining_files} Files"),
                        ratatui::style::Style::default()
                            .fg(theme.text_bright)
                            .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                    ),
                ])
            }
        };

        Paragraph::new(content)
            .alignment(Alignment::Right)
            .style(CyrupTheme::default_style())
            .render(inner, buf);
    }
}

/// Widget showing overall download progress with enhanced styling
///
/// Uses ProgressCalculator for all progress calculations and formatting
/// to maintain consistency with the architecture.
pub struct OverallProgressWidget<'a> {
    app_state: &'a AppState,
}

impl<'a> OverallProgressWidget<'a> {
    pub fn new(app_state: &'a AppState) -> Self {
        Self { app_state }
    }

    /// Get the progress percentage from the app state
    fn progress_percentage(&self) -> u16 {
        // Use ProgressCalculator high-level formatted method with Option handling
        if let Some((_, _, percentage_str)) = self.app_state.total_progress_formatted() {
            if percentage_str == "---" {
                return 0;
            }
            // Extract percentage value from formatted string like "76.4%"
            percentage_str
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                .min(100.0) as u16
        } else {
            0
        }
    }

    /// Get the formatted progress label using ProgressCalculator
    fn progress_label(&self) -> Option<String> {
        // Use ProgressCalculator high-level formatted method with Option handling - zero fake formatting
        self.app_state
            .total_progress_formatted()
            .map(|(_, _, percentage_str)| percentage_str)
    }
}

impl<'a> Widget for OverallProgressWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let progress_percent = self.progress_percentage();
        let progress_ratio = progress_percent as f64 / 100.0;

        // Enhanced gauge with beautiful rounded corners and progress-based border coloring
        let gauge = Gauge::default()
            .block(create_rounded_block(
                Borders::LEFT | Borders::RIGHT,
                true,
                Some(progress_ratio),
            ))
            .style(CyrupTheme::gauge_style(progress_ratio))
            .percent(progress_percent)
            .label(self.progress_label().unwrap_or_default())
            .use_unicode(true); // Enable beautiful Unicode block characters

        gauge.render(area, buf);
    }
}

/// Widget showing bandwidth monitor with sparkline graph
pub struct BandwidthMonitorWidget {
    current_mbps: f64,
    history: Vec<f64>,
}

impl BandwidthMonitorWidget {
    pub fn new(current_mbps: f64, history: Vec<f64>) -> Self {
        Self {
            current_mbps,
            history,
        }
    }

    fn format_speed(&self) -> String {
        // Convert Mbps to bytes per second for format_speed_display
        let bytes_per_sec = self.current_mbps * 1_000_000.0 / 8.0;
        progresshub_progress::calculator::formatter::format_speed_display(bytes_per_sec)
    }
}

impl Widget for BandwidthMonitorWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Beautiful themed block with rounded corners
        let block = create_rounded_block(Borders::LEFT, false, None);

        let inner = block.inner(area).inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        block.render(area, buf);

        if inner.height < 2 {
            return;
        }

        // Split into speed display and graph with proper spacing
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        let theme = Theme::default();

        // Beautiful speed display with visual text hierarchy
        let speed_line = Line::from(vec![
            Span::styled(
                NetworkIcons::BANDWIDTH,
                ratatui::style::Style::default().fg(theme.accent), // Accent for icon
            ),
            Span::raw(" "),
            Span::styled(
                self.format_speed(),
                CyrupTheme::speed_indicator(self.current_mbps), // Keep dynamic color based on speed
            ),
        ]);

        Paragraph::new(speed_line)
            .alignment(Alignment::Center)
            .style(CyrupTheme::default_style())
            .render(chunks[0], buf);

        // Enhanced sparkline graph with proper scaling and colors
        if chunks[1].height > 0 && !self.history.is_empty() {
            let max_val = self.history.iter().cloned().fold(1.0, f64::max);
            let width = chunks[1].width as usize;

            // Sample the history to fit the available width
            let samples: Vec<f64> = if self.history.len() > width {
                let step = self.history.len() as f64 / width as f64;
                (0..width)
                    .map(|i| {
                        let idx = (i as f64 * step) as usize;
                        self.history.get(idx).copied().unwrap_or(0.0)
                    })
                    .collect()
            } else {
                self.history.clone()
            };

            // Render beautiful sparkline with Unicode blocks
            for (i, &value) in samples.iter().enumerate() {
                if i >= chunks[1].width as usize {
                    break;
                }

                let x = chunks[1].x + i as u16;
                let normalized_height = if max_val > 0.0 { value / max_val } else { 0.0 };
                let height = (normalized_height * chunks[1].height as f64) as u16;

                // Use different Unicode characters for a smoother looking graph
                for y in 0..height {
                    let y_pos = chunks[1].y + chunks[1].height - 1 - y;
                    if let Some(cell) = buf.cell_mut((x, y_pos)) {
                        let symbol = if y == height - 1 && height < chunks[1].height {
                            "▀" // Top half block for smoother edges
                        } else {
                            "█" // Full block for main bars
                        };

                        cell.set_symbol(symbol)
                            .set_style(CyrupTheme::speed_indicator(value));
                    }
                }
            }
        }
    }
}

/// Create a professional rounded corner block for widget styling with optional progress-based colors
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
