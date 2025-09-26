//! Professional CLI theme system matching TUI CyrupTheme standards
//!
//! Provides comprehensive theming for CLI components with dynamic color coding,
//! modern styling, and blazing-fast performance optimization.

use std::io::Write;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

/// Professional CLI theme system with dynamic color coding
///
/// Provides comprehensive theming matching TUI CyrupTheme standards
/// with efficient color management and zero-allocation patterns.
#[allow(dead_code)] // Comprehensive theming API - partially integrated
#[derive(Debug, Clone)]
pub struct CliTheme {
    /// Color choice configuration for terminal compatibility
    color_choice: termcolor::ColorChoice,
}

impl CliTheme {
    /// Create new CLI theme with automatic color detection
    ///
    /// Automatically detects terminal color support and configures
    /// optimal color choice for maximum compatibility.
    ///
    /// # Returns
    /// New CliTheme instance with optimal configuration
    #[inline]
    pub fn new() -> Self {
        Self {
            color_choice: if atty::is(atty::Stream::Stdout) {
                termcolor::ColorChoice::Auto
            } else {
                termcolor::ColorChoice::Never
            },
        }
    }

    /// Create CLI theme with explicit color choice
    ///
    /// Allows manual control over color output for specific scenarios
    /// or testing environments where automatic detection isn't suitable.
    ///
    /// # Arguments
    /// * `color_choice` - Explicit color choice configuration
    ///
    /// # Returns
    /// New CliTheme with specified color configuration
    #[allow(dead_code)]
    #[inline]
    pub fn with_color_choice(color_choice: termcolor::ColorChoice) -> Self {
        Self { color_choice }
    }

    /// Get standard output stream with theme configuration
    ///
    /// Creates StandardStream with theme color choice for consistent
    /// color handling across all CLI components.
    ///
    /// # Returns
    /// StandardStream configured with theme settings
    #[inline]
    pub fn stdout(&self) -> StandardStream {
        StandardStream::stdout(self.color_choice)
    }

    /// Get standard error stream with theme configuration
    ///
    /// Creates StandardStream for error output with theme color choice
    /// for consistent error styling across CLI components.
    ///
    /// # Returns
    /// StandardStream for stderr with theme settings
    #[allow(dead_code)]
    #[inline]
    pub fn stderr(&self) -> StandardStream {
        StandardStream::stderr(self.color_choice)
    }
}

impl Default for CliTheme {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Professional color palette matching CyrupTheme standards
///
/// Provides comprehensive color definitions with semantic meaning
/// and dynamic color coding for various CLI components.
#[allow(dead_code)] // Comprehensive color palette - many colors not yet used
pub struct CliColors;

impl CliColors {
    // Core Professional Palette - matching CyrupTheme
    pub const PRIMARY: Color = Color::Rgb(240, 245, 255);
    pub const SECONDARY: Color = Color::Rgb(200, 205, 215);
    pub const MUTED: Color = Color::Rgb(140, 145, 155);
    pub const ACCENT: Color = Color::Rgb(130, 170, 255);

    // Status Colors - semantic and professional
    pub const SUCCESS: Color = Color::Rgb(130, 220, 150);
    pub const WARNING: Color = Color::Rgb(250, 200, 100);
    pub const ERROR: Color = Color::Rgb(250, 120, 120);
    pub const INFO: Color = Color::Rgb(130, 170, 255);

    // Progress Colors - dynamic based on performance
    #[allow(dead_code)]
    pub const SPEED_SLOW: Color = Color::Rgb(250, 120, 120); // Red for <10 MB/s
    #[allow(dead_code)]
    pub const SPEED_MEDIUM: Color = Color::Rgb(250, 200, 100); // Yellow for 10-50 MB/s
    #[allow(dead_code)]
    pub const SPEED_FAST: Color = Color::Rgb(130, 220, 150); // Green for 50-100 MB/s
    #[allow(dead_code)]
    pub const SPEED_BLAZING: Color = Color::Rgb(130, 170, 255); // Blue for >100 MB/s

    // Progress Bar Colors
    pub const PROGRESS_LOW: Color = Color::Rgb(250, 120, 120); // Red for <30%
    pub const PROGRESS_MEDIUM: Color = Color::Rgb(250, 200, 100); // Yellow for 30-70%
    pub const PROGRESS_HIGH: Color = Color::Rgb(130, 220, 150); // Green for >70%
    pub const PROGRESS_COMPLETE: Color = Color::Rgb(130, 170, 255); // Blue for 100%

    // Special Effect Colors
    #[allow(dead_code)]
    pub const HIGHLIGHT: Color = Color::Rgb(255, 255, 255); // Pure white for emphasis
    #[allow(dead_code)]
    pub const BORDER: Color = Color::Rgb(80, 85, 95); // Subtle border color
    #[allow(dead_code)]
    pub const BACKGROUND: Color = Color::Rgb(30, 32, 40); // Dark background hint
}

/// Professional color specs for CLI components
///
/// Provides pre-configured ColorSpec instances for common styling patterns
/// with efficient creation and consistent appearance across components.
#[allow(dead_code)] // Comprehensive ColorSpec API - some methods not yet used
pub struct CliColorSpecs;

#[allow(dead_code)]
impl CliColorSpecs {
    /// Header title style - bold primary with accent
    #[inline]
    pub fn header_title() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::ACCENT)).set_bold(true);
        spec
    }

    /// Model name style - primary text with bold
    #[inline]
    pub fn model_name() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::PRIMARY)).set_bold(true);
        spec
    }

    /// Progress percentage style with dynamic coloring
    ///
    /// Returns ColorSpec with color based on progress percentage
    /// for immediate visual feedback on completion status.
    ///
    /// # Arguments
    /// * `percentage` - Progress percentage (0-100)
    ///
    /// # Returns
    /// ColorSpec with appropriate color for progress level
    #[inline]
    pub fn progress_percentage(percentage: u8) -> ColorSpec {
        let mut spec = ColorSpec::new();
        let color = match percentage {
            0..=29 => CliColors::PROGRESS_LOW,
            30..=69 => CliColors::PROGRESS_MEDIUM,
            70..=99 => CliColors::PROGRESS_HIGH,
            100 => CliColors::PROGRESS_COMPLETE,
            101..=u8::MAX => CliColors::PROGRESS_COMPLETE, // Handle any percentage above 100% as complete
        };
        spec.set_fg(Some(color)).set_bold(true);
        spec
    }

    /// Speed indicator style with dynamic coloring
    ///
    /// Returns ColorSpec with color based on download speed
    /// for immediate visual feedback on performance.
    ///
    /// # Arguments
    /// * `speed_mbps` - Download speed in MB/s
    ///
    /// # Returns
    /// ColorSpec with appropriate color for speed level
    #[inline]
    pub fn speed_indicator(speed_mbps: f64) -> ColorSpec {
        let mut spec = ColorSpec::new();
        let color = match speed_mbps {
            x if x < 10.0 => CliColors::SPEED_SLOW,
            x if x < 50.0 => CliColors::SPEED_MEDIUM,
            x if x < 100.0 => CliColors::SPEED_FAST,
            _ => CliColors::SPEED_BLAZING,
        };
        spec.set_fg(Some(color)).set_bold(true);
        spec
    }

    /// File size style - magenta secondary
    #[inline]
    pub fn file_size() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::SECONDARY));
        spec
    }

    /// Status icon style with semantic coloring
    ///
    /// Returns ColorSpec appropriate for file status indication
    /// with semantic color coding for immediate recognition.
    ///
    /// # Arguments
    /// * `completed` - Whether file is completed
    /// * `active` - Whether file is actively downloading
    /// * `cached` - Whether file is from cache
    ///
    /// # Returns
    /// ColorSpec with appropriate semantic color
    #[inline]
    #[allow(dead_code)]
    pub fn status_icon(completed: bool, active: bool, cached: bool) -> ColorSpec {
        let mut spec = ColorSpec::new();
        let color = match (completed, active, cached) {
            (true, _, _) => CliColors::SUCCESS, // Completed - green
            (_, true, _) => CliColors::WARNING, // Active - yellow
            (_, _, true) => CliColors::INFO,    // Cached - blue
            _ => CliColors::MUTED,              // Pending - muted
        };
        spec.set_fg(Some(color));
        spec
    }

    /// Tree structure style - muted for hierarchy
    #[inline]
    pub fn tree_structure() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::MUTED));
        spec
    }

    /// Overall progress style - accent with bold
    #[inline]
    pub fn overall_progress() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::ACCENT)).set_bold(true);
        spec
    }

    /// Error message style - error color with bold
    #[inline]
    pub fn error_message() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::ERROR)).set_bold(true);
        spec
    }

    /// Success message style - success color with bold
    #[inline]
    pub fn success_message() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::SUCCESS)).set_bold(true);
        spec
    }

    /// Reset style for clearing formatting
    #[inline]
    pub fn reset() -> ColorSpec {
        ColorSpec::new()
    }
}

/// Terminal capability detection and optimization
///
/// Provides utilities for detecting terminal capabilities and
/// optimizing output for different terminal environments.
#[allow(dead_code)] // Terminal capability API - methods not all used yet
pub struct TerminalCapabilities;

#[allow(dead_code)]
impl TerminalCapabilities {
    /// Check if terminal supports color output
    ///
    /// Detects color support using multiple methods for
    /// maximum compatibility across terminal environments.
    ///
    /// # Returns
    /// Boolean indicating color support availability
    #[inline]
    pub fn supports_color() -> bool {
        atty::is(atty::Stream::Stdout)
            && (std::env::var("NO_COLOR").is_err()
                || std::env::var("NO_COLOR") == Ok("".to_string()))
    }

    /// Check if terminal supports Unicode characters
    ///
    /// Detects Unicode support for advanced progress bars
    /// and modern icon rendering capabilities.
    ///
    /// # Returns
    /// Boolean indicating Unicode support availability
    #[inline]
    pub fn supports_unicode() -> bool {
        // Check common environment variables that indicate Unicode support
        if let Ok(term) = std::env::var("TERM") {
            !term.contains("vt100") && !term.contains("ansi")
        } else {
            true // Assume Unicode support by default
        }
    }

    /// Get optimal progress bar width for terminal
    ///
    /// Calculates optimal progress bar width based on terminal
    /// size and content requirements for best visual appearance.
    ///
    /// # Returns
    /// Optimal progress bar width in characters
    #[inline]
    pub fn optimal_progress_width() -> usize {
        if let Some((width, _)) = term_size::dimensions() {
            // Reserve space for text and padding, use 40% of terminal width
            ((width as f32 * 0.4) as usize).clamp(20, 60)
        } else {
            40 // Default width for unknown terminal size
        }
    }

    /// Check if terminal supports true color (24-bit)
    ///
    /// Detects true color support for advanced color gradients
    /// and professional color schemes.
    ///
    /// # Returns
    /// Boolean indicating true color support availability
    #[inline]
    pub fn supports_true_color() -> bool {
        std::env::var("COLORTERM")
            .map(|v| v == "truecolor" || v == "24bit")
            .unwrap_or(false)
    }
}

/// Efficient styling utilities for CLI components
///
/// Provides helper functions for common styling operations
/// with zero-allocation patterns and optimized performance.
#[allow(dead_code)] // Styling utility API - some methods not used yet
pub struct CliStyling;

#[allow(dead_code)]
impl CliStyling {
    /// Write styled text with automatic color spec application
    ///
    /// Efficiently writes text with color formatting and automatic
    /// reset to prevent color bleeding between components.
    ///
    /// # Arguments
    /// * `stream` - StandardStream for output
    /// * `text` - Text content to write
    /// * `spec` - ColorSpec for styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Performance
    /// - Efficient color application with minimal state changes
    /// - Automatic reset prevents formatting conflicts
    /// - Zero-allocation text handling with direct writing
    #[inline]
    pub fn write_styled<W: WriteColor + Write>(
        stream: &mut W,
        text: &str,
        spec: &ColorSpec,
    ) -> std::io::Result<()> {
        stream.set_color(spec)?;
        write!(stream, "{text}")?;
        stream.reset()?;
        Ok(())
    }

    /// Write styled line with automatic newline and reset
    ///
    /// Efficiently writes complete line with styling and automatic
    /// newline termination for structured output formatting.
    ///
    /// # Arguments
    /// * `stream` - StandardStream for output
    /// * `text` - Text content to write
    /// * `spec` - ColorSpec for styling
    ///
    /// # Returns
    /// Result indicating success or write error
    #[inline]
    pub fn write_styled_line<W: WriteColor + Write>(
        stream: &mut W,
        text: &str,
        spec: &ColorSpec,
    ) -> std::io::Result<()> {
        stream.set_color(spec)?;
        writeln!(stream, "{text}")?;
        stream.reset()?;
        Ok(())
    }

    /// Create gradient color between two colors
    ///
    /// Generates intermediate color for smooth color transitions
    /// and professional gradient effects in progress indicators.
    ///
    /// # Arguments
    /// * `start` - Starting color for gradient
    /// * `end` - Ending color for gradient  
    /// * `position` - Position in gradient (0.0 to 1.0)
    ///
    /// # Returns
    /// Interpolated Color for gradient position
    #[inline]
    pub fn gradient_color(start: Color, end: Color, position: f32) -> Color {
        let pos = position.clamp(0.0, 1.0);

        match (start, end) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let r = (r1 as f32 + (r2 as f32 - r1 as f32) * pos) as u8;
                let g = (g1 as f32 + (g2 as f32 - g1 as f32) * pos) as u8;
                let b = (b1 as f32 + (b2 as f32 - b1 as f32) * pos) as u8;
                Color::Rgb(r, g, b)
            }
            _ => start, // Fallback to start color for unsupported color types
        }
    }
}
