//! Data structures and types for download statistics analysis.
//!
//! Contains all the core data structures used for statistical analysis of download progress,
//! including component state, file information, insights, and display optimization structures.

use ratatui::widgets::Widget;

/// Component for displaying comprehensive download statistics and analysis
#[derive(Debug)]
pub struct DownloadStatsComponent {
    /// Current ProgressCalculator snapshot - THE SOURCE OF TRUTH
    pub(super) current_progress_calculator: Option<progresshub_progress::ProgressCalculator>,
    /// Current bandwidth from bandwidth events
    pub(super) current_bandwidth_mbps: f64,
}

impl DownloadStatsComponent {
    /// Create a new download statistics component with optimized initialization
    #[inline]
    pub fn new() -> Self {
        Self {
            current_progress_calculator: None,
            current_bandwidth_mbps: 0.0,
        }
    }

    /// Update bandwidth from bandwidth event stream - PURE EVENT-DRIVEN
    #[inline]
    pub fn update_bandwidth(&mut self, bandwidth_bytes_per_sec: f64) {
        self.current_bandwidth_mbps = bandwidth_bytes_per_sec / 1_048_576.0; // Convert bytes/s to MB/s
    }
}

impl Default for DownloadStatsComponent {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Widget implementation for DownloadStatsComponent - PURE EVENT-DRIVEN
/// Consumes ONLY bandwidth and ProgressCalculator event streams via flume channels
impl Widget for &DownloadStatsComponent {
    #[inline]
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Get formatted strings directly from ProgressCalculator event stream
        let (bytes_text, percentage_text, speed_text) =
            if let Some(ref progress_calc) = self.current_progress_calculator {
                (
                    progress_calc.bytes_formatted(),
                    progress_calc.percentage_formatted(),
                    progress_calc.speed_formatted(),
                )
            } else {
                (
                    "Waiting...".to_string(),
                    "0%".to_string(),
                    "N/A".to_string(),
                )
            };
        let bandwidth_text = format!("{:.1} Mbps", self.current_bandwidth_mbps);

        // Create content lines with zero allocation where possible
        let content = vec![
            Line::from(vec![
                Span::styled("Progress: ", Style::default().fg(Color::Cyan)),
                Span::styled(percentage_text, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Data: ", Style::default().fg(Color::Cyan)),
                Span::styled(bytes_text, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Speed: ", Style::default().fg(Color::Cyan)),
                Span::styled(speed_text, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Bandwidth: ", Style::default().fg(Color::Cyan)),
                Span::styled(bandwidth_text, Style::default().fg(Color::Magenta)),
            ]),
        ];

        // Render with blazing-fast paragraph widget
        let widget = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Download Stats")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White));

        widget.render(area, buf);
    }
}
