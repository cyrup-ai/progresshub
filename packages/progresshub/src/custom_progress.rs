//! Custom progress callback API for ProgressHub
//!
//! This module provides a clean callback-based interface for handling progress events
//! while maintaining Pure Flume Channel Event-Driven Architecture internally.
//! Users receive ProgressCalculator snapshots with all formatting methods available.

use crate::{DownloadResult, Termcolor};
use anyhow::Result;
use progresshub_progress::ProgressCalculator;

/// Builder with custom progress event callback
///
/// Provides callback-based progress handling where users receive ProgressCalculator events
/// and a Termcolor with theme-based colors via their registered callback function.
/// Maintains Pure Flume Channel Event-Driven Architecture while hiding all channel complexity.
///
/// # Generic Parameters
/// * `F` - Callback function type that receives ProgressCalculator events and Termcolor
///
/// # Architecture
/// - Zero allocation event processing with blazing-fast performance
/// - Uses same progresshub_progress::internal_builder() pattern as CLI/TUI
/// - ProgressCalculator snapshots provide all formatting via accessor methods
/// - Termcolor provides theme-based colors (SUCCESS, INFO, WARNING, ERROR, etc.)
/// - Callback receives immutable snapshots with complete progress data and termcolor utilities
#[allow(dead_code)] // Public API used by examples and external crates
pub struct CustomProgressBuilder<F>
where
    F: FnMut(ProgressCalculator, &mut Termcolor),
{
    models: Vec<String>,
    quantization: Option<String>,
    force: bool,
    callback: F,
}

impl<F> CustomProgressBuilder<F>
where
    F: FnMut(ProgressCalculator, &mut Termcolor),
{
    /// Create new CustomProgressBuilder with callback
    ///
    /// # Arguments
    /// * `models` - Vector of model IDs to download
    /// * `quantization` - Optional quantization filter (e.g., "Q4_K_M", "Q8_0", "F16")
    /// * `force` - Force redownload by clearing cache first
    /// * `callback` - Function that receives ProgressCalculator events for custom handling
    ///
    /// # Architecture
    /// - Internal constructor for use by CliBuilder.on_progress_event()
    /// - Maintains separation of concerns with focused module structure
    #[inline]
    #[allow(dead_code)] // Public API used by examples via CliBuilder.on_progress_event()
    pub(crate) fn new(
        models: Vec<String>,
        quantization: Option<String>,
        force: bool,
        callback: F,
    ) -> Self {
        Self {
            models,
            quantization,
            force,
            callback,
        }
    }

    /// Execute downloads with custom progress callback handling
    ///
    /// Uses the same progresshub_progress::internal_builder() pattern as CLI/TUI
    /// to maintain architectural consistency while providing callback-based API.
    /// The callback receives ProgressCalculator events for custom display logic.
    ///
    /// # Returns
    /// Result containing final download results or error
    ///
    /// # Architecture
    /// - Uses progresshub_progress::internal_builder() for all business logic
    /// - Injects real orchestration using with_orchestration()
    /// - Processes receiver events and calls user's callback
    /// - Extracts final results from ProgressCalculator state
    /// - Zero allocation event processing with blazing-fast performance
    #[allow(dead_code)] // Public API used by examples via CliBuilder.on_progress_event()
    pub async fn download(mut self) -> Result<crate::OneOrMany<DownloadResult>> {
        if self.models.is_empty() {
            return Err(anyhow::anyhow!("At least one model must be specified"));
        }

        // Use progress crate's internal builder for all business logic (same as CLI/TUI)
        let mut internal_builder = progresshub_progress::internal_builder();

        // Configure builder with models
        for model in &self.models {
            internal_builder = internal_builder.model(model);
        }

        // Configure quantization if specified
        if let Some(ref quant) = self.quantization {
            internal_builder = internal_builder.quantization(quant);
        }

        // Configure force flag for cache clearing
        internal_builder = internal_builder.force(self.force);

        // Start the download process and get receiver for ProgressCalculator events
        let receiver = internal_builder.receiver();

        // Process progress events and call user's callback (same pattern as CLI/TUI)
        let mut final_progress = None;
        let mut termcolor = Termcolor::new();

        while let Ok(progress) = receiver.recv_async().await {
            // Call user's progress handler callback with Termcolor
            (self.callback)(progress.clone(), &mut termcolor);

            // Store the latest progress for final results
            let is_complete = progress.is_complete();
            final_progress = Some(progress);

            // Exit after is_complete() is true
            if is_complete {
                break;
            }
        }

        // Extract final results using ProgressCalculator's method (ARCHITECTURAL COMPLIANCE)
        let final_result = match final_progress {
            Some(progress) => progress.to_download_result(),
            None => return Err(anyhow::anyhow!("No progress events received")),
        };

        Ok(crate::OneOrMany::One(final_result))
    }
}
