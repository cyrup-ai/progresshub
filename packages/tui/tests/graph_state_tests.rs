use progresshub_tui::ui::components::widgets::graph_state::DownloadGraphState;
use std::time::Duration;

#[test]
fn test_state_creation() {
    let state = DownloadGraphState::new();
    assert!(state.current_level().is_none());
    assert_eq!(state.session_peak(), 0.0);
    assert!(state.latest_point().is_none());
}

#[test]
fn test_progress_update() {
    let mut state = DownloadGraphState::new();
    state.update_download_progress(10.5, 1024, 2048, 0.5);

    assert!(state.latest_point().is_some());
    if let Some(point) = state.latest_point() {
        assert_eq!(point.speed_mbps, 10.5);
        assert_eq!(point.bytes_downloaded, 1024);
        assert_eq!(point.total_bytes, 2048);
        assert_eq!(point.progress_percentage, 0.5);
    }
}

#[test]
fn test_effect_management() {
    let mut state = DownloadGraphState::new();

    // Add some data to trigger effects
    state.update_download_progress(50.0, 1024, 2048, 0.5);

    // Update effects
    state.update_effects(Duration::from_millis(16));

    // Check that effects are properly managed
    assert!(!state.effects_dirty());
}

#[test]
fn test_frame_interval() {
    let state = DownloadGraphState::new();
    assert_eq!(state.frame_interval(), Duration::from_millis(16));
}

#[test]
fn test_activity_detection() {
    let mut state = DownloadGraphState::new();

    // Initially inactive
    assert!(!state.is_active());

    // Active after adding data
    state.update_download_progress(10.0, 512, 1024, 0.5);
    assert!(state.is_active());
}
