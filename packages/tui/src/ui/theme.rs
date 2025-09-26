use ratatui::style::{Color, Modifier, Style};

/// Professional theme matching modern terminal UI standards
#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_bright: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_bright: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            surface_bright: Color::Rgb(50, 55, 65),
            border: Color::Rgb(60, 65, 75),
            border_focused: Color::Rgb(120, 130, 145),
            text: Color::Rgb(200, 205, 215),
            text_dim: Color::Rgb(140, 145, 155),
            text_bright: Color::Rgb(240, 245, 255),
            accent: Color::Rgb(130, 170, 255),
            accent_dim: Color::Rgb(80, 120, 200),
            success: Color::Rgb(130, 220, 150),
            warning: Color::Rgb(250, 200, 100),
            error: Color::Rgb(250, 120, 120),
        }
    }
}

/// Rounded corner characters for modern look
pub struct RoundedCorners;

impl RoundedCorners {
    pub const TOP_LEFT: &'static str = "╭";
    pub const TOP_RIGHT: &'static str = "╮";
    pub const BOTTOM_LEFT: &'static str = "╰";
    pub const BOTTOM_RIGHT: &'static str = "╯";
    pub const HORIZONTAL: &'static str = "─";
    pub const VERTICAL: &'static str = "│";
}

/// Compatibility wrapper to maintain existing API
pub struct CyrupTheme;

impl CyrupTheme {
    // Core Color Palette - transparent background
    pub const BG_PRIMARY: Color = Color::Reset;
    pub const BG_SECONDARY: Color = Color::Reset;
    pub const TEXT_PRIMARY: Color = Color::Rgb(240, 245, 255);
    pub const TEXT_SECONDARY: Color = Color::Rgb(200, 205, 215);
    pub const TEXT_MUTED: Color = Color::Rgb(140, 145, 155);
    pub const BORDER: Color = Color::Rgb(60, 65, 75);
    pub const BORDER_FOCUSED: Color = Color::Rgb(120, 130, 145);

    // Professional Color Pops - replaced hideous purple
    pub const ACCENT: Color = Color::Rgb(130, 170, 255);
    pub const SUCCESS: Color = Color::Rgb(130, 220, 150);
    pub const WARNING: Color = Color::Rgb(250, 200, 100);
    pub const ERROR: Color = Color::Rgb(250, 120, 120);
    pub const INFO: Color = Color::Rgb(130, 170, 255);

    // Base Styles
    pub fn default_style() -> Style {
        Style::new().fg(Self::TEXT_PRIMARY)
    }

    pub fn primary_text() -> Style {
        Style::new().fg(Self::TEXT_PRIMARY)
    }

    pub fn secondary_style() -> Style {
        Style::new().fg(Self::TEXT_SECONDARY)
    }

    pub fn muted_style() -> Style {
        Style::new().fg(Self::TEXT_MUTED)
    }

    pub fn selected_style() -> Style {
        Style::new().fg(Self::ACCENT).add_modifier(Modifier::BOLD)
    }

    // Borders
    pub fn border_default() -> Style {
        Style::new().fg(Self::BORDER)
    }

    pub fn border_focused() -> Style {
        Style::new().fg(Self::BORDER_FOCUSED)
    }

    pub fn border_selected() -> Style {
        Style::new().fg(Self::ACCENT)
    }

    // Headers and Titles
    pub fn header_title() -> Style {
        Style::new()
            .fg(Self::TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn block_title() -> Style {
        Style::new()
            .fg(Self::TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    // Progress and Status
    pub fn progress_pending() -> Style {
        Style::new().fg(Self::TEXT_MUTED)
    }

    pub fn progress_downloading() -> Style {
        Style::new().fg(Self::INFO).add_modifier(Modifier::BOLD)
    }

    pub fn progress_completed() -> Style {
        Style::new().fg(Self::SUCCESS).add_modifier(Modifier::BOLD)
    }

    pub fn progress_failed() -> Style {
        Style::new().fg(Self::ERROR).add_modifier(Modifier::BOLD)
    }

    // Files
    pub fn file_complete() -> Style {
        Style::new().fg(Self::SUCCESS)
    }

    pub fn file_downloading() -> Style {
        Style::new().fg(Self::TEXT_SECONDARY)
    }

    pub fn file_pending() -> Style {
        Style::new().fg(Self::TEXT_MUTED)
    }

    // Bandwidth (dynamic based on speed)
    pub fn speed_indicator(mbps: f64) -> Style {
        match mbps {
            x if x < 10.0 => Style::new().fg(Self::ERROR).add_modifier(Modifier::BOLD),
            x if x < 50.0 => Style::new().fg(Self::WARNING).add_modifier(Modifier::BOLD),
            x if x < 100.0 => Style::new().fg(Self::SUCCESS).add_modifier(Modifier::BOLD),
            _ => Style::new().fg(Self::ACCENT).add_modifier(Modifier::BOLD),
        }
    }

    // Gauge (dynamic based on progress)
    pub fn gauge_style(progress: f64) -> Style {
        match progress {
            p if p < 0.3 => Style::new().fg(Self::ERROR).add_modifier(Modifier::BOLD),
            p if p < 0.7 => Style::new().fg(Self::WARNING).add_modifier(Modifier::BOLD),
            _ => Style::new().fg(Self::SUCCESS).add_modifier(Modifier::BOLD),
        }
    }

    // Progress percentage styling based on completion (for text displays)
    pub fn progress_percentage_style(progress: f64) -> Style {
        let theme = Theme::default();
        match progress {
            p if p < 30.0 => Style::new().fg(theme.error).add_modifier(Modifier::BOLD),
            p if p < 70.0 => Style::new().fg(theme.warning).add_modifier(Modifier::BOLD),
            p if p >= 100.0 => Style::new().fg(theme.success).add_modifier(Modifier::BOLD),
            _ => Style::new()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        }
    }

    // Model name styling based on overall progress
    pub fn model_name_style(progress: f64) -> Style {
        let theme = Theme::default();
        match progress {
            p if p >= 100.0 => Style::new().fg(theme.success).add_modifier(Modifier::BOLD),
            p if p >= 70.0 => Style::new().fg(theme.text_bright),
            p if p >= 30.0 => Style::new().fg(theme.warning),
            _ => Style::new().fg(theme.error),
        }
    }

    // Border styling based on progress state
    pub fn progress_border_style(progress: f64, focused: bool) -> Style {
        let theme = Theme::default();
        if focused {
            Style::new().fg(theme.border_focused)
        } else {
            match progress {
                p if p >= 100.0 => Style::new().fg(theme.success),
                p if p >= 70.0 => Style::new().fg(theme.accent),
                p if p >= 30.0 => Style::new().fg(theme.warning),
                _ => Style::new().fg(theme.error),
            }
        }
    }

    // Summary
    pub fn summary_value() -> Style {
        Style::new()
            .fg(Self::TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
}
