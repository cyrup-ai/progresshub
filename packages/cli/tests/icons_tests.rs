use progresshub_cli::display::icons::{
    CliIcons, CliSpinner, CliStatusIcon, CliTreeRenderer, SpinnerStyle, StatusType,
};
use termcolor::Buffer;

#[test]
#[allow(clippy::len_zero, clippy::const_is_empty)] // Testing const strings - clippy knows they're not empty
fn test_icon_constants() {
    // Test that all icon constants are defined
    assert!(!CliIcons::APP_LOGO.is_empty());
    assert!(!CliIcons::SUCCESS.is_empty());
    assert!(!CliIcons::FILE_COMPLETED.is_empty());
    assert!(!CliIcons::SPEED_BLAZING.is_empty());
}

#[test]
fn test_nerd_font_fallbacks() {
    // Test that Nerd Font functions return valid strings
    assert!(!CliIcons::checkmark().is_empty());
    assert!(!CliIcons::download().is_empty());
    assert!(!CliIcons::folder().is_empty());
    assert!(!CliIcons::error().is_empty());
}

#[test]
fn test_spinner_creation() {
    let spinner = CliSpinner::new()
        .style(SpinnerStyle::Dots)
        .speed(2.0)
        .active(true);

    assert_eq!(spinner.get_style(), SpinnerStyle::Dots);
    assert_eq!(spinner.get_speed(), 2.0);
    assert!(spinner.get_active());

    // Test that current_frame returns valid string
    assert!(!spinner.current_frame().is_empty());
}

#[test]
fn test_spinner_rendering() {
    let spinner = CliSpinner::new();
    let mut buffer = Buffer::no_color();

    spinner
        .render(&mut buffer)
        .unwrap_or_else(|_| panic!("Rendering should succeed"));

    let output = String::from_utf8_lossy(buffer.as_slice());
    assert!(!output.is_empty());
}

#[test]
fn test_status_icon() {
    let completed = CliStatusIcon::new(StatusType::Completed);
    let downloading = CliStatusIcon::new(StatusType::Downloading).animated(false);
    let custom = CliStatusIcon::new(StatusType::Info).custom_icon("🔥");

    let mut buffer = Buffer::no_color();

    completed
        .render(&mut buffer)
        .unwrap_or_else(|_| panic!("Should render"));
    downloading
        .render(&mut buffer)
        .unwrap_or_else(|_| panic!("Should render"));
    custom
        .render(&mut buffer)
        .unwrap_or_else(|_| panic!("Should render"));
}

#[test]
fn test_tree_renderer() {
    let renderer = CliTreeRenderer::new()
        .indent_level(2)
        .is_last(true)
        .unicode(true);

    assert_eq!(renderer.get_indent_level(), 2);
    assert!(renderer.get_is_last());
    assert!(renderer.get_unicode());

    let mut buffer = Buffer::no_color();
    renderer
        .render_prefix(&mut buffer)
        .unwrap_or_else(|_| panic!("Should render"));
}

#[test]
fn test_spinner_styles() {
    for style in [
        SpinnerStyle::Dots,
        SpinnerStyle::Blocks,
        SpinnerStyle::Braille,
        SpinnerStyle::Line,
        SpinnerStyle::Bounce,
    ] {
        let spinner = CliSpinner::new().style(style);
        let frames = spinner.get_frames();
        assert!(!frames.is_empty());
        assert!(frames.iter().all(|frame| !frame.is_empty()));
    }
}
