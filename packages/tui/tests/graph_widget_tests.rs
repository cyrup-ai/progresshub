use progresshub_tui::ui::components::widgets::graph_widget::DownloadGraphWidget;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

#[test]
fn test_widget_creation() {
    let widget = DownloadGraphWidget::new();
    assert_eq!(widget.get_title(), "DOWNLOAD GRAPH");
    assert!(widget.has_borders());
}

#[test]
fn test_builder_pattern() {
    let widget = DownloadGraphWidget::new()
        .title("Custom Title")
        .borders(false);

    assert_eq!(widget.get_title(), "Custom Title");
    assert!(!widget.has_borders());
}

#[test]
fn test_area_validation() {
    let widget = DownloadGraphWidget::new();

    // Valid area
    let valid_area = Rect::new(0, 0, 80, 10);
    assert!(widget.validate_area(valid_area));

    // Invalid area - too small
    let invalid_area = Rect::new(0, 0, 20, 3);
    assert!(!widget.validate_area(invalid_area));
}

#[test]
fn test_minimum_dimensions() {
    let (min_width, min_height) = DownloadGraphWidget::min_dimensions();
    assert_eq!(min_width, 40);
    assert_eq!(min_height, 6);
}

#[test]
fn test_content_area_calculation() {
    let widget = DownloadGraphWidget::new();
    let area = Rect::new(10, 10, 80, 20);
    let content_area = widget.content_area(area);

    // With borders, content area should be the same as widget area
    assert_eq!(content_area, area);
}

#[test]
fn test_content_area_without_borders() {
    let widget = DownloadGraphWidget::new().borders(false);
    let area = Rect::new(10, 10, 80, 20);
    let content_area = widget.content_area(area);

    // Without borders, content area should be the same
    assert_eq!(content_area, area);
}

#[test]
fn test_border_style_setting() {
    let custom_style = Style::default().fg(Color::Red);
    let widget = DownloadGraphWidget::new().border_style(custom_style);

    assert_eq!(widget.get_border_style().fg, Some(Color::Red));
}
