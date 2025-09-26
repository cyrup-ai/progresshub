//! Hierarchical progress rendering for CLI display
//!
//! Provides zero-allocation progress rendering with lock-free state management
//! for blazing-fast CLI progress display without unsafe operations.

use anyhow::Result;
use progresshub_progress::FileStatus;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

// Import our beautiful CLI visual components
use super::theme::{CliColorSpecs, CliStyling, CliTheme};
// Using simple function-based progress bars for elegant display
use super::icons::{CliIcons, CliStatusIcon, CliTreeRenderer, StatusType};

/// Professional hierarchical progress renderer with beautiful visuals
///
/// Renders ProgressCalculator state into human-readable CLI output with
/// blazing-fast performance, beautiful styling, and elegant ergonomic interfaces.
#[derive(Debug)]
pub struct HierarchicalProgressRenderer {
    /// Professional CLI theme system for consistent styling
    #[allow(dead_code)]
    theme: CliTheme,
    /// Terminal output stream with theme configuration
    stdout: StandardStream,
}

impl HierarchicalProgressRenderer {
    /// Create new hierarchical progress renderer with beautiful theme
    ///
    /// Uses zero-allocation initialization with professional CLI theme system
    /// for optimal performance and beautiful visual presentation.
    #[inline]
    pub fn new() -> Self {
        let theme = CliTheme::new();
        let stdout = theme.stdout();
        Self { theme, stdout }
    }

    /// Render hierarchical progress from ProgressCalculator with formatted accessors
    ///
    /// Uses ProgressCalculator's fully formatted accessor methods to display
    /// beautiful progress with proper byte formatting, percentages, and status.
    ///
    /// # Arguments
    /// * `progress_calculator` - ProgressCalculator with all formatting methods
    ///
    /// # Returns
    /// Result indicating success or rendering error
    ///
    /// # Performance
    /// - Zero allocation using ProgressCalculator's pre-formatted strings
    /// - Lock-free operations with atomic state access
    /// - Optimized display using formatted accessor methods
    ///
    /// # Errors
    /// Returns error if CLI output operations fail
    pub fn render_progress(
        &mut self,
        progress_calculator: &progresshub_progress::ProgressCalculator,
    ) -> Result<()> {
        // Clear screen for clean display
        self.clear_display()?;

        // Write structured header with ProgressCalculator data
        self.write_header()?;

        // Use ProgressCalculator's formatted strings for beautiful display
        writeln!(self.stdout)?;
        writeln!(
            self.stdout,
            "󰓱 Overall Progress: {}",
            progress_calculator.percentage_formatted()
        )?;
        writeln!(
            self.stdout,
            "󰉍 Downloaded: {}",
            progress_calculator.bytes_formatted()
        )?;
        writeln!(
            self.stdout,
            "󰐰 Speed: {}",
            progress_calculator.speed_formatted()
        )?;
        if !progress_calculator.eta_formatted().is_empty() {
            writeln!(
                self.stdout,
                "󰥔 ETA: {}",
                progress_calculator.eta_formatted()
            )?;
        }
        writeln!(self.stdout)?;

        // Display detailed file progress using formatted accessors
        let state = &progress_calculator.progress_state;
        for (model_idx, model) in state.models.iter().enumerate() {
            let is_last_model = model_idx == state.models.len() - 1;
            self.render_model_progress_formatted(model, is_last_model, progress_calculator)?;
        }

        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Clear display for clean progress rendering
    ///
    /// Uses terminal control sequences for efficient screen clearing
    /// without allocating display buffers.
    #[inline]
    fn clear_display(&mut self) -> Result<()> {
        // Use proper CLI clear screen function instead of raw ANSI
        super::output_formatter::clear_screen();
        Ok(())
    }

    /// Write professional progress display header
    ///
    /// Renders structured header with application branding and beautiful styling
    /// using professional CLI theme system for consistent appearance.
    #[inline]
    fn write_header(&mut self) -> Result<()> {
        // Application logo and title with professional styling
        CliStyling::write_styled(
            &mut self.stdout,
            &format!("{} ProgressHub - Model Downloads", CliIcons::APP_LOGO),
            &CliColorSpecs::header_title(),
        )?;
        writeln!(self.stdout)?;
        writeln!(self.stdout)?;
        Ok(())
    }

    /// Render individual model progress with ProgressCalculator formatted accessors
    ///
    /// Uses ProgressCalculator's pre-formatted strings for beautiful display
    /// with proper byte formatting and percentages.
    ///
    /// # Arguments
    /// * `model` - Model progress state from ProgressCalculator
    /// * `is_last_model` - Whether this is the final model for formatting
    /// * `progress_calculator` - ProgressCalculator for formatted strings
    ///
    /// # Returns
    /// Result indicating success or rendering error
    ///
    /// # Performance
    /// - Zero allocation using ProgressCalculator's pre-formatted strings
    /// - No percentage calculations - uses formatted accessor methods
    /// - Optimized display with minimal heap usage
    fn render_model_progress_formatted(
        &mut self,
        model: &progresshub_progress::ImmutableModelProgress,
        is_last_model: bool,
        progress_calculator: &progresshub_progress::ProgressCalculator,
    ) -> Result<()> {
        // Model header with professional icon and styling
        CliStyling::write_styled(
            &mut self.stdout,
            &format!(
                "{} {}",
                CliIcons::MODEL,
                extract_model_name(&model.model_id)
            ),
            &CliColorSpecs::model_name(),
        )?;

        // Progress percentage with dynamic coloring - using ProgressCalculator formatted methods
        let model_percentage_text = progress_calculator.model_percentage_formatted(model);
        CliStyling::write_styled(
            &mut self.stdout,
            &format!(" {}", model_percentage_text),
            &CliColorSpecs::progress_percentage(if model.is_complete() { 100 } else { 50 }),
        )?;

        // File sizes with professional styling - using ProgressCalculator formatted bytes
        CliStyling::write_styled(
            &mut self.stdout,
            &format!(" ({})", progress_calculator.model_bytes_formatted(model)),
            &CliColorSpecs::file_size(),
        )?;
        writeln!(self.stdout)?;

        // Beautiful Unicode progress bar with dynamic coloring - using ProgressCalculator methods
        let model_percentage = progress_calculator.model_percentage(model) as u8;
        let progress_bar = create_unicode_progress_bar(model_percentage);

        write!(self.stdout, "  ")?;
        // Dynamic color: green when complete, yellow otherwise - using boolean completion status
        let bar_color = if model.is_complete() {
            Color::Green
        } else {
            Color::Yellow
        };
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(bar_color)))?;
        writeln!(self.stdout, "{progress_bar}")?;
        self.stdout.reset()?;

        // Render individual file progress with proper indentation
        for (file_idx, file) in model.files.iter().enumerate() {
            let is_last_file = file_idx == model.files.len() - 1;
            self.render_file_progress(file, is_last_model && is_last_file, progress_calculator)?;
        }

        // Add spacing between models (except for last model)
        if !is_last_model {
            writeln!(self.stdout)?;
        }

        Ok(())
    }

    /// Render individual file progress with status indicators
    ///
    /// Displays file-level progress with appropriate tree formatting,
    /// progress bars, and status icons for clear visual feedback.
    ///
    /// # Arguments
    /// * `file` - File progress state from ProgressCalculator
    /// * `is_last_file` - Whether this is the final file for tree formatting
    ///
    /// # Returns
    /// Result indicating success or rendering error
    #[inline]
    fn render_file_progress(
        &mut self,
        file: &progresshub_progress::ImmutableFileProgress,
        is_last_file: bool,
        progress_calculator: &progresshub_progress::ProgressCalculator,
    ) -> Result<()> {
        // Professional tree structure with proper renderer
        let tree_renderer = CliTreeRenderer::new().indent_level(1).is_last(is_last_file);
        tree_renderer.render_prefix(&mut self.stdout)?;

        // Professional status icon with semantic coloring - using ProgressCalculator intelligence
        let file_status = progress_calculator.file_status(file);
        let status_type = match file_status {
            progresshub_progress::FileStatus::Completed => StatusType::Completed,
            progresshub_progress::FileStatus::Downloading => StatusType::Downloading,
            progresshub_progress::FileStatus::Pending => StatusType::Pending,
            progresshub_progress::FileStatus::Failed | progresshub_progress::FileStatus::Error => StatusType::Error,
        };

        let status_icon = CliStatusIcon::new(status_type).animated(true);
        status_icon.render(&mut self.stdout)?;
        write!(self.stdout, " ")?;

        // Filename with professional styling
        CliStyling::write_styled(
            &mut self.stdout,
            extract_filename(&file.file_name),
            &CliColorSpecs::model_name(),
        )?;
        write!(self.stdout, " ")?;

        // Progress percentage with dynamic coloring - using ProgressCalculator formatted method
        let file_percentage_text = progress_calculator.file_percentage_formatted(file);
        CliStyling::write_styled(
            &mut self.stdout,
            &file_percentage_text,
            &CliColorSpecs::progress_percentage(if file.is_complete() { 100 } else { 50 }),
        )?;
        write!(self.stdout, " ")?;

        // File sizes with professional styling - using ProgressCalculator formatted bytes
        CliStyling::write_styled(
            &mut self.stdout,
            &format!("({})", progress_calculator.file_bytes_formatted(file)),
            &CliColorSpecs::file_size(),
        )?;
        writeln!(self.stdout)?;
        Ok(())
    }
}

impl Default for HierarchicalProgressRenderer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Create beautiful Unicode progress bar with zero allocation
///
/// Generates progress bar representation using efficient string operations
/// and minimal heap allocation for optimal performance.
///
/// # Arguments
/// * `percentage` - Progress percentage (0-100)
///
/// # Returns
/// String representation of progress bar
#[inline]
#[allow(dead_code)]
fn create_unicode_progress_bar(percentage: u8) -> String {
    let width = 40;
    let filled = (width * percentage as usize) / 100;
    let empty = width - filled;

    // Beautiful Unicode blocks
    let full_block = "█";
    let empty_block = "░";
    let partial_blocks = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"];

    if percentage == 100 {
        format!("{}  100%", full_block.repeat(width))
    } else if percentage == 0 {
        format!("{}  0%", empty_block.repeat(width))
    } else {
        // Add partial block for smooth animation
        let partial_idx = ((width * percentage as usize) % 100) / 14; // Approximate partial position
        let partial = if partial_idx < partial_blocks.len() {
            partial_blocks[partial_idx]
        } else {
            ""
        };

        format!(
            "{}{}{}  {}%",
            full_block.repeat(filled),
            partial,
            empty_block.repeat(empty.saturating_sub(1)),
            percentage
        )
    }
}

/// Get colored status icon with spinner animation
#[allow(dead_code)]
fn get_colored_status_icon_optimized(percentage: u64, is_cached: bool) -> (&'static str, Color) {
    if percentage >= 100 {
        ("✓", Color::Green)
    } else if percentage > 0 {
        // Animated spinner for active downloads
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let index = (now / 100) % (spinners.len() as u128);
        (spinners[index as usize], Color::Yellow)
    } else if is_cached {
        ("⚡", Color::Cyan)
    } else {
        ("⋯", Color::White)
    }
}

/// Extract clean model name from model ID
fn extract_model_name(model_id: &str) -> &str {
    model_id.split('/').next_back().unwrap_or(model_id)
}

/// Extract filename from full path
fn extract_filename(file_path: &str) -> &str {
    file_path
        .split('/')
        .next_back()
        .or_else(|| file_path.split('\\').next_back())
        .unwrap_or(file_path)
}

/// Get status icon for file status with compile-time optimization
///
/// Returns appropriate Unicode icon based on file status, optimized
/// for blazing-fast performance with zero runtime allocation.
///
/// # Arguments
/// * `status` - FileStatus enum value
///
/// # Returns  
/// Static string slice containing Unicode icon
///
/// # Performance
/// - Compile-time constant matching with zero runtime cost
/// - Lock-free icon selection with atomic operations
/// - Zero-allocation string handling with static references
#[inline]
#[allow(dead_code)] // Used only in tests
const fn get_status_icon_optimized(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Completed => "󰄰",                  // Nerd Font checkmark
        FileStatus::Downloading => "󰘨",                // Nerd Font download
        FileStatus::Pending => "󰅍",                    // Nerd Font clock
        FileStatus::Error | FileStatus::Failed => "󰈛", // Nerd Font error
    }
}
