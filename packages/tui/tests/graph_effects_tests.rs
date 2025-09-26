use progresshub_tui::ui::components::widgets::graph_effects::{
    EffectRegion, GraphAnimationController, RegionEffectType,
};
use ratatui::layout::Rect;
use std::time::Duration;

#[test]
fn test_effect_region_creation() {
    let area = Rect::new(0, 0, 80, 6);
    let region = EffectRegion::new(
        "test".to_string(),
        area,
        0.5,
        RegionEffectType::SparklinePulse,
    );

    assert_eq!(region.region_id, "test");
    assert_eq!(region.intensity, 0.5);
    assert_eq!(region.effect_type, RegionEffectType::SparklinePulse);
}

#[test]
fn test_intensity_clamping() {
    let area = Rect::new(0, 0, 80, 6);

    // Test upper bound clamping
    let region = EffectRegion::new(
        "test".to_string(),
        area,
        2.0, // Above 1.0
        RegionEffectType::SparklinePulse,
    );
    assert_eq!(region.intensity, 1.0);

    // Test lower bound clamping
    let region = EffectRegion::new(
        "test".to_string(),
        area,
        -0.5, // Below 0.0
        RegionEffectType::SparklinePulse,
    );
    assert_eq!(region.intensity, 0.0);
}

#[test]
fn test_sparkline_region_creation() {
    let base_area = Rect::new(0, 0, 80, 6);
    let region = EffectRegion::sparkline_region(base_area, 0.7);

    assert_eq!(region.region_id, "sparkline");
    assert_eq!(region.intensity, 0.7);
    assert_eq!(region.effect_type, RegionEffectType::SparklinePulse);
    assert_eq!(region.area.x, 2); // Skip "│ " prefix
    assert_eq!(region.area.y, 2); // Sparkline row
}

#[test]
fn test_animation_controller() {
    let mut controller = GraphAnimationController::new();
    assert_eq!(controller.frame_count(), 0);

    // Update with frame duration
    controller.update_frame(Duration::from_millis(16));
    assert_eq!(controller.frame_count(), 1);

    // Test animation progress calculation
    let progress = controller.animation_progress();
    assert!((0.0..=1.0).contains(&progress));
}

#[test]
fn test_region_effect_types() {
    use RegionEffectType::*;

    // Test all effect types
    let types = [
        SparklinePulse,
        PeakHighlight,
        FlowAnimation,
        ScaleShimmer,
        HeaderGlow,
    ];

    for effect_type in types {
        let region =
            EffectRegion::new("test".to_string(), Rect::new(0, 0, 10, 1), 0.5, effect_type);
        assert_eq!(region.effect_type, effect_type);
    }
}
