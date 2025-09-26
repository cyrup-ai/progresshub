//! Remaining widget showing count of incomplete models and files

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use crate::ui::{icons::DownloadIcons, state::AppState, theme::Theme};

/// Widget to display remaining model and file counts
pub struct CounterWidget {
    /// Number of models not yet at 100% completion
    remaining_models: usize,
    /// Number of files not yet at 100% completion  
    remaining_files: usize,
    /// Optional custom block styling
    block: Option<Block<'static>>,
    /// Whether to show detailed breakdown
    show_detailed: bool,
}

impl CounterWidget {
    /// Create a new remaining widget with the given counts
    pub fn new(remaining_models: usize, remaining_files: usize) -> Self {
        Self {
            remaining_models,
            remaining_files,
            block: None,
            show_detailed: true,
        }
    }

    /// Set a custom block for styling
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set whether to show detailed breakdown
    pub fn show_detailed(mut self, show: bool) -> Self {
        self.show_detailed = show;
        self
    }
}

impl Widget for CounterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate the inner area if we have a block
        let render_area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        let theme = Theme::default();

        // Create styled text spans with visual hierarchy
        let remaining_spans = if self.show_detailed {
            vec![
                Span::styled(
                    "Remaining: ",
                    ratatui::style::Style::default().fg(theme.text_dim), // Dim for labels
                ),
                Span::styled(
                    format!("{} ", self.remaining_models),
                    ratatui::style::Style::default()
                        .fg(theme.text_bright)
                        .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                ),
                Span::styled(
                    format!("{} ", DownloadIcons::DOWNLOADING),
                    ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                ),
                Span::styled(
                    "Models | ",
                    ratatui::style::Style::default().fg(theme.text), // Normal for descriptive text
                ),
                Span::styled(
                    format!("{} ", self.remaining_files),
                    ratatui::style::Style::default()
                        .fg(theme.text_bright)
                        .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                ),
                Span::styled(
                    "▫ ",                                              // Professional file symbol
                    ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                ),
                Span::styled(
                    "Files",
                    ratatui::style::Style::default().fg(theme.text), // Normal for descriptive text
                ),
            ]
        } else {
            vec![
                Span::styled(
                    format!("{} ", DownloadIcons::DOWNLOADING),
                    ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                ),
                Span::styled(
                    format!("{}", self.remaining_models),
                    ratatui::style::Style::default()
                        .fg(theme.text_bright)
                        .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                ),
                Span::styled(
                    " | ",
                    ratatui::style::Style::default().fg(theme.text_dim), // Dim for separators
                ),
                Span::styled(
                    "▫ ",                                              // Professional file symbol
                    ratatui::style::Style::default().fg(theme.accent), // Accent for icons
                ),
                Span::styled(
                    format!("{}", self.remaining_files),
                    ratatui::style::Style::default()
                        .fg(theme.text_bright)
                        .add_modifier(ratatui::style::Modifier::BOLD), // Bright for values
                ),
            ]
        };

        // Create paragraph with right alignment for proper spacing
        let remaining_paragraph =
            Paragraph::new(Line::from(remaining_spans)).alignment(Alignment::Right);

        remaining_paragraph.render(render_area, buf);
    }
}

/// Calculate remaining model and file counts from ProgressCalculator state
///
/// Returns (remaining_models, remaining_files) where:
/// - remaining_models: Number of models not yet at 100% completion
/// - remaining_files: Number of files not yet at 100% completion
pub fn calculate_remaining_counts(app_state: &AppState) -> (usize, usize) {
    let remaining_models = app_state
        .progress_state
        .models
        .iter()
        .filter(|model| !model.is_complete())
        .count();

    let remaining_files = app_state
        .progress_state
        .models
        .iter()
        .map(|model| model.files.iter().filter(|f| !f.is_complete()).count())
        .sum::<usize>();

    (remaining_models, remaining_files)
}
