use progresshub_tui::ui::components::widgets::graph_renderer::GraphRenderer;

#[test]
fn test_renderer_creation() {
    let renderer = GraphRenderer::new();
    // Renderer should be created successfully
    let _ = format!("{renderer:?}");
}

#[test]
fn test_renderer_default() {
    let renderer = GraphRenderer::default();
    let _ = format!("{renderer:?}");
}

// Note: Full rendering tests would require mocking ratatui Buffer
// and creating test scenarios with actual download data.
// These integration tests would be better placed in an integration test module.
