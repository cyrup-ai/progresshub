use progresshub_tui::ui::effects::download_effects::DownloadEffects;
use progresshub_tui::ui::components::data::download_data::DownloadLevel;
use progresshub_tui::ui::theme::CyrupTheme;
use tachyonfx::Shader;

#[test]
fn test_level_themed_color() {
    // Test all download levels return valid colors
    let low_color = DownloadEffects::level_themed_color(DownloadLevel::Low);
    let medium_color = DownloadEffects::level_themed_color(DownloadLevel::Medium);
    let high_color = DownloadEffects::level_themed_color(DownloadLevel::High);
    let critical_color = DownloadEffects::level_themed_color(DownloadLevel::Critical);

    // Colors should be different for different levels
    assert_ne!(low_color, medium_color);
    assert_ne!(medium_color, high_color);
    assert_ne!(high_color, critical_color);
}

#[test]
fn test_intensity_enhanced_color() {
    let base_color = CyrupTheme::INFO;

    // Test high intensity
    let high_intensity = DownloadEffects::intensity_enhanced_color(base_color, 0.9);
    assert_eq!(high_intensity, CyrupTheme::TEXT_PRIMARY);

    // Test medium intensity
    let medium_intensity = DownloadEffects::intensity_enhanced_color(base_color, 0.7);
    assert_eq!(medium_intensity, base_color);

    // Test low intensity
    let low_intensity = DownloadEffects::intensity_enhanced_color(base_color, 0.3);
    assert_eq!(low_intensity, CyrupTheme::TEXT_MUTED);
}

#[test]
fn test_level_color_params() {
    // Test all levels return valid HSL parameters
    let (h, s, l) = DownloadEffects::level_color_params(DownloadLevel::Low);
    assert!(h >= 0.0 && h <= 360.0);
    assert!(s >= 0.0 && s <= 1.0);
    assert!(l >= 0.0 && l <= 1.0);

    // Test parameters are different for different levels
    let low_params = DownloadEffects::level_color_params(DownloadLevel::Low);
    let high_params = DownloadEffects::level_color_params(DownloadLevel::High);
    assert_ne!(low_params, high_params);
}

#[test]
fn test_effect_creation() {
    // Test effects can be created without panicking
    let gradient = DownloadEffects::create_gradient_effect(DownloadLevel::High, 0.7);
    let pulse = DownloadEffects::create_pulse_effect();
    let transition = DownloadEffects::create_progress_transition(DownloadLevel::Medium);
    let shimmer = DownloadEffects::create_shimmer_effect();
    let error_effect = DownloadEffects::create_error_effect();

    // Effects should be created successfully (basic smoke test)
    let _ = format!("{:?}", gradient.timer());
    let _ = format!("{:?}", pulse.timer());
    let _ = format!("{:?}", transition.timer());
    let _ = format!("{:?}", shimmer.timer());
    let _ = format!("{:?}", error_effect.timer());
}
