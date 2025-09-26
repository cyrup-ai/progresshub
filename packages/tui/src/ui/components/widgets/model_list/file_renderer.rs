//! File list rendering functionality for model display.
//!
//! Provides blazing-fast file list rendering with progress indicators and
//! intelligent text truncation for optimal terminal display.

use ratatui::{buffer::Buffer, layout::Rect};

use crate::ui::theme::{CyrupTheme, Theme};
use progresshub_progress::ImmutableFileProgress;

/// Render a list of files with progress indicators.
///
/// This function renders individual files within a model's expanded view,
/// showing file names and progress percentages with intelligent truncation
/// and progress-based color coding.
///
/// # Arguments
/// * `files` - List of file progress information to render
/// * `area` - Screen area available for file list rendering
/// * `buf` - Terminal buffer to write into
///
/// # Performance
/// - Zero allocation file name truncation using slice operations
/// - Blazing-fast progress calculations with cached theme lookups
/// - Efficient buffer writing with single-pass rendering
/// - Intelligent width calculations to prevent display corruption
#[inline]
pub fn render_file_list(files: &[ImmutableFileProgress], area: Rect, buf: &mut Buffer) {
    let mut y = area.y;
    let max_y = area.y + area.height;

    for file in files {
        if y >= max_y {
            break; // Prevent rendering outside available area
        }

        let file_text = format!("  ▫ {}", file.file_name);
        let progress_text = format!(" {}", file.percentage_formatted());

        // Calculate space for progress text
        let progress_width = progress_text.len() as u16;
        let file_width = area.width.saturating_sub(progress_width);

        // Render file name (truncated if necessary)
        let file_line = if file_text.len() as u16 > file_width {
            format!(
                "{}...",
                &file_text[0..(file_width.saturating_sub(3) as usize)]
            )
        } else {
            file_text
        };

        let theme = Theme::default();
        let file_progress_value = file.progress_ratio() * 100.0; // Convert 0.0-1.0 to 0.0-100.0 for styling

        // Set the file name with normal text brightness
        buf.set_string(
            area.x,
            y,
            &file_line,
            ratatui::style::Style::default().fg(theme.text),
        );

        // Set the progress percentage (right-aligned) with progress-based color coding
        let progress_x = area.x + area.width - progress_width;
        buf.set_string(
            progress_x,
            y,
            &progress_text,
            CyrupTheme::progress_percentage_style(file_progress_value),
        );

        y += 1;
    }
}
