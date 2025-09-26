//! Core App struct and initialization with zero-allocation patterns
//!
//! Blazing-fast application core with sophisticated state management,
//! magical effects integration, and elegant ergonomic initialization.

use tokio::sync::oneshot;

use crate::ui::{
    components::widgets::{DownloadStatsComponent, ModelListState},
    progress_bars::ProgressBarType,
    progress_renderer::{ProgressColorScheme, ProgressRenderer, ProgressRendererConfig},
    state::AppState,
    theme::CyrupTheme,
};

use progresshub_progress::ProgressCalculator;

/// Main application with sophisticated progress visualization and magical effects
pub struct App {
    /// Core application state with focus management and completion tracking
    pub(crate) state: AppState,
    /// Stateful widget state for model list navigation and expansion
    pub(crate) model_list_state: ModelListState,
    /// Progress event receiver from download operations
    pub(crate) progress_rx: flume::Receiver<ProgressCalculator>,
    /// Optional completion signal receiver for download finish detection
    pub(crate) done_rx: Option<oneshot::Receiver<()>>,
    /// Magical progress renderer with sophisticated tachyonfx effects
    pub(crate) progress_renderer: ProgressRenderer,
    /// Download statistics component for comprehensive analysis and insights
    pub(crate) download_stats: DownloadStatsComponent,
}

impl App {
    /// Create a new application with pure progress visualization and magical effects.
    ///
    /// Initializes sophisticated progress visualization system with:
    /// - Pure ProgressCalculator event consumption via flume channels
    /// - Magical tachyonfx effects for spectacular progress visualization  
    /// - Intelligent state management with focus navigation
    /// - Zero-allocation data structures for blazing-fast performance
    ///
    /// # Arguments
    /// * `progress_rx` - Channel receiver for ProgressCalculator snapshots from CentralProgressDispatcher
    /// * `done_rx` - One-shot receiver for download completion signal
    ///
    /// # Returns
    /// Fully initialized App with production-ready configuration
    ///
    /// # Architecture
    /// - Pure Flume Channel Event-Driven Architecture
    /// - Zero business logic - only display and UI state management
    /// - ProgressCalculator provides ALL formatted data via accessor methods
    #[inline]
    pub fn new(
        progress_rx: flume::Receiver<ProgressCalculator>,
        done_rx: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            state: AppState::default(),
            model_list_state: ModelListState::default(),
            progress_rx,
            done_rx: Some(done_rx),
            progress_renderer: ProgressRenderer::new(ProgressRendererConfig {
                enable_effects: true,
                default_bar_type: ProgressBarType::Flow,
                animation_intensity: 0.8,
                enable_celebrations: true,
                color_scheme: ProgressColorScheme {
                    queued: CyrupTheme::TEXT_MUTED,
                    starting: CyrupTheme::INFO,
                    active: CyrupTheme::ACCENT,
                    completing: CyrupTheme::WARNING,
                    complete: CyrupTheme::SUCCESS,
                    paused: CyrupTheme::TEXT_SECONDARY,
                    error: CyrupTheme::ERROR,
                    background: CyrupTheme::BG_PRIMARY,
                    border: CyrupTheme::BORDER,
                },
            }),
            download_stats: DownloadStatsComponent::new(),
        }
    }

    /// Check if the user requested a force quit.
    ///
    /// # Returns
    /// True if force quit was requested (Ctrl+C or Ctrl+D)
    #[inline]
    #[must_use]
    pub fn is_force_quit(&self) -> bool {
        self.state.force_quit
    }
}
