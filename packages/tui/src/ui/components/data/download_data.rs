//! Download data management for progress tracking and visualization.
//!
//! This module provides comprehensive data structures and management for download progress,
//! including historical tracking, peak detection, and sparkline generation.

use std::collections::VecDeque;
use std::time::Instant;

/// Download speed classification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadLevel {
    /// Low download speeds (< 10 MB/s)
    Low,
    /// Medium download speeds (10-50 MB/s)
    Medium,
    /// High download speeds (50-100 MB/s)
    High,
    /// Critical/excellent download speeds (> 100 MB/s)
    Critical,
}

impl DownloadLevel {
    /// Determine download level from speed in MB/s
    #[inline]
    pub fn from_speed(speed_mbps: f64) -> Self {
        if speed_mbps > 100.0 {
            Self::Critical
        } else if speed_mbps > 50.0 {
            Self::High
        } else if speed_mbps > 10.0 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Get color for this download level
    #[inline]
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            Self::Low => ratatui::style::Color::Blue,
            Self::Medium => ratatui::style::Color::Cyan,
            Self::High => ratatui::style::Color::Yellow,
            Self::Critical => ratatui::style::Color::Red,
        }
    }
}

/// A single download progress data point
#[derive(Debug, Clone)]
pub struct DownloadPoint {
    /// Download speed in MB/s
    pub speed_mbps: f64,
    /// Bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total bytes to download
    pub total_bytes: u64,
    /// Progress percentage (0.0 to 1.0)
    pub progress_percentage: f32,
    /// Download intensity for effects (0.0 to 1.0)
    pub intensity: f64,
    /// Timestamp when this point was recorded
    pub timestamp: Instant,
    /// File path being downloaded
    pub file_path: String,
}

impl DownloadPoint {
    /// Create a new download point
    #[inline]
    pub fn new(
        speed_mbps: f64,
        bytes_downloaded: u64,
        total_bytes: u64,
        progress_percentage: f32,
        file_path: String,
    ) -> Self {
        // Calculate intensity based on speed (normalized to typical download speeds)
        let intensity = (speed_mbps / 100.0).clamp(0.0, 1.0);

        Self {
            speed_mbps,
            bytes_downloaded,
            total_bytes,
            progress_percentage,
            intensity,
            timestamp: Instant::now(),
            file_path,
        }
    }

    /// Get download level for this point
    #[inline]
    pub fn level(&self) -> DownloadLevel {
        DownloadLevel::from_speed(self.speed_mbps)
    }

    /// Check if this is a peak point (higher than surrounding points)
    #[inline]
    pub fn is_peak(&self, threshold: f64) -> bool {
        self.speed_mbps > threshold
    }
}

/// Manages download progress data with history tracking and analysis
#[derive(Debug)]
pub struct DownloadDataManager {
    /// Historical download points (circular buffer, max 120 points = 2 minutes)
    history: VecDeque<DownloadPoint>,
    /// Current download level
    current_level: Option<DownloadLevel>,
    /// Previous download level for change detection
    previous_level: Option<DownloadLevel>,
    /// Session peak download speed
    session_peak: f64,
    /// Peak positions in history for visual markers
    peak_positions: Vec<usize>,
    /// Maximum history size
    max_history_size: usize,
}

impl DownloadDataManager {
    /// Create a new download data manager
    #[inline]
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(120),
            current_level: None,
            previous_level: None,
            session_peak: 0.0,
            peak_positions: Vec::new(),
            max_history_size: 120,
        }
    }

    /// Add a new download progress point
    #[inline]
    pub fn add_point(&mut self, point: DownloadPoint) {
        // Update session peak
        if point.speed_mbps > self.session_peak {
            self.session_peak = point.speed_mbps;
        }

        // Update level tracking
        self.previous_level = self.current_level;
        self.current_level = Some(point.level());

        // Add to history with circular buffer behavior
        if self.history.len() >= self.max_history_size {
            self.history.pop_front();
        }
        self.history.push_back(point);

        // Update peak positions
        self.update_peak_positions();
    }

    /// Check if download level changed since last update
    #[inline]
    pub fn level_changed(&self) -> bool {
        self.current_level != self.previous_level
    }

    /// Get current download level
    #[inline]
    pub fn current_level(&self) -> Option<DownloadLevel> {
        self.current_level
    }

    /// Get session peak download speed
    #[inline]
    pub fn session_peak(&self) -> f64 {
        self.session_peak
    }

    /// Get latest download point
    #[inline]
    pub fn latest_point(&self) -> Option<&DownloadPoint> {
        self.history.back()
    }

    /// Get reference to history for analysis
    #[inline]
    pub fn history(&self) -> &VecDeque<DownloadPoint> {
        &self.history
    }

    /// Get positions of peak download speeds
    #[inline]
    pub fn peak_positions(&self) -> &[usize] {
        &self.peak_positions
    }

    /// Generate sparkline string for terminal display
    #[inline]
    pub fn sparkline_string(&self) -> String {
        if self.history.is_empty() {
            return " ".repeat(60);
        }

        // Use Unicode block characters for sparkline
        const SPARKLINE_CHARS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        // Find min and max for normalization
        let speeds: Vec<f64> = self.history.iter().map(|p| p.speed_mbps).collect();
        let min_speed = speeds.iter().copied().fold(f64::INFINITY, f64::min);
        let max_speed = speeds.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let range = if max_speed > min_speed {
            max_speed - min_speed
        } else {
            1.0 // Avoid division by zero
        };

        // Generate sparkline characters
        let mut sparkline = String::with_capacity(self.history.len());
        for point in &self.history {
            let normalized = if range > 0.0 {
                ((point.speed_mbps - min_speed) / range).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let char_index = (normalized * (SPARKLINE_CHARS.len() - 1) as f64) as usize;
            sparkline.push(SPARKLINE_CHARS[char_index.min(SPARKLINE_CHARS.len() - 1)]);
        }

        // Pad to consistent width
        let target_width = 60;
        if sparkline.len() < target_width {
            sparkline.push_str(&" ".repeat(target_width - sparkline.len()));
            sparkline
        } else {
            sparkline
        }
    }

    /// Update peak positions based on current history
    #[inline]
    fn update_peak_positions(&mut self) {
        self.peak_positions.clear();

        if self.history.len() < 3 {
            return;
        }

        // Calculate dynamic threshold based on average and variance
        let speeds: Vec<f64> = self.history.iter().map(|p| p.speed_mbps).collect();
        let average = speeds.iter().sum::<f64>() / speeds.len() as f64;

        // Calculate standard deviation
        let variance: f64 = speeds
            .iter()
            .map(|&speed| {
                let diff = speed - average;
                diff * diff
            })
            .sum::<f64>()
            / speeds.len() as f64;

        let std_dev = variance.sqrt();
        let peak_threshold = average + std_dev;

        // Find peaks (local maxima above threshold)
        for (i, point) in self.history.iter().enumerate() {
            if point.speed_mbps > peak_threshold && i > 0 && i < self.history.len() - 1 {
                // Check if it's a local maximum
                let prev_speed = self.history[i - 1].speed_mbps;
                let next_speed = self.history[i + 1].speed_mbps;

                if point.speed_mbps >= prev_speed && point.speed_mbps >= next_speed {
                    self.peak_positions.push(i);
                }
            }
        }
    }

    /// Calculate download efficiency based on speed consistency
    #[inline]
    pub fn calculate_efficiency(&self) -> f32 {
        if self.history.len() < 2 {
            return 1.0;
        }

        let speeds: Vec<f64> = self.history.iter().map(|p| p.speed_mbps).collect();
        let average = speeds.iter().sum::<f64>() / speeds.len() as f64;

        if average == 0.0 {
            return 0.0;
        }

        // Calculate coefficient of variation (lower is more efficient)
        let variance: f64 = speeds
            .iter()
            .map(|&speed| {
                let diff = speed - average;
                diff * diff
            })
            .sum::<f64>()
            / speeds.len() as f64;

        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / average;

        // Convert to efficiency score (0.0 to 1.0, higher is better)
        (1.0 / (1.0 + coefficient_of_variation)).min(1.0) as f32
    }

    /// Get average download speed over the session
    #[inline]
    pub fn average_speed(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.history.iter().map(|p| p.speed_mbps).sum();
        sum / self.history.len() as f64
    }

    /// Get download trend (positive = increasing, negative = decreasing, 0 = stable)
    #[inline]
    pub fn speed_trend(&self) -> f32 {
        if self.history.len() < 10 {
            return 0.0;
        }

        // Compare recent speeds to earlier speeds
        let recent_count = self.history.len().min(5);
        let earlier_count = self.history.len().min(10) - recent_count;

        if earlier_count == 0 {
            return 0.0;
        }

        let recent_avg: f64 = self
            .history
            .iter()
            .rev()
            .take(recent_count)
            .map(|p| p.speed_mbps)
            .sum::<f64>()
            / recent_count as f64;

        let earlier_avg: f64 = self
            .history
            .iter()
            .rev()
            .skip(recent_count)
            .take(earlier_count)
            .map(|p| p.speed_mbps)
            .sum::<f64>()
            / earlier_count as f64;

        if earlier_avg == 0.0 {
            return 0.0;
        }

        ((recent_avg - earlier_avg) / earlier_avg).clamp(-1.0, 1.0) as f32
    }

    /// Clear all historical data
    #[inline]
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_level = None;
        self.previous_level = None;
        self.session_peak = 0.0;
        self.peak_positions.clear();
    }

    /// Get estimated time remaining based on current speed and progress
    #[inline]
    pub fn estimated_time_remaining(&self) -> Option<std::time::Duration> {
        let latest = self.latest_point()?;

        if latest.speed_mbps <= 0.0 || latest.total_bytes <= latest.bytes_downloaded {
            return None;
        }

        let remaining_bytes = latest.total_bytes - latest.bytes_downloaded;
        let speed_bytes_per_sec = latest.speed_mbps * 1_000_000.0; // MB/s to bytes/s
        let eta_seconds = remaining_bytes as f64 / speed_bytes_per_sec;

        Some(std::time::Duration::from_secs(eta_seconds as u64))
    }
}

impl Default for DownloadDataManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
