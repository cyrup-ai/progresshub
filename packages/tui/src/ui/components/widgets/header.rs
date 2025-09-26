use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::{ui::icons::ModelIcons, ui::theme::CyrupTheme};

/// The header widget displays the application title
pub struct HeaderWidget<'a> {
    /// Application title
    title: &'a str,
    /// Optional subtitle or version information
    subtitle: Option<&'a str>,
}

impl<'a> HeaderWidget<'a> {
    /// Create a new header widget with the given title
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            subtitle: None,
        }
    }

    /// Add a subtitle to the header
    pub fn with_subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }
}

impl<'a> Widget for HeaderWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Single-line header with app name only
        let hf_icon = ModelIcons::HUGGINGFACE;
        let title_spans = vec![
            Span::styled(
                format!("{hf_icon} "),
                CyrupTheme::header_title().fg(CyrupTheme::WARNING),
            ),
            Span::styled(self.title, CyrupTheme::header_title()),
        ];

        let title_line = Line::from(title_spans);
        let header_text = Paragraph::new(title_line)
            .alignment(Alignment::Center)
            .style(CyrupTheme::default_style());

        header_text.render(area, buf);
    }
}
