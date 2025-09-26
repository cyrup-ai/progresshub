// Temporarily commented out during YStream integration to avoid cryypt compilation errors
// use lib_bandwydth::display::components::BandwidthGraphWidget;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::{
    icons::{ModelIcons, UIIcons},
    state::AppState,
    theme::CyrupTheme,
};

/// Render application header with title, download count, and bandwidth information
#[inline]
pub fn render_header(frame: &mut ratatui::Frame, area: Rect, state: &mut AppState) {
    let active_count = state
        .progress_state
        .models
        .iter()
        .filter(|model| !model.is_complete())
        .count();

    // Split header area: main title on left, bandwidth widget on right
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    let mut header_text = format!(
        "{} ProgressHub - {} active downloads",
        ModelIcons::HUGGINGFACE,
        active_count
    );

    // Add scroll indicator if we have many models
    let total_models = state
        .progress_state
        .models
        .iter()
        .filter(|model| !model.is_complete())
        .count();
    if total_models > 10 {
        header_text.push_str(&format!(" {} Use ↑↓ to scroll", UIIcons::INFO));
    }

    let header = Paragraph::new(header_text)
        .style(CyrupTheme::header_title())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(CyrupTheme::border_default()),
        );

    frame.render_widget(header, header_chunks[0]);

    // Real bandwidth monitoring using lib_bandwydth - temporarily commented out
    // let bandwidth_widget = BandwidthGraphWidget::new()
    //     .title("BANDWIDTH")
    //     .border_style(CyrupTheme::border_default())
    //     .borders(false);

    // frame.render_stateful_widget(
    //     bandwidth_widget,
    //     header_chunks[1],
    //     &mut state.bandwidth_state,
    // );
}
