//! Column layout management for adaptive model list display.
//!
//! Provides blazing-fast column distribution algorithms with intelligent
//! layout optimization for varying terminal sizes and model counts.

use ratatui::layout::Rect;

use crate::ui::layout::adaptive::AdaptiveLayout;
use progresshub_progress::ImmutableModelProgress;

/// Type alias for column distribution layout
pub type ColumnRects = Vec<Rect>;

/// Type alias for grouped model progress data  
pub type ModelGroups<'a> = Vec<Vec<(usize, &'a ImmutableModelProgress)>>;

/// Type alias for column distribution result
pub type ColumnDistribution<'a> = (ColumnRects, ModelGroups<'a>);

/// Calculate optimal column distribution for adaptive layout.
///
/// This function uses adaptive layout algorithms to determine the optimal
/// number of columns and distributes models across them for maximum
/// visual efficiency and readability.
///
/// # Arguments
/// * `models` - List of model progress data to distribute
/// * `area` - Available screen area for layout
///
/// # Returns
/// Tuple of (column rectangles, grouped models per column)
///
/// # Performance
/// - Zero allocation for empty model lists (fast path)
/// - Efficient ceiling division for model distribution
/// - Bottom-up model ordering for visual hierarchy
/// - Adaptive column calculation based on terminal dimensions
#[inline]
pub fn calculate_column_distribution<'a>(
    models: &'a [ImmutableModelProgress],
    area: Rect,
) -> ColumnDistribution<'a> {
    if models.is_empty() {
        return (vec![area], vec![vec![]]);
    }

    // Use adaptive layout to determine optimal columns
    let widget_count = models.len();
    let adaptive_layout = AdaptiveLayout::new(area.width, area.height, widget_count);
    let (column_chunks, columns, _rows) = adaptive_layout.create_main_layout(area);

    // Distribute models across columns in bottom-up order with their indices
    let models_with_indices: Vec<_> = models.iter().enumerate().rev().collect();
    let models_per_column = if columns > 0 {
        models_with_indices.len().div_ceil(columns) // Ceiling division
    } else {
        models_with_indices.len()
    };

    let mut column_models = Vec::new();
    for col in 0..columns {
        let start_idx = col * models_per_column;
        let end_idx = ((col + 1) * models_per_column).min(models_with_indices.len());
        if start_idx < models_with_indices.len() {
            column_models.push(models_with_indices[start_idx..end_idx].to_vec());
        } else {
            column_models.push(vec![]);
        }
    }

    (column_chunks, column_models)
}
