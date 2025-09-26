//! Column rendering functionality for model list display.
//!
//! Provides blazing-fast column rendering with adaptive height distribution,
//! focus state management, and elegant collapsible widget orchestration.

use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};

use crate::ui::{
    components::core::{Collapsible, CollapsibleState},
    state::{AppState, FocusState},
    theme::CyrupTheme,
};
use progresshub_progress::ImmutableModelProgress;

use super::model_collapsible::create_model_collapsible;

/// Render models within a single column with focus states and adaptive height.
///
/// This function handles the vertical layout of models within a column,
/// calculating optimal height distribution and managing focus states
/// for each model widget.
///
/// # Arguments
/// * `area` - Screen area available for the column
/// * `buf` - Terminal buffer to write into
/// * `state` - Mutable list state for expansion tracking
/// * `models` - List of models with their indices to render
/// * `app_state` - Application state for focus and completion information
///
/// # Performance
/// - Zero allocation height calculations using saturating arithmetic
/// - Blazing-fast height distribution with efficient division algorithms
/// - Intelligent space management to prevent rendering outside bounds
/// - Efficient separator rendering with single-pass buffer operations
#[inline]
pub fn render_column<T>(
    area: Rect,
    buf: &mut Buffer,
    state: &mut T,
    models: &[(usize, &ImmutableModelProgress)],
    app_state: &AppState,
) where
    T: ModelListStateAccess,
{
    if area.height == 0 {
        return;
    }

    let mut y_offset = area.y;
    let available_height = area.height;

    // Calculate base height per model (minimum 3 lines: header + content + separator)
    let base_height_per_model = 3;
    let total_base_height = models.len() as u16 * base_height_per_model;

    // If we have extra space, distribute it among expanded models
    let extra_height = available_height.saturating_sub(total_base_height);

    let expanded_count = models
        .iter()
        .filter(|m| state.is_expanded(&m.1.model_id))
        .count() as u16;
    let extra_per_expanded = if expanded_count > 0 {
        extra_height / expanded_count
    } else {
        0
    };

    for (i, (model_index, model)) in models.iter().enumerate() {
        let is_expanded = state.is_expanded(&model.model_id);
        let focus_state: FocusState = app_state.get_model_focus_state(*model_index);

        // Calculate height for this model
        let model_height = if is_expanded {
            base_height_per_model + extra_per_expanded
        } else {
            2 // Just header + separator when collapsed
        };

        // Make sure we don't exceed available space
        let remaining_height = area.y + area.height - y_offset;
        let actual_height = model_height.min(remaining_height);

        if actual_height == 0 {
            break;
        }

        let model_area = Rect {
            x: area.x,
            y: y_offset,
            width: area.width,
            height: actual_height,
        };

        // Prepare collapsible state for this model
        let mut collapsible_state = CollapsibleState::new(is_expanded);

        // Create the collapsible widget with focus state and completion state
        let collapsible: Collapsible = create_model_collapsible(
            model,
            model_area.height,
            model_area.width,
            focus_state,
            &app_state.completion_state,
        );

        // Render the collapsible
        StatefulWidget::render(collapsible, model_area, buf, &mut collapsible_state);

        y_offset += actual_height;

        // Add separator line between models (except for last one)
        if i < models.len() - 1 && y_offset < area.y + area.height {
            render_model_separator(area, buf, y_offset);
            y_offset += 1;
        }

        if y_offset >= area.y + area.height {
            break;
        }
    }
}

/// Render a separator line between models.
///
/// # Arguments
/// * `area` - Column area for width calculation
/// * `buf` - Terminal buffer to write into  
/// * `y` - Y coordinate for the separator line
///
/// # Performance
/// - Zero allocation separator rendering
/// - Blazing-fast buffer operations in single horizontal pass
/// - Efficient range iteration with inline bounds checking
#[inline]
fn render_model_separator(area: Rect, buf: &mut Buffer, y: u16) {
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(" ");
            cell.set_style(CyrupTheme::default_style());
        }
    }
}

/// Trait for accessing model list state in a generic way.
///
/// This trait allows the column renderer to work with different state types
/// while maintaining zero-cost abstractions and compile-time optimization.
pub trait ModelListStateAccess {
    /// Check if a model is currently expanded
    fn is_expanded(&self, model_id: &str) -> bool;
}
