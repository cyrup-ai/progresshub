//! TUI state management for download progress.
//!
//! This module defines the core data structures for tracking the state of
//! multiple concurrent downloads in a thread-safe, lock-free manner.

use std::collections::HashMap;
use std::time::{Duration, Instant};
// NOTE: Removed calculate_percentage import - violates Pure Flume Channel Event-Driven Architecture
// All calculations must be in ProgressCalculator (progress crate)

// Represents the current status of a download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
}

// Identifies the backend used for a download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    Quic,
    Xet,
}

// Represents a progress update for a single download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub files_completed: usize,
    pub total_files: usize,
    pub download_speed: f64, // bytes per second
    pub eta: Duration,
    pub status: DownloadStatus,
    pub backend: BackendKind,
    pub last_update: Instant,
    pub last_bytes: u64,
}

impl DownloadProgress {
    pub fn new(model_id: String, backend: BackendKind) -> Self {
        Self {
            model_id,
            bytes_downloaded: 0,
            total_bytes: None,
            files_completed: 0,
            total_files: 0,
            download_speed: 0.0,
            eta: Duration::from_secs(0),
            status: DownloadStatus::Pending,
            backend,
            last_update: Instant::now(),
            last_bytes: 0,
        }
    }

    // NOTE: Percentage calculation removed - violates Pure Flume Channel Event-Driven Architecture
    // TUI should receive ProgressCalculator snapshots and call .percentage_formatted() accessor method
    pub fn get_raw_progress_data(&self) -> (u64, Option<u64>) {
        (self.bytes_downloaded, self.total_bytes)
    }
}

// Represents an error that occurred during a download.
#[derive(Debug, Clone)]
pub struct DownloadError {
    pub model_id: String,
    pub message: String,
    pub timestamp: Instant,
}

// Holds aggregate statistics for all active downloads.
#[derive(Debug, Clone)]
pub struct OverallStats {
    pub total_bytes_downloaded: u64,
    pub overall_speed: f64,
    pub completed_downloads: usize,
    pub total_downloads: usize,
}

impl Default for OverallStats {
    fn default() -> Self {
        Self {
            total_bytes_downloaded: 0,
            overall_speed: 0.0,
            completed_downloads: 0,
            total_downloads: 0,
        }
    }
}

// The central state for the download TUI.
#[derive(Debug, Clone, Default)]
pub struct DownloadState {
    pub downloads: HashMap<String, DownloadProgress>,
    pub overall_stats: OverallStats,
    pub errors: Vec<DownloadError>,
    pub selected_index: usize,
}

impl DownloadState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_download(&mut self, model_id: String, backend: BackendKind) {
        self.downloads.insert(model_id.clone(), DownloadProgress::new(model_id, backend));
        self.recalculate_stats();
    }

    pub fn update_progress(&mut self, model_id: &str, bytes: u64, total: Option<u64>) {
        if let Some(progress) = self.downloads.get_mut(model_id) {
            let now = Instant::now();
            let time_diff = now.duration_since(progress.last_update).as_secs_f64();

            if time_diff > 0.5 { // Update speed calculation periodically
                let bytes_diff = bytes.saturating_sub(progress.last_bytes);
                let current_speed = bytes_diff as f64 / time_diff;
                // Exponential moving average for smooth speed
                progress.download_speed = progress.download_speed * 0.7 + current_speed * 0.3;
                progress.last_update = now;
                progress.last_bytes = bytes;
            }

            progress.bytes_downloaded = bytes;
            progress.total_bytes = total;
            progress.status = DownloadStatus::Downloading;

            if progress.download_speed > 0.0 {
                if let Some(total_bytes) = progress.total_bytes {
                    let remaining_bytes = total_bytes.saturating_sub(progress.bytes_downloaded);
                    progress.eta = Duration::from_secs_f64(remaining_bytes as f64 / progress.download_speed);
                } else {
                    progress.eta = Duration::from_secs(0);
                }
            } else {
                progress.eta = Duration::from_secs(0);
            }
        }
        self.recalculate_stats();
    }

    pub fn complete_download(&mut self, model_id: &str) {
        if let Some(progress) = self.downloads.get_mut(model_id) {
            progress.status = DownloadStatus::Completed;
            progress.download_speed = 0.0;
            progress.eta = Duration::from_secs(0);
        }
        self.recalculate_stats();
    }

    pub fn fail_download(&mut self, model_id: &str, error_message: String) {
        if let Some(progress) = self.downloads.get_mut(model_id) {
            progress.status = DownloadStatus::Failed;
        }
        self.errors.push(DownloadError {
            model_id: model_id.to_string(),
            message: error_message,
            timestamp: Instant::now(),
        });
        self.recalculate_stats();
    }

    fn recalculate_stats(&mut self) {
        self.overall_stats.total_bytes_downloaded = self.downloads.values().map(|p| p.bytes_downloaded).sum();
        self.overall_stats.overall_speed = self.downloads.values().map(|p| p.download_speed).sum();
        self.overall_stats.completed_downloads = self.downloads.values().filter(|p| p.status == DownloadStatus::Completed).count();
        self.overall_stats.total_downloads = self.downloads.len();
    }

    pub fn scroll_up(&mut self) {
        if !self.downloads.is_empty() {
            self.selected_index = self.selected_index.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if !self.downloads.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.downloads.len() - 1);
        }
    }

    pub fn clear_completed_downloads(&mut self) {
        self.downloads.retain(|_, p| p.status != DownloadStatus::Completed);
        self.recalculate_stats();
    }
}
