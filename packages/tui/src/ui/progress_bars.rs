//! Zero-Allocation Progress Bar Data Structures
//!
//! This module provides data structures for progress tracking with strict
//! zero-allocation constraints and blazing-fast performance.

use std::time::Instant;

/// Zero-allocation progress data structure
#[derive(Debug, Clone, Copy)]
pub struct ProgressData {
    /// Current progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Download speed in bytes per second
    pub speed: f64,
    /// Current status of the download
    pub status: ProgressStatus,
    /// Total size in bytes
    pub total_size: u64,
    /// Downloaded size in bytes
    pub downloaded_size: u64,
    /// Time when download started
    pub start_time: Instant,
}

/// Atomic progress status enum (Copy for zero allocation)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressStatus {
    /// Download is queued but not started
    Queued,
    /// Download is starting up
    Starting,
    /// Download is actively progressing
    Active,
    /// Download is completing
    Completing,
    /// Download completed successfully
    Complete,
    /// Download paused
    Paused,
    /// Download failed (simplified error state)
    Error,
}

/// Progress bar type for effect selection (Copy for zero allocation)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressBarType {
    /// Animated flow of data chunks moving left-to-right
    Flow,
    /// Rhythmic pulsing with intensity matching download speed
    Pulse,
    /// Sine wave effects that ripple through the progress
    Wave,
    /// Smooth color transitions from cold to hot
    Gradient,
    /// Discrete particles representing data packets
    Particle,
    /// Special completion effects with sparkles and bursts
    Celebration,
}

impl Default for ProgressData {
    fn default() -> Self {
        Self {
            progress: 0.0,
            speed: 0.0,
            status: ProgressStatus::Queued,
            total_size: 0,
            downloaded_size: 0,
            start_time: Instant::now(),
        }
    }
}

impl ProgressData {
    /// Create new progress data with validation
    #[inline]
    pub fn new(
        progress: f32,
        speed: f64,
        status: ProgressStatus,
        total_size: u64,
        downloaded_size: u64,
        start_time: Instant,
    ) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            speed,
            status,
            total_size,
            downloaded_size: downloaded_size.min(total_size),
            start_time,
        }
    }

    /// Get elapsed time since start
    #[inline]
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Calculate ETA based on current progress and speed
    #[inline]
    pub fn eta_seconds(&self) -> Option<u64> {
        if self.speed == 0.0 || self.progress >= 1.0 {
            return None;
        }

        let remaining_bytes = self.total_size.saturating_sub(self.downloaded_size);
        Some((remaining_bytes as f64 / self.speed) as u64)
    }

    /// Check if download is active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            ProgressStatus::Active | ProgressStatus::Starting | ProgressStatus::Completing
        )
    }

    /// Check if download is complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.status == ProgressStatus::Complete || self.progress >= 1.0
    }

    /// Get normalized speed (0.0 to 1.0) for effect intensity
    #[inline]
    pub fn normalized_speed(&self) -> f32 {
        // Normalize to typical download speeds (10MB/s = 1.0)
        (self.speed as f32 / 10_000_000.0).min(1.0)
    }
}

impl ProgressStatus {
    /// Check if status represents an active state
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Starting | Self::Completing)
    }

    /// Check if status represents completion
    #[inline]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Check if status represents an error state
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }
}

impl Default for ProgressBarType {
    fn default() -> Self {
        Self::Flow
    }
}

impl ProgressBarType {
    /// Get effect duration for this bar type
    #[inline]
    pub fn effect_duration_ms(&self) -> u64 {
        match self {
            Self::Flow => 1000,
            Self::Pulse => 800,
            Self::Wave => 1200,
            Self::Gradient => 2000,
            Self::Particle => 600,
            Self::Celebration => 3000,
        }
    }

    /// Check if this bar type supports looping effects
    #[inline]
    pub fn supports_loop(&self) -> bool {
        matches!(self, Self::Flow | Self::Pulse | Self::Wave | Self::Particle)
    }

    /// Get base effect intensity for this bar type
    #[inline]
    pub fn base_intensity(&self) -> f32 {
        match self {
            Self::Flow => 0.6,
            Self::Pulse => 0.8,
            Self::Wave => 0.5,
            Self::Gradient => 0.4,
            Self::Particle => 0.7,
            Self::Celebration => 1.0,
        }
    }
}
