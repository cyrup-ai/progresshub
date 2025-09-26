//! Termcolor abstraction for custom progress callbacks
//!
//! Provides high-level termcolor utilities with theme-based colors and professional
//! terminal output for use in custom progress event callbacks.

use anyhow::Result;
use std::io::Write as IoWrite;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// High-level termcolor abstraction with theme-based colors
///
/// Provides semantic color methods and professional terminal output
/// for use in custom progress event callbacks. Uses termcolor architecture
/// with meaningful theme-based color semantics.
///
/// # Architecture
/// - Professional termcolor output with StandardStream
/// - Theme-based semantic colors (SUCCESS, INFO, WARNING, ERROR)
/// - Zero allocation string formatting where possible
/// - Approved iconset for consistent visual design
pub struct Termcolor {
    stdout: StandardStream,
}

impl Termcolor {
    /// Create new Termcolor with automatic color choice
    ///
    /// # Returns
    /// New Termcolor instance ready for themed output
    ///
    /// # Architecture
    /// - Uses ColorChoice::Auto for intelligent terminal color detection
    /// - Professional StandardStream for consistent output
    #[inline]
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
        }
    }

    /// Write text with SUCCESS theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with success styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses bright green for success indication
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn success(&mut self, text: &str) -> Result<()> {
        self.stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true),
        )?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write text with INFO theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with info styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses bright cyan for information display
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn info(&mut self, text: &str) -> Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_intense(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write text with WARNING theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with warning styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses bright yellow for warning indication
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn warning(&mut self, text: &str) -> Result<()> {
        self.stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_intense(true),
        )?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write text with ERROR theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with error styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses bright red for error indication
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn error(&mut self, text: &str) -> Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_intense(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write text with ACCENT theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with accent styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses bright blue for accent/highlight display
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn accent(&mut self, text: &str) -> Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_intense(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write text with MUTED theme color
    ///
    /// # Arguments
    /// * `text` - Text to display with muted styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Theme
    /// - Uses dim white for muted/secondary information
    /// - Professional terminal output with proper color reset
    #[inline]
    pub fn muted(&mut self, text: &str) -> Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::White)).set_dimmed(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write plain text without styling
    ///
    /// # Arguments
    /// * `text` - Text to display without color styling
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Architecture
    /// - Direct write without color modifications
    /// - Professional terminal output with flush
    #[inline]
    pub fn plain(&mut self, text: &str) -> Result<()> {
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write newline
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Architecture
    /// - Simple newline output with flush
    #[inline]
    pub fn newline(&mut self) -> Result<()> {
        writeln!(&mut self.stdout)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Clear current line (carriage return without newline)
    ///
    /// # Returns
    /// Result indicating success or write error
    ///
    /// # Architecture
    /// - Carriage return for line clearing/overwriting
    /// - Useful for updating progress on same line
    #[inline]
    pub fn clear_line(&mut self) -> Result<()> {
        write!(&mut self.stdout, "\r")?;
        self.stdout.flush()?;
        Ok(())
    }
}

impl Default for Termcolor {
    fn default() -> Self {
        Self::new()
    }
}
