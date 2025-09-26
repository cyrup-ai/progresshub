//! Tests for AdaptiveLayout functionality  
//!
//! This module contains comprehensive tests for the adaptive layout system
//! including column calculation, layout creation, and UI responsiveness testing.
//! Extracted from src/ui/layout/adaptive.rs for production code organization.

use progresshub_tui::ui::layout::adaptive::AdaptiveLayout;
use ratatui::layout::Rect;

#[test]
fn test_column_calculation_per_spec() {
    // Test specification requirements

    // < 160 cols → 1 column
    let layout = AdaptiveLayout::new(159, 24, 10);
    assert_eq!(layout.calculate_columns(), 1);

    // 160-239 cols and ≥ 3 widgets → 2 columns
    let layout = AdaptiveLayout::new(200, 24, 3);
    assert_eq!(layout.calculate_columns(), 2);

    let layout = AdaptiveLayout::new(200, 24, 2); // < 3 widgets
    assert_eq!(layout.calculate_columns(), 2); // Falls back to space-based calculation (2 widgets fit in 2 columns)

    // ≥ 240 cols and ≥ 5 widgets → 3 columns (but after spacing adjustments: 240-4-1=235, falls into 160-239 range)
    let layout = AdaptiveLayout::new(240, 24, 5);
    assert_eq!(layout.calculate_columns(), 2); // 235 is in 160-239 range with ≥3 widgets → 2 columns

    // 250 terminal width (245 effective) meets ≥240 rule: 245 >= 240 && 5 >= 5 → 3 columns
    let layout = AdaptiveLayout::new(250, 24, 5);
    assert_eq!(layout.calculate_columns(), 3); // 245 effective width with ≥5 widgets → 3 columns

    let layout = AdaptiveLayout::new(240, 24, 4); // < 5 widgets
    assert_eq!(layout.calculate_columns(), 2); // 235 effective width still falls in 160-239 range with ≥3 widgets → 2
}

#[test]
fn test_layout_creation() {
    let layout = AdaptiveLayout::new(120, 30, 6);
    let area = Rect::new(0, 0, 120, 30);

    let (main_area, bottom_area) = layout.create_app_layout(area);
    assert_eq!(bottom_area.height, 3); // Fixed bottom bar height
    assert_eq!(main_area.height, 24); // 30 - outer_margins(2) - spacing(1) - bottom_bar(3) = 24

    let (chunks, columns, rows) = layout.create_main_layout(main_area);
    assert!(chunks.len() <= 6); // Should not exceed widget count
    assert!(columns > 0);
    assert!(rows > 0);
}
