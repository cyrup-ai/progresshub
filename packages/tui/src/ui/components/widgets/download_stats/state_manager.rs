//! State management for download statistics component.
//!
//! PURE EVENT-DRIVEN: Consumes ONLY bandwidth and ProgressCalculator event streams.
//! NO CACHED STATE, NO COMPLEX LOGIC - JUST EVENT STREAM CONSUMPTION.

use super::types::DownloadStatsComponent;

/// Legacy type for compatibility
pub struct DownloadPoint {
    pub speed_mbps: f64,
}

impl DownloadStatsComponent {
    /// Update from ProgressCalculator event stream
    pub fn update_from_progress_calculator(
        &mut self,
        progress_calculator: &progresshub_progress::ProgressCalculator,
    ) {
        self.current_progress_calculator = Some(progress_calculator.clone());
    }

    // bandwidth updates handled in types.rs with proper bytes/s to MB/s conversion

    /// Legacy method for compatibility - delegates to ProgressCalculator events
    pub fn update_download_stats(&mut self, _point: &DownloadPoint) {
        // This is now a no-op since we only use ProgressCalculator events
    }

    /// Get bytes formatted string from ProgressCalculator event stream ONLY
    pub fn bytes_formatted(&self) -> String {
        self.current_progress_calculator
            .as_ref()
            .map(|pc| pc.bytes_formatted())
            .unwrap_or_else(|| "0 B / 0 B".to_string())
    }

    /// Get percentage formatted string from ProgressCalculator event stream ONLY  
    pub fn percentage_formatted(&self) -> String {
        self.current_progress_calculator
            .as_ref()
            .map(|pc| pc.percentage_formatted())
            .unwrap_or_else(|| "0%".to_string())
    }

    /// Get speed formatted string from ProgressCalculator event stream ONLY
    pub fn speed_formatted(&self) -> String {
        self.current_progress_calculator
            .as_ref()
            .map(|pc| pc.speed_formatted())
            .unwrap_or_else(|| "0 MB/s".to_string())
    }

    /// Legacy rendering method for Widget trait compatibility
    pub fn render_to_buffer(
        &self,
        _area: ratatui::layout::Rect,
        _buf: &mut ratatui::buffer::Buffer,
    ) {
        // No-op - pure event-driven component doesn't need buffer rendering
    }
}
