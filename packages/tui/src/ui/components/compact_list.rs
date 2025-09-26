use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
    Frame,
};

use crate::ui::{
    app::{AppState, DownloadStatus, ModelDownload},
    theme::CyrupTheme,
    icons::IconUtils,
};

/// Render compact layout for model downloads with scrolling support
pub fn render_compact_layout(
    frame: &mut ratatui::Frame,
    area: Rect,
    models: &[&ModelDownload],
    state: &AppState,
) {
    // Create sorted list of models for consistent ordering
    let mut sorted_models = models.to_vec();
    sorted_models.sort_by_key(|m| &m.namespace_model);

    // Calculate viewport height (subtract 2 for borders)
    let viewport_height = area.height.saturating_sub(2) as usize;
    let total_items = sorted_models.len();

    // Ensure scroll offset is within bounds
    let scroll_offset = state
        .scroll_offset
        .min(total_items.saturating_sub(viewport_height));

    let visible_items: Vec<ListItem> = sorted_models
        .iter()
        .skip(scroll_offset)
        .take(viewport_height)
        .map(|model| {
            let progress = model.percentage() / 100.0;  // Convert back to 0-1 range for display

            let status_icon = IconUtils::download_status_icon(&model.overall_progress.status);
            let status_style = match &model.overall_progress.status {
                DownloadStatus::Completed => CyrupTheme::progress_completed(),
                DownloadStatus::Downloading => CyrupTheme::progress_downloading(),
                DownloadStatus::Failed(_) => CyrupTheme::progress_failed(),
                _ => CyrupTheme::progress_pending(),
            };

            // Use model's percentage() method which provides properly formatted percentage
            let percentage = model.percentage();
            let line = format!(
                "{} {:<30} [{:>6.1}%] {:>8.2} MB/s",
                status_icon,
                truncate_filename(&model.namespace_model, 30),  
                percentage,
                model.overall_progress.speed_mbps
            );

            ListItem::new(line).style(status_style)
        })
        .collect();

    let downloads_list =
        List::new(visible_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Model Downloads ({}/{})",
                    scroll_offset + 1,
                    total_items
                ))
                .border_style(CyrupTheme::border_default())
                .title_style(CyrupTheme::block_title())
        );

    frame.render_widget(downloads_list, area);

    // Render scrollbar if needed
    if total_items > viewport_height {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(CyrupTheme::muted_style());

        let mut scrollbar_state = ScrollbarState::new(total_items)
            .position(scroll_offset)
            .viewport_content_length(viewport_height);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin::new(1, 0)),
            &mut scrollbar_state,
        );
    }
}

/// Truncate filename to fit display width
fn truncate_filename(filename: &str, max_len: usize) -> String {
    if filename.len() <= max_len {
        filename.to_string()
    } else {
        format!("{}...", &filename[..max_len.saturating_sub(3)])
    }
}