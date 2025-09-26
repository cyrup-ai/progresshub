//! CLI Display Layout - ARCHITECTURAL COMPLIANCE
//!
//! Consumes ProgressCalculator events from flume channels ONLY
//! Displays formatted data using ProgressCalculator.accessor_methods()

// ARCHITECTURAL COMPLIANCE: ONLY ProgressCalculator events and bandwidth events via flume channels
// NO local state management, NO raw calculations, NO data transformation
// ALL formatting comes from ProgressCalculator accessor methods ONLY

use anyhow::Result;
use progresshub_progress::{DownloadResult, ProgressCalculator};

/// CLI Result Display - consumes ProgressCalculator events ONLY
pub struct CliResultDisplay;

impl CliResultDisplay {
    /// Display progress using ProgressCalculator formatted methods ONLY
    #[inline]
    pub fn display_progress(progress_calculator: &ProgressCalculator) {
        // Use ONLY ProgressCalculator accessor methods:
        tracing::debug!(
            "📊 Progress: {}",
            progress_calculator.percentage_formatted()
        );
        tracing::debug!("📈 Downloaded: {}", progress_calculator.bytes_formatted());
        tracing::debug!("🚀 Speed: {}", progress_calculator.speed_formatted());
        if !progress_calculator.eta_formatted().is_empty() {
            tracing::debug!("⏱️  ETA: {}", progress_calculator.eta_formatted());
        }
    }
}

/// Download Result Processor - PURE EVENT-DRIVEN ARCHITECTURE
/// Processes download results using ONLY ProgressCalculator events via flume channels
pub struct DownloadResultProcessor;

impl DownloadResultProcessor {
    /// Display concise results summary for completed downloads
    /// Uses only data available in DownloadResult without extra allocations
    pub fn display_download_results(&mut self, results: &Vec<DownloadResult>) -> Result<()> {
        for result in results {
            // Iterate models safely via ZeroOneOrMany::iter()
            for model in result.models.iter() {
                let status = match &model.status {
                    progresshub_progress::DownloadStatus::Complete => "complete",
                    progresshub_progress::DownloadStatus::Partial { .. } => "partial",
                    progresshub_progress::DownloadStatus::Failed { .. } => "failed",
                };
                tracing::debug!(
                    "✅ Model: {} | status: {} | files: {}/{} | bytes: {} / {}",
                    model.model_id,
                    status,
                    model.files_downloaded,
                    model.files_expected,
                    model.total_downloaded_bytes,
                    model.total_expected_bytes,
                );
            }
        }
        Ok(())
    }

    /// Calculate aggregate statistics across all results
    /// Returns: (total_files, completed_files, total_expected_bytes, total_downloaded_bytes)
    pub fn calculate_download_statistics(
        &self,
        results: &Vec<DownloadResult>,
    ) -> (usize, usize, u64, u64) {
        let mut total_files: usize = 0;
        let mut completed_files: usize = 0;
        let mut total_expected_bytes: u64 = 0;
        let mut total_downloaded_bytes: u64 = 0;

        for result in results {
            total_expected_bytes = total_expected_bytes.saturating_add(result.total_expected_bytes);
            total_downloaded_bytes =
                total_downloaded_bytes.saturating_add(result.total_downloaded_bytes);

            for model in result.models.iter() {
                total_files = total_files.saturating_add(model.files_expected);
                completed_files = completed_files.saturating_add(model.files_downloaded);
            }
        }

        (
            total_files,
            completed_files,
            total_expected_bytes,
            total_downloaded_bytes,
        )
    }

    /// Validate that all downloads are complete and reconciled across all results
    pub fn validate_download_completeness(&self, results: &Vec<DownloadResult>) -> bool {
        for result in results {
            if !result.fully_reconciled {
                return false;
            }
            for model in result.models.iter() {
                // Must be complete and file counts match
                match &model.status {
                    progresshub_progress::DownloadStatus::Complete => {}
                    _ => return false,
                }
                if model.files_downloaded != model.files_expected {
                    return false;
                }
                if model.total_downloaded_bytes != model.total_expected_bytes {
                    return false;
                }
            }
        }
        true
    }
    /// Create new processor with zero allocation
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Process download results using ProgressCalculator events ONLY
    #[inline]
    pub fn process_results(&self, progress_calculator: &ProgressCalculator) {
        // All formatting comes from ProgressCalculator accessor methods ONLY
        tracing::debug!("🎉 Download Complete!");
        tracing::debug!(
            "📊 Final Progress: {}",
            progress_calculator.percentage_formatted()
        );
        tracing::debug!(
            "📈 Total Downloaded: {}",
            progress_calculator.bytes_formatted()
        );
        tracing::debug!(
            "🚀 Average Speed: {}",
            progress_calculator.speed_formatted()
        );

        // Display completion summary using accessor methods ONLY
        let model_progress = progress_calculator.get_model_progress();
        if model_progress.len() > 1 {
            tracing::debug!("📦 Models Completed: {} models", model_progress.len());
        }

        // Blazing-fast completion display with elegant formatting
    }

    /// Display live progress updates with zero allocation
    #[inline]
    pub fn display_live_progress(&self, progress_calculator: &ProgressCalculator) {
        // Real-time progress display using ONLY ProgressCalculator methods
        // NOTE: This method violates termcolor architecture and should be migrated
        tracing::debug!(
            "📊 {} | {} | 🚀 {}",
            progress_calculator.percentage_formatted(),
            progress_calculator.bytes_formatted(),
            progress_calculator.speed_formatted()
        );
    }

    /// Display model-specific progress with elegant formatting
    #[inline]
    pub fn display_model_progress(&self, progress_calculator: &ProgressCalculator) {
        let model_progress = progress_calculator.get_model_progress();

        for model in model_progress {
            tracing::debug!(
                "🤖 Model: {} | {} | {}",
                model.model_id,
                progress_calculator.percentage_formatted(),
                progress_calculator.bytes_formatted()
            );
        }
    }

    /// Display completion summary using ONLY ProgressCalculator accessor methods
    #[inline]
    pub fn display_completion_summary(&self, progress_calculator: &ProgressCalculator) {
        // PURE EVENT-DRIVEN: Use ONLY ProgressCalculator accessor methods
        tracing::debug!("🎉 Download Summary:");
        tracing::debug!(
            "📊 Progress: {}",
            progress_calculator.percentage_formatted()
        );
        tracing::debug!("📈 Downloaded: {}", progress_calculator.bytes_formatted());
        tracing::debug!("🚀 Speed: {}", progress_calculator.speed_formatted());

        if !progress_calculator.eta_formatted().is_empty() {
            tracing::debug!("⏱️  ETA: {}", progress_calculator.eta_formatted());
        }
    }
}

impl Default for DownloadResultProcessor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
