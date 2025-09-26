//! Modern CLI icons with Nerd Font symbols and animations
//! Professional icon and spinner infrastructure for CLI applications
//!
//! Provides professional icon system with animated spinners,
//! semantic symbols, and fallback compatibility for all terminals.
//!
//! NOTE: This module contains library-style infrastructure that may not be fully
//! utilized in the current binary configuration, but provides extensible foundation
//! for future CLI enhancements.

#![allow(dead_code)]

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use termcolor::{ColorSpec, WriteColor};

use super::theme::{CliColors, TerminalCapabilities};

/// Professional CLI icon system with modern symbols
///
/// Provides comprehensive icon collection with semantic meaning,
/// animated effects, and graceful fallback for terminal compatibility.
#[derive(Debug, Clone)]
pub struct CliIcons;

impl CliIcons {
    // Application Icons - Professional CyrupTheme branding with Nerd Font icons
    pub const APP_LOGO: &'static str = "󰾍"; // Professional app icon
    pub const PROGRESS_HUB: &'static str = "󰓱"; // Grid/dashboard icon
    pub const DOWNLOAD: &'static str = "󰉍"; // Download icon
    pub const MODEL: &'static str = "󰛗"; // Model/AI icon

    // Status Icons - Semantic CyrupTheme with Nerd Font icons
    pub const SUCCESS: &'static str = "󰄬"; // Nerd Font checkmark
    pub const ERROR: &'static str = "󰈛"; // Close/error icon
    pub const WARNING: &'static str = "󰚽"; // Warning notification
    pub const INFO: &'static str = "󰋖";

    // File Status Icons - Professional CyrupTheme with Nerd Font icons
    pub const FILE_COMPLETED: &'static str = "󰄬"; // Checkmark
    pub const FILE_DOWNLOADING: &'static str = "󰉍"; // Download arrow
    pub const FILE_PENDING: &'static str = "󰥔"; // Clock
    pub const FILE_CACHED: &'static str = "󰞁"; // Lightning
    pub const FILE_ERROR: &'static str = "󰈛"; // Close/error

    // Progress Icons - Dynamic CyrupTheme with Nerd Font icons
    pub const PROGRESS_COMPLETE: &'static str = "󰄬"; // Success checkmark
    pub const PROGRESS_HIGH: &'static str = "󰾆"; // High gauge
    pub const PROGRESS_MEDIUM: &'static str = "󰾅"; // Medium gauge
    pub const PROGRESS_LOW: &'static str = "󰓅"; // Low gauge

    // Speed Icons - Performance indication with CyrupTheme
    pub const SPEED_BLAZING: &'static str = "󰞁"; // Lightning fast
    pub const SPEED_FAST: &'static str = "󰘳"; // Command/fast
    pub const SPEED_MEDIUM: &'static str = "󰾅"; // Medium gauge
    pub const SPEED_SLOW: &'static str = "󰥔"; // Clock/slow

    // Nerd Font Icons - Modern and professional (with fallbacks)

    /// Get checkmark icon with Nerd Font support
    #[inline]
    pub fn checkmark() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰄬" // Nerd Font checkmark
        } else {
            "✓" // Unicode fallback
        }
    }

    /// Get download icon with Nerd Font support
    #[inline]
    pub fn download() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰇚" // Nerd Font download
        } else {
            "↓" // Unicode fallback
        }
    }

    /// Get folder icon with Nerd Font support
    #[inline]
    pub fn folder() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰉋" // Nerd Font folder
        } else {
            "📁" // Unicode fallback
        }
    }

    /// Get file icon with Nerd Font support
    #[inline]
    pub fn file() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰈙" // Nerd Font file
        } else {
            "📄" // Unicode fallback
        }
    }

    /// Get clock icon with Nerd Font support
    #[inline]
    pub fn clock() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰥔" // Nerd Font clock
        } else {
            "🕐" // Unicode fallback
        }
    }

    /// Get error icon with Nerd Font support
    #[inline]
    pub fn error() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰅖" // Nerd Font error
        } else {
            "✗" // Unicode fallback
        }
    }

    /// Get lightning icon with Nerd Font support
    #[inline]
    pub fn lightning() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰞁" // Nerd Font lightning
        } else {
            "⚡" // Unicode fallback
        }
    }

    /// Get gear icon with Nerd Font support
    #[inline]
    pub fn gear() -> &'static str {
        if TerminalCapabilities::supports_unicode() {
            "󰒓" // Nerd Font gear
        } else {
            "⚙️" // Unicode fallback
        }
    }
}

/// Animated spinner with multiple styles
///
/// Provides professional spinner animations with different styles
/// and smooth frame transitions for active progress indication.
#[derive(Debug, Clone)]
pub struct CliSpinner {
    /// Current spinner style
    style: SpinnerStyle,
    /// Animation speed multiplier
    speed: f32,
    /// Whether spinner is currently active
    active: bool,
}

/// Available spinner animation styles
///
/// Defines different spinner styles for various contexts
/// and visual preferences with Unicode compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerStyle {
    /// Classic dots spinner - universal compatibility
    Dots,
    /// Modern blocks spinner - requires Unicode
    Blocks,
    /// Professional braille spinner - smooth animation
    Braille,
    /// Simple line spinner - ASCII compatible
    Line,
    /// Bouncing ball spinner - playful but professional
    Bounce,
}

impl CliSpinner {
    /// Create new spinner with default style
    ///
    /// Initializes spinner with optimal style based on
    /// terminal capabilities and professional appearance.
    ///
    /// # Returns
    /// New CliSpinner with default configuration
    #[inline]
    pub fn new() -> Self {
        let style = if TerminalCapabilities::supports_unicode() {
            SpinnerStyle::Braille
        } else {
            SpinnerStyle::Dots
        };

        Self {
            style,
            speed: 1.0,
            active: true,
        }
    }

    /// Set spinner style
    ///
    /// Configures spinner animation style with automatic
    /// fallback for terminal compatibility.
    ///
    /// # Arguments
    /// * `style` - Desired spinner style
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set animation speed
    ///
    /// Configures spinner animation speed with reasonable
    /// bounds for optimal visual experience.
    ///
    /// # Arguments
    /// * `speed` - Animation speed multiplier (0.1-5.0)
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.clamp(0.1, 5.0);
        self
    }

    /// Control spinner activity
    ///
    /// Toggles spinner animation for performance optimization
    /// and context-appropriate display control.
    ///
    /// # Arguments
    /// * `active` - Whether spinner should animate
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Get current spinner style
    ///
    /// Returns the currently configured spinner style
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Current SpinnerStyle configuration
    #[inline]
    pub fn get_style(&self) -> SpinnerStyle {
        self.style
    }

    /// Get current animation speed
    ///
    /// Returns the currently configured animation speed
    /// multiplier for inspection and testing purposes.
    ///
    /// # Returns
    /// Current animation speed multiplier
    #[inline]
    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    /// Get current activity status
    ///
    /// Returns whether the spinner is currently active
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Current activity status
    #[inline]
    pub fn get_active(&self) -> bool {
        self.active
    }

    /// Get current spinner frame
    ///
    /// Returns current animation frame based on system time
    /// and configured animation speed for smooth transitions.
    ///
    /// # Returns
    /// Current spinner character for display
    #[inline]
    pub fn current_frame(&self) -> &'static str {
        if !self.active {
            return " ";
        }

        let frames = self.get_frames();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f32;

        let frame_duration = 100.0 / self.speed; // Base 100ms per frame
        let frame_index = ((now / frame_duration) as usize) % frames.len();

        frames[frame_index]
    }

    /// Render spinner with styling
    ///
    /// Renders current spinner frame with appropriate coloring
    /// and professional styling for active progress indication.
    ///
    /// # Arguments
    /// * `stream` - Output stream for rendering
    ///
    /// # Returns
    /// Result indicating success or render error
    pub fn render<W: WriteColor + Write>(&self, stream: &mut W) -> std::io::Result<()> {
        if !self.active {
            write!(stream, " ")?;
            return Ok(());
        }

        // Style spinner with warning color (yellow) for activity
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::WARNING));

        stream.set_color(&spec)?;
        write!(stream, "{}", self.current_frame())?;
        stream.reset()?;

        Ok(())
    }

    /// Get animation frames for current style
    ///
    /// Returns frame sequence for current spinner style
    /// with appropriate fallbacks for terminal compatibility.
    ///
    /// # Returns
    /// Array of animation frames for current style
    #[inline]
    pub fn get_frames(&self) -> &'static [&'static str] {
        match self.style {
            SpinnerStyle::Dots => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            SpinnerStyle::Blocks => &["▁", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃"],
            SpinnerStyle::Braille => &["⠋", "⠙", "⠚", "⠞", "⠖", "⠦", "⠴", "⠲", "⠳", "⠓"],
            SpinnerStyle::Line => &["|", "/", "-", "\\"],
            SpinnerStyle::Bounce => &["⠁", "⠂", "⠄", "⠂"],
        }
    }
}

impl Default for CliSpinner {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Status icon renderer with semantic coloring
///
/// Provides professional status indication with appropriate
/// coloring, symbols, and consistent visual feedback.
#[derive(Debug, Clone)]
pub struct CliStatusIcon {
    /// Icon status type
    status: StatusType,
    /// Whether to use animated spinner for active states
    animated: bool,
    /// Custom icon override
    custom_icon: Option<String>,
}

/// Status types for semantic icon selection
///
/// Defines status categories with appropriate visual representation
/// and semantic meaning for clear user feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusType {
    /// Completed successfully - green checkmark
    Completed,
    /// Currently downloading - animated spinner
    Downloading,
    /// Pending/waiting - clock or dots
    Pending,
    /// From cache - lightning bolt
    Cached,
    /// Error occurred - red X
    Error,
    /// Warning state - yellow triangle
    Warning,
    /// Information - blue i
    Info,
}

impl CliStatusIcon {
    /// Create status icon for specific status
    ///
    /// Initializes status icon with appropriate symbol and
    /// coloring based on status type and context.
    ///
    /// # Arguments
    /// * `status` - Status type for icon selection
    ///
    /// # Returns
    /// New CliStatusIcon with appropriate configuration
    #[inline]
    pub fn new(status: StatusType) -> Self {
        Self {
            status,
            animated: true,
            custom_icon: None,
        }
    }

    /// Control animation for active states
    ///
    /// Toggles animation for downloading and active states
    /// for performance optimization and visual control.
    ///
    /// # Arguments
    /// * `animated` - Whether to show animations
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Set custom icon override
    ///
    /// Allows custom icon specification for special cases
    /// or specific visual requirements.
    ///
    /// # Arguments
    /// * `icon` - Custom icon string
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn custom_icon<T: Into<String>>(mut self, icon: T) -> Self {
        self.custom_icon = Some(icon.into());
        self
    }

    /// Render status icon with appropriate styling
    ///
    /// Renders status icon with semantic coloring and
    /// animation support for professional appearance.
    ///
    /// # Arguments
    /// * `stream` - Output stream for rendering
    ///
    /// # Returns
    /// Result indicating success or render error
    pub fn render<W: WriteColor + Write>(&self, stream: &mut W) -> std::io::Result<()> {
        let (icon, color) = self.get_icon_and_color();

        let mut spec = ColorSpec::new();
        spec.set_fg(Some(color));

        stream.set_color(&spec)?;
        write!(stream, "{icon}")?;
        stream.reset()?;

        Ok(())
    }

    /// Get icon and color for current status
    ///
    /// Returns appropriate icon and color combination
    /// based on status type and terminal capabilities.
    ///
    /// # Returns
    /// Tuple of (icon_string, color) for rendering
    #[inline]
    fn get_icon_and_color(&self) -> (String, termcolor::Color) {
        // Use custom icon if provided
        if let Some(ref custom) = self.custom_icon {
            let color = match self.status {
                StatusType::Completed => CliColors::SUCCESS,
                StatusType::Downloading => CliColors::WARNING,
                StatusType::Pending => CliColors::MUTED,
                StatusType::Cached => CliColors::INFO,
                StatusType::Error => CliColors::ERROR,
                StatusType::Warning => CliColors::WARNING,
                StatusType::Info => CliColors::INFO,
            };
            return (custom.clone(), color);
        }

        match self.status {
            StatusType::Completed => (CliIcons::checkmark().to_string(), CliColors::SUCCESS),
            StatusType::Downloading => {
                if self.animated {
                    let spinner = CliSpinner::new().style(SpinnerStyle::Braille);
                    (spinner.current_frame().to_string(), CliColors::WARNING)
                } else {
                    (CliIcons::download().to_string(), CliColors::WARNING)
                }
            }
            StatusType::Pending => (CliIcons::clock().to_string(), CliColors::MUTED),
            StatusType::Cached => (CliIcons::lightning().to_string(), CliColors::INFO),
            StatusType::Error => (CliIcons::error().to_string(), CliColors::ERROR),
            StatusType::Warning => (CliIcons::WARNING.to_string(), CliColors::WARNING),
            StatusType::Info => (CliIcons::INFO.to_string(), CliColors::INFO),
        }
    }
}

/// Tree structure renderer for hierarchical display
///
/// Provides professional tree visualization with proper indentation,
/// connecting lines, and consistent hierarchical appearance.
#[derive(Debug, Clone)]
pub struct CliTreeRenderer {
    /// Current indentation level
    indent_level: usize,
    /// Whether this is the last item at current level
    is_last: bool,
    /// Whether to use Unicode tree characters
    unicode: bool,
}

impl CliTreeRenderer {
    /// Create new tree renderer
    ///
    /// Initializes tree renderer with optimal settings
    /// for hierarchical display and terminal compatibility.
    ///
    /// # Returns
    /// New CliTreeRenderer with default configuration
    #[inline]
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            is_last: false,
            unicode: TerminalCapabilities::supports_unicode(),
        }
    }

    /// Set indentation level
    ///
    /// Configures tree indentation depth for proper
    /// hierarchical visualization.
    ///
    /// # Arguments
    /// * `level` - Indentation level (0-10)
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn indent_level(mut self, level: usize) -> Self {
        self.indent_level = level.min(10);
        self
    }

    /// Mark as last item in current level
    ///
    /// Configures tree connector style for proper
    /// branch termination visualization.
    ///
    /// # Arguments
    /// * `is_last` - Whether this is the last item
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }

    /// Control Unicode character usage
    ///
    /// Toggles Unicode tree characters for compatibility
    /// with different terminal environments.
    ///
    /// # Arguments
    /// * `unicode` - Whether to use Unicode characters
    ///
    /// # Returns
    /// Self for method chaining
    #[inline]
    pub fn unicode(mut self, unicode: bool) -> Self {
        self.unicode = unicode;
        self
    }

    /// Get current indentation level
    ///
    /// Returns the currently configured indentation level
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Current indentation level
    #[inline]
    pub fn get_indent_level(&self) -> usize {
        self.indent_level
    }

    /// Get current last item status
    ///
    /// Returns whether this renderer is configured for
    /// the last item at the current level.
    ///
    /// # Returns
    /// Current last item status
    #[inline]
    pub fn get_is_last(&self) -> bool {
        self.is_last
    }

    /// Get current Unicode usage status
    ///
    /// Returns whether Unicode tree characters are
    /// currently enabled for rendering.
    ///
    /// # Returns
    /// Current Unicode usage status
    #[inline]
    pub fn get_unicode(&self) -> bool {
        self.unicode
    }

    /// Render tree prefix for hierarchical item
    ///
    /// Renders appropriate tree connector and indentation
    /// for hierarchical display with professional appearance.
    ///
    /// # Arguments
    /// * `stream` - Output stream for rendering
    ///
    /// # Returns
    /// Result indicating success or render error
    pub fn render_prefix<W: WriteColor + Write>(&self, stream: &mut W) -> std::io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(CliColors::MUTED));
        stream.set_color(&spec)?;

        // Render indentation
        for _ in 0..self.indent_level {
            write!(stream, "  ")?;
        }

        // Render tree connector
        if self.indent_level > 0 {
            let connector = if self.unicode {
                if self.is_last { "└─ " } else { "├─ " }
            } else if self.is_last {
                "`- "
            } else {
                "|- "
            };
            write!(stream, "{connector}")?;
        }

        stream.reset()?;
        Ok(())
    }
}

impl Default for CliTreeRenderer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
