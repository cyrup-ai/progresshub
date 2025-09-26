//! Pure display management with receiver pattern
//!
//! Provides pure display logic consuming ProgressCalculator events from flume channels
//! with zero business logic and beautiful hierarchical rendering.

use crate::display::progress_renderer::HierarchicalProgressRenderer;
use anyhow::Result;
use progresshub_progress::ProgressCalculator;

/// Run CLI table display with pure receiver pattern
///
/// Consumes ProgressCalculator events from a flume receiver and displays them
/// using the beautiful HierarchicalProgressRenderer until downloads are complete.
/// This function has ZERO business logic - pure display only.
///
/// # Arguments
/// * `progress_receiver` - Flume receiver for ProgressCalculator events
///
/// # Returns
/// Result indicating success or rendering error
///
/// # Architecture
/// - Pure display logic with ZERO calculations
/// - Uses HierarchicalProgressRenderer for sweet hierarchical layout
/// - Consumes ProgressCalculator events until is_complete() is true
/// - Beautiful tree structure: Overall → Models → Files with progress bars
pub async fn run_cli_table_display_with_receiver(
    progress_receiver: flume::Receiver<ProgressCalculator>,
) -> Result<()> {
    tracing::debug!("Starting CLI table display with receiver pattern");

    // Initialize the beautiful hierarchical progress renderer
    let mut renderer = HierarchicalProgressRenderer::new();

    // Pure receiver pattern - consume ProgressCalculator events until complete
    while let Ok(progress) = progress_receiver.recv_async().await {
        // Beautiful hierarchical display - the "sweet hierarchical layout"
        if let Err(e) = renderer.render_progress(&progress) {
            tracing::warn!("Progress rendering failed: {}", e);
            // Continue processing other events even if one rendering fails
        }

        // Check if downloads are complete
        if progress.is_complete() {
            tracing::info!("Downloads completed - CLI display shutting down");
            break;
        }
    }

    tracing::info!("CLI table display receiver mode completed successfully");
    Ok(())
}
