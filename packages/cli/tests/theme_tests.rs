use progresshub_cli::display::theme::{CliColorSpecs, CliStyling, CliTheme, TerminalCapabilities};
use termcolor::Color;

#[test]
fn test_cli_theme_creation() {
    let theme = CliTheme::new();
    let _stdout = theme.stdout();
    let _stderr = theme.stderr();
}

#[test]
fn test_progress_percentage_colors() {
    let low = CliColorSpecs::progress_percentage(15);
    let medium = CliColorSpecs::progress_percentage(50);
    let high = CliColorSpecs::progress_percentage(85);
    let complete = CliColorSpecs::progress_percentage(100);

    // Colors should be different for different progress levels
    assert_ne!(low.fg(), medium.fg());
    assert_ne!(medium.fg(), high.fg());
    assert_ne!(high.fg(), complete.fg());
}

#[test]
fn test_speed_indicator_colors() {
    let slow = CliColorSpecs::speed_indicator(5.0);
    let medium = CliColorSpecs::speed_indicator(25.0);
    let fast = CliColorSpecs::speed_indicator(75.0);
    let blazing = CliColorSpecs::speed_indicator(150.0);

    // Colors should be different for different speed levels
    assert_ne!(slow.fg(), medium.fg());
    assert_ne!(medium.fg(), fast.fg());
    assert_ne!(fast.fg(), blazing.fg());
}

#[test]
fn test_terminal_capabilities() {
    // These tests check capability detection without requiring specific terminal
    let _color_support = TerminalCapabilities::supports_color();
    let _unicode_support = TerminalCapabilities::supports_unicode();
    let _true_color = TerminalCapabilities::supports_true_color();
    let width = TerminalCapabilities::optimal_progress_width();

    // Width should be within reasonable bounds
    assert!((20..=60).contains(&width));
}

#[test]
fn test_gradient_color() {
    let start = Color::Rgb(255, 0, 0); // Red
    let end = Color::Rgb(0, 255, 0); // Green

    let middle = CliStyling::gradient_color(start, end, 0.5);
    if let Color::Rgb(r, g, b) = middle {
        // Middle should be between red and green
        assert!(r > 0 && r < 255);
        assert!(g > 0 && g < 255);
        assert_eq!(b, 0);
    }
}
