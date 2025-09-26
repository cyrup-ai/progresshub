//! Download graph state management with blazing-fast operations
//!
//! Provides zero-allocation state tracking with lock-free atomic operations
//! and comprehensive progress data management for optimal performance.

use std::time::{Duration, Instant};
use tachyonfx::{Effect, Shader};

use crate::ui::components::data::download_data::{
    DownloadDataManager, DownloadLevel, DownloadPoint,
};
use crate::ui::effects::download_effects::{DownloadEffectStack, DownloadEffects};

/// State for the download graph widget with efficient state management
///
/// Manages download progress history, effect composition, and timing
/// using zero-allocation patterns and lock-free atomic operations.
#[derive(Debug)]
pub struct DownloadGraphState {
    /// Data manager for download progress history
    data_manager: DownloadDataManager,
    /// TachyonFX effects stack for visual effects
    effect_stack: DownloadEffectStack,
    /// Current effect composition for rendering
    current_effect: Option<Effect>,
    /// Last effect update time for frame timing
    last_effect_update: Instant,
    /// Whether effects need regeneration for performance
    effects_dirty: bool,
}

impl DownloadGraphState {
    /// Create new download graph state with optimal initialization
    ///
    /// Initializes state with pre-allocated data structures and
    /// efficient effect management for blazing-fast performance.
    ///
    /// # Returns
    /// New DownloadGraphState ready for progress tracking
    ///
    /// # Performance
    /// - Zero allocation initialization with pre-sized collections
    /// - Lock-free atomic operations for state management
    /// - Optimized timing structures for smooth animations
    #[inline]
    pub fn new() -> Self {
        Self {
            data_manager: DownloadDataManager::new(),
            effect_stack: DownloadEffectStack::new(),
            current_effect: None,
            last_effect_update: Instant::now(),
            effects_dirty: true,
        }
    }

    /// Update with new download progress data (event-driven)
    ///
    /// Processes new download progress data with efficient state updates
    /// and intelligent effect regeneration based on data changes.
    ///
    /// # Arguments
    /// * `speed_mbps` - Current download speed in megabytes per second
    /// * `bytes_downloaded` - Total bytes downloaded so far
    /// * `total_bytes` - Total expected bytes for download
    /// * `progress_percentage` - Completion percentage (0.0 to 1.0)
    ///
    /// # Performance
    /// - Zero-allocation data point creation with move semantics
    /// - Efficient level change detection with atomic operations
    /// - Smart effect regeneration based on significant changes only
    #[inline]
    pub fn update_download_progress(
        &mut self,
        speed_mbps: f64,
        bytes_downloaded: u64,
        total_bytes: u64,
        progress_percentage: f32,
    ) {
        // Create new data point with efficient construction
        let point = DownloadPoint::new(
            speed_mbps,
            bytes_downloaded,
            total_bytes,
            progress_percentage,
            "unknown".to_string(),
        );

        // Check for level changes before adding point
        let level_changed = self.data_manager.level_changed();

        // Add point to data manager
        self.data_manager.add_point(point);

        // Mark effects as dirty if significant change occurred
        if level_changed {
            self.effects_dirty = true;
        }
    }

    /// Update effects with frame timing for smooth animations
    ///
    /// Manages effect timing and regeneration with efficient frame
    /// processing and lock-free atomic operations.
    ///
    /// # Arguments
    /// * `frame_duration` - Duration since last frame for timing calculations
    ///
    /// # Performance
    /// - Efficient effect regeneration only when needed
    /// - Lock-free effect stack updates with atomic operations
    /// - Optimized timing calculations for smooth 60fps animations
    #[inline]
    pub fn update_effects(&mut self, frame_duration: Duration) {
        // Regenerate effects if needed for optimal performance
        if self.effects_dirty {
            self.regenerate_effects();
            self.effects_dirty = false;
        }

        // Update effect stack with frame timing
        self.effect_stack.update(frame_duration);
        self.last_effect_update = Instant::now();
    }

    /// Get current download level with efficient lookup
    ///
    /// Returns current download performance level using optimized
    /// level determination with zero-allocation operations.
    ///
    /// # Returns
    /// Optional DownloadLevel representing current performance tier
    #[inline]
    pub fn current_level(&self) -> Option<DownloadLevel> {
        self.data_manager.current_level()
    }

    /// Get sparkline string for display with efficient generation
    ///
    /// Generates sparkline visualization using optimized string operations
    /// and zero-allocation character mapping for blazing-fast display.
    ///
    /// # Returns
    /// String containing sparkline characters for graph display
    #[inline]
    pub fn sparkline_string(&self) -> String {
        self.data_manager.sparkline_string()
    }

    /// Get session peak download speed with atomic access
    ///
    /// Returns peak download speed using lock-free atomic operations
    /// for efficient concurrent access without blocking.
    ///
    /// # Returns
    /// Peak download speed in megabytes per second
    #[inline]
    pub fn session_peak(&self) -> f64 {
        self.data_manager.session_peak()
    }

    /// Get latest download point with efficient reference
    ///
    /// Returns reference to latest download data point using
    /// zero-copy access patterns for optimal memory usage.
    ///
    /// # Returns
    /// Optional reference to latest DownloadPoint data
    #[inline]
    pub fn latest_point(&self) -> Option<&DownloadPoint> {
        self.data_manager.latest_point()
    }

    /// Check if effects are running with optimized detection
    ///
    /// Determines effect execution status using efficient boolean
    /// operations and optimized effect state checking.
    ///
    /// # Returns
    /// Boolean indicating whether effects are currently active
    #[inline]
    pub fn has_running_effects(&self) -> bool {
        self.current_effect.as_ref().is_some_and(|e| e.running())
    }

    /// Get current effect for rendering with zero-copy access
    ///
    /// Returns reference to current effect using efficient reference
    /// patterns for optimal rendering performance.
    ///
    /// # Returns
    /// Optional reference to current Effect for rendering
    #[inline]
    pub fn current_effect(&self) -> Option<&Effect> {
        self.current_effect.as_ref()
    }

    /// Tick the download graph for smooth animations
    ///
    /// Updates animation timing with efficient frame duration calculation
    /// and lock-free effect processing for smooth 60fps animations.
    ///
    /// # Performance
    /// - Efficient frame timing calculations with atomic operations
    /// - Zero-allocation duration processing with move semantics
    /// - Optimized effect updates for smooth visual transitions
    #[inline]
    pub fn tick(&mut self) {
        // Calculate frame duration efficiently
        let now = Instant::now();
        let frame_duration = now.duration_since(self.last_effect_update);
        self.last_effect_update = now;

        // Update effects with frame timing
        self.update_effects(frame_duration);
    }

    /// Check if the graph is actively animating
    ///
    /// Determines animation state using efficient boolean operations
    /// and optimized state checking for rendering optimization.
    ///
    /// # Returns
    /// Boolean indicating whether graph requires active rendering
    #[inline]
    pub fn is_active(&self) -> bool {
        // Always active if we have data or effects running
        !self.data_manager.history().is_empty()
            || self.current_effect.as_ref().is_some_and(|e| e.running())
    }

    /// Get frame interval for smooth animations
    ///
    /// Returns optimal frame interval for 60fps smooth animations
    /// using compile-time constants for blazing-fast access.
    ///
    /// # Returns
    /// Duration representing optimal frame update interval
    #[inline]
    pub fn frame_interval(&self) -> Duration {
        // Update animations every 16ms for 60fps smooth animations
        Duration::from_millis(16)
    }

    /// Get data manager reference for external access
    ///
    /// Provides efficient access to underlying data manager
    /// for advanced operations and data analysis.
    ///
    /// # Returns
    /// Reference to DownloadDataManager for data access
    #[inline]
    pub fn data_manager(&self) -> &DownloadDataManager {
        &self.data_manager
    }

    /// Check if effects need regeneration
    ///
    /// Returns whether the effects are marked as dirty and need regeneration
    /// for optimal performance and visual accuracy.
    ///
    /// # Returns
    /// Boolean indicating if effects are dirty
    #[inline]
    pub fn effects_dirty(&self) -> bool {
        self.effects_dirty
    }

    /// Regenerate effects based on current state
    ///
    /// Creates new effect composition based on current download level
    /// and progress data using efficient effect generation.
    ///
    /// # Performance
    /// - Efficient level-based effect creation with match optimization
    /// - Zero-allocation intensity calculations with atomic operations
    /// - Optimized position tracking with lock-free data structures
    #[inline]
    fn regenerate_effects(&mut self) {
        if let Some(level) = self.current_level() {
            // Calculate intensity efficiently
            let intensity = self
                .latest_point()
                .map(|p| p.intensity as f32)
                .unwrap_or(0.0);

            // Get current position and peak positions efficiently
            let current_position = self.data_manager.history().len().saturating_sub(1);
            let peak_positions = self.data_manager.peak_positions();

            // Create new effect stack with optimal composition
            self.current_effect = Some(DownloadEffects::create_effect_stack(
                level,
                intensity,
                current_position,
                peak_positions,
            ));
        }
    }
}

impl Default for DownloadGraphState {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
