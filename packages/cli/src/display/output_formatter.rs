//! CLI output formatting and structured output handling
//!
//! Provides zero-allocation CLI output abstraction that separates user-facing
//! output from diagnostic logging, with proper error handling and no unwrap/expect calls.

use std::io::{self, Write};
use tracing;

/// High-performance CLI output abstraction with zero-allocation formatting
///
/// Separates user-facing CLI output from diagnostic logging, providing structured
/// output with proper error handling and no unwrap/expect calls.
pub struct CliOutput {
    writer: Box<dyn Write + Send + Sync>,
}

impl std::fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliOutput").finish()
    }
}

impl Default for CliOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl CliOutput {
    /// Create new CLI output writer with stdout as default
    pub fn new() -> Self {
        Self {
            writer: Box::new(io::stdout()),
        }
    }

    /// Write a line to CLI output with proper error handling
    #[inline]
    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.writer, "{line}")?;
        self.writer.flush()
    }

    /// Write formatted CLI table header with structured logging
    #[allow(dead_code)]
    pub fn write_progress_header(&mut self) -> io::Result<()> {
        tracing::debug!("Displaying CLI progress table header");

        self.write_line(
            "┌─────────────────────────────────────────────────────────────────────────────┐",
        )?;
        self.write_line(
            "│ 📊 Model Download Progress (Live)                                          │",
        )?;
        self.write_line(
            "├─────────────────────────────────────────────────────────────────────────────┤",
        )?;
        self.write_line(
            "│ File                    Status      Progress                      Speed     │",
        )?;
        self.write_line(
            "├─────────────────────────────────────────────────────────────────────────────┤",
        )
    }

    /// Write formatted CLI table footer
    #[allow(dead_code)]
    pub fn write_progress_footer(&mut self) -> io::Result<()> {
        self.write_line(
            "└─────────────────────────────────────────────────────────────────────────────┘",
        )
    }

    /// Write empty state message
    #[allow(dead_code)]
    pub fn write_empty_state(&mut self) -> io::Result<()> {
        tracing::info!("No files to display in progress table");
        self.write_line(
            "│ No files to display                                                         │",
        )
    }

    /// Write formatted progress row with zero-allocation formatting
    #[allow(dead_code)]
    pub fn write_progress_row(
        &mut self,
        display_filename: &str,
        status_display: &str,
        downloaded_display: &str,
        speed_display: &str,
    ) -> io::Result<()> {
        // Use write! for zero-allocation formatting with proper error handling
        match writeln!(
            self.writer,
            "│ {display_filename} {status_display} {downloaded_display:<20} {speed_display:>10} │"
        ) {
            Ok(()) => {
                self.writer.flush()?;
                tracing::trace!(
                    file = display_filename,
                    status = status_display,
                    downloaded = downloaded_display,
                    speed = speed_display,
                    "CLI progress row displayed"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to write CLI progress row");
                Err(e)
            }
        }
    }
}

/// Clear screen and move cursor to top with TTY-independent handling
/// NOTE: This function should NEVER be called during TUI mode as it corrupts the terminal
pub fn clear_screen() {
    // Check if we're in a TTY using atty crate (already in dependencies)
    if !atty::is(atty::Stream::Stdout) {
        return; // Don't clear screen if not in a proper TTY
    }

    // Only clear screen in CLI mode, never in TUI mode
    tracing::debug!("Clearing screen for CLI display");

    // Use standard ANSI escape sequences for clearing screen
    if let Err(e) = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[2J\x1b[H") {
        tracing::warn!("Failed to clear screen: {}", e);
        // No fallback - continue without clearing
    }
}
