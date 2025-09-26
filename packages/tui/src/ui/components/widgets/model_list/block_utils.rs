//! Block creation and styling utilities for model list display.
//!
//! Provides blazing-fast block creation with progress-based styling and
//! professional rounded corner styling for optimal visual hierarchy.

use ratatui::widgets::{Block, Borders};

use crate::ui::theme::{CyrupTheme, Theme};

/// Create a professional rounded corner block for widget styling with progress-based colors.
///
/// This function creates styled blocks with dynamic border colors based on progress state
/// and focus state, providing clear visual hierarchy and user feedback.
///
/// # Arguments
/// * `title` - Optional title text for the block
/// * `focused` - Whether this block should use focus styling
/// * `progress` - Optional progress value (0.0-1.0) for progress-based coloring
///
/// # Returns
/// Styled block with appropriate borders and colors
///
/// # Performance
/// - Zero allocation after title conversion
/// - Blazing-fast style calculations with efficient theme lookups
/// - Inline operations for optimal performance
#[inline]
pub fn create_rounded_block(
    title: Option<&str>,
    focused: bool,
    progress: Option<f64>,
) -> Block<'static> {
    let theme = Theme::default();
    let border_style = if let Some(progress_val) = progress {
        CyrupTheme::progress_border_style(progress_val, focused)
    } else if focused {
        ratatui::style::Style::default().fg(theme.border_focused)
    } else {
        ratatui::style::Style::default().fg(theme.border)
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default());

    if let Some(title_text) = title {
        block = block.title(title_text.to_string());
    }

    block
}
