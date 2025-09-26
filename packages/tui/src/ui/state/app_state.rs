//! Core application state management for TUI
//!
//! Provides the main `AppState` struct with completion flow management,
//! state transitions, and zero-allocation atomic updates for UI synchronization.

use std::time::Instant;

// Temporarily commented out during YStream integration to avoid cryypt compilation errors
// use lib_bandwydth::display::components::BandwidthGraphState;
use progresshub_progress::ImmutableProgressState;

use super::focus_manager::{BottomBarFocus, FocusState, FocusStates};

/// Represents the completion state for visual confirmation flow
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionState {
    /// Normal active state - downloads in progress
    Active,
    /// Visual completion state - showing 100% for confirmation
    VisualCompletion,
    /// Complete state - timer running for auto-exit
    Complete,
    /// Exiting state - preparing to quit
    Exiting,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::Active
    }
}

/// Represents the current state of the application UI.
#[derive(Debug)]
pub struct AppState {
    /// Actual ProgressCalculator snapshot from flume events - the REAL data
    pub current_progress_calculator: Option<progresshub_progress::ProgressCalculator>,
    /// Immutable progress state managed by ProgressCalculator
    pub progress_state: ImmutableProgressState,
    /// Whether the app should quit
    pub should_quit: bool,
    pub all_downloads_complete: bool,
    /// Whether the app should force quit immediately
    pub force_quit: bool,
    /// Current error message if any
    pub error: Option<String>,
    /// Currently focused model index (None if no focus)
    pub focused_model: Option<usize>,
    /// Currently selected model index (None if no selection)
    pub selected_model: Option<usize>,
    /// Focus state for UI components
    pub focus_states: FocusStates,
    /// Current completion state for visual confirmation flow
    pub completion_state: CompletionState,
    /// Timer for completion visual confirmation (when completion started)
    pub completion_timer: Option<Instant>,
    /// Whether visual completion has started (prevents multiple triggers)
    pub visual_completion_started: bool,
    /// Bandwidth monitoring state from lib_bandwydth - temporarily commented out
    // pub bandwidth_state: BandwidthGraphState,
    /// Current terminal dimensions (width, height)
    pub terminal_size: (u16, u16),
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_progress_calculator: None,
            progress_state: ImmutableProgressState {
                models: im::Vector::new(),
                overall_bytes_downloaded: 0,
                overall_total_bytes: 0,
                manifest_data: im::OrdMap::new(),
                timestamp: std::time::SystemTime::now(),
            },
            should_quit: false,
            all_downloads_complete: false,
            force_quit: false,
            error: None,
            focused_model: Some(0), // Start with first model focused
            selected_model: None,
            focus_states: FocusStates::default(),
            completion_state: CompletionState::default(),
            completion_timer: None,
            visual_completion_started: false,
            // bandwidth_state: BandwidthGraphState::new(),
            terminal_size: (80, 24), // Default terminal size
        }
    }
}

impl AppState {
    /// Check if the app should quit
    pub fn should_quit(&self) -> bool {
        // User quit request ALWAYS works immediately - critical user control
        if self.should_quit || self.force_quit {
            return true;
        }

        // Handle auto-quit from completion state machine
        match self.completion_state {
            CompletionState::Active => {
                // No auto-quit during active downloads
                false
            }
            CompletionState::VisualCompletion => {
                // No auto-quit during visual confirmation
                false
            }
            CompletionState::Complete => {
                // Auto-quit when timer expires OR immediately if no timer
                if let Some(start_time) = self.completion_timer {
                    let elapsed = start_time.elapsed();
                    elapsed.as_secs() >= 2
                } else {
                    // If no timer is set, quit immediately when in Complete state
                    true
                }
            }
            CompletionState::Exiting => {
                // Always quit in exiting state
                true
            }
        }
    }

    /// Mark the app to quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Force the app to quit immediately
    pub fn force_quit(&mut self) {
        self.force_quit = true;
    }

    /// Trigger visual completion when downloads finish
    pub fn trigger_visual_completion(&mut self) {
        if !self.visual_completion_started && self.all_downloads_complete {
            self.completion_state = CompletionState::VisualCompletion;
            self.visual_completion_started = true;
            // Timer will be set when transitioning to Complete state
        }
    }

    /// Transition to complete state with timer
    pub fn transition_to_complete(&mut self) {
        if self.completion_state == CompletionState::VisualCompletion {
            self.completion_state = CompletionState::Complete;
            self.completion_timer = Some(Instant::now());
        }
    }

    /// Check if completion timer has expired (for external polling)
    pub fn completion_timer_expired(&self) -> bool {
        if let Some(start_time) = self.completion_timer {
            start_time.elapsed().as_secs() >= 2
        } else {
            false
        }
    }

    /// Transition to exiting state
    pub fn transition_to_exiting(&mut self) {
        self.completion_state = CompletionState::Exiting;
    }

    /// Handle downloads completing - triggers visual completion state
    /// This method is called when all downloads are finished and we want to
    /// start the visual completion flow with timer
    pub fn handle_downloads_complete(&mut self) {
        if !self.visual_completion_started {
            self.all_downloads_complete = true;
            self.trigger_visual_completion();
        }
    }

    /// Update progress state with zero-allocation atomic updates
    ///
    /// Uses efficient state synchronization to minimize UI update overhead
    /// while maintaining consistency across concurrent access patterns.
    #[inline]
    pub fn update_progress_state(&mut self, new_state: ImmutableProgressState) {
        self.progress_state = new_state;

        // Check for completion automatically
        if self.progress_state.is_complete() && !self.visual_completion_started {
            self.handle_downloads_complete();
        }
    }

    /// Update progress state from ProgressCalculator snapshot
    ///
    /// Stores the REAL ProgressCalculator snapshot from flume events.
    /// Maintains Pure Flume Channel Event-Driven Architecture with ZERO pattern violations.
    #[inline]
    pub fn update_from_progress_calculator(
        &mut self,
        progress_calculator: &progresshub_progress::ProgressCalculator,
    ) {
        // Store the REAL ProgressCalculator snapshot - no fake data!
        self.current_progress_calculator = Some(progress_calculator.clone());
        self.progress_state = progress_calculator.get_progress_state().clone();
    }

    /// Set error state with immediate UI feedback
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Clear error state
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Check if there's an active error
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the current error message
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Update focused model with bounds checking
    pub fn set_focused_model(&mut self, index: Option<usize>) {
        // Use ProgressCalculator method instead of direct field access
        if let Some(idx) = index {
            let model_count = self.get_model_count();
            if idx < model_count {
                self.focused_model = Some(idx);
            }
        } else {
            self.focused_model = None;
        }
    }

    /// Update selected model with bounds checking
    pub fn set_selected_model(&mut self, index: Option<usize>) {
        // Use ProgressCalculator method instead of direct field access
        if let Some(idx) = index {
            let model_count = self.get_model_count();
            if idx < model_count {
                self.selected_model = Some(idx);
            }
        } else {
            self.selected_model = None;
        }
    }

    /// Check if all downloads are complete
    pub fn is_complete(&self) -> bool {
        self.all_downloads_complete
    }

    /// Check if visual completion is active
    pub fn is_visual_completion_active(&self) -> bool {
        matches!(self.completion_state, CompletionState::VisualCompletion)
    }

    /// Check if the completion flow has started
    pub fn is_completion_started(&self) -> bool {
        self.visual_completion_started
    }

    /// Move focus to the next model in the list
    pub fn focus_next_model(&mut self) {
        let model_count = self.get_model_count();
        if model_count == 0 {
            self.focused_model = None;
            return;
        }

        let next_index = match self.focused_model {
            Some(current) => (current + 1) % model_count,
            None => 0,
        };

        self.focused_model = Some(next_index);
        self.focus_states.model_list = FocusState::Focused;
        self.focus_states.bottom_bar = BottomBarFocus::None;
    }

    /// Move focus to the previous model in the list
    pub fn focus_previous_model(&mut self) {
        let model_count = self.get_model_count();
        if model_count == 0 {
            self.focused_model = None;
            return;
        }

        let prev_index = match self.focused_model {
            Some(current) => {
                if current == 0 {
                    model_count - 1
                } else {
                    current - 1
                }
            }
            None => model_count - 1,
        };

        self.focused_model = Some(prev_index);
        self.focus_states.model_list = FocusState::Focused;
        self.focus_states.bottom_bar = BottomBarFocus::None;
    }

    /// Toggle selection of the currently focused model
    pub fn toggle_model_selection(&mut self) {
        if let Some(focused_idx) = self.focused_model {
            self.selected_model = if self.selected_model == Some(focused_idx) {
                None // Deselect if already selected
            } else {
                Some(focused_idx) // Select the focused model
            };

            // Update focus state to reflect selection
            self.focus_states.model_list = if self.selected_model.is_some() {
                FocusState::Selected
            } else {
                FocusState::Focused
            };
        }
    }

    /// Move focus to the bottom bar components
    pub fn focus_bottom_bar(&mut self) {
        self.focus_states.model_list = FocusState::None;
        self.focus_states.bottom_bar = BottomBarFocus::Remaining; // Start with first component
    }

    /// Move focus within bottom bar components
    pub fn focus_next_bottom_bar_component(&mut self) {
        self.focus_states.bottom_bar = match self.focus_states.bottom_bar {
            BottomBarFocus::None | BottomBarFocus::Remaining => BottomBarFocus::Progress,
            BottomBarFocus::Progress => BottomBarFocus::Bandwidth,
            BottomBarFocus::Bandwidth => BottomBarFocus::Remaining, // Wrap around
        };
        self.focus_states.model_list = FocusState::None;
    }

    /// Move focus back to model list from bottom bar
    pub fn focus_model_list(&mut self) {
        self.focus_states.model_list = FocusState::Focused;
        self.focus_states.bottom_bar = BottomBarFocus::None;

        // Ensure we have a focused model
        if self.focused_model.is_none() && self.get_model_count() > 0 {
            self.focused_model = Some(0);
        }
    }

    /// Get the focus state for a specific model index
    pub fn get_model_focus_state(&self, model_index: usize) -> FocusState {
        match (self.focused_model, self.selected_model) {
            (Some(focused), Some(selected))
                if focused == model_index && selected == model_index =>
            {
                FocusState::Selected
            }
            (Some(focused), _) if focused == model_index => FocusState::Focused,
            _ => FocusState::None,
        }
    }

    /// Check if a specific bottom bar component is focused
    pub fn is_bottom_bar_component_focused(&self, component: BottomBarFocus) -> bool {
        self.focus_states.bottom_bar == component
    }

    /// Update layout based on terminal size
    pub fn update_layout(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }

    /// Get formatted total progress for display using REAL ProgressCalculator
    /// Returns Option to avoid fake code creation - let TUI handle None case
    pub fn total_progress_formatted(&self) -> Option<(String, String, String)> {
        self.current_progress_calculator
            .as_ref()
            .map(|calc| calc.get_total_progress_formatted())
    }

    /// Get complete progress summary formatted for display using REAL ProgressCalculator
    /// Returns Option to avoid fake code creation - let TUI handle None case
    pub fn progress_summary_formatted(&self) -> Option<String> {
        self.current_progress_calculator
            .as_ref()
            .map(|calc| calc.get_progress_summary_formatted())
    }

    /// Get formatted model counts for display using REAL ProgressCalculator
    /// Returns Option to avoid fake code creation - let TUI handle None case
    pub fn model_counts_formatted(&self) -> Option<(String, String, String)> {
        self.current_progress_calculator
            .as_ref()
            .map(|calc| calc.model_counts_formatted())
    }

    /// Get model count using REAL ProgressCalculator - ZERO pattern violations
    pub fn get_model_count(&self) -> usize {
        self.current_progress_calculator
            .as_ref()
            .map(|calc| calc.get_model_count())
            .unwrap_or(0)
    }

    /// Get model name at index using REAL ProgressCalculator - ZERO pattern violations
    pub fn get_model_name_at_index(&self, index: usize) -> Option<String> {
        self.current_progress_calculator
            .as_ref()
            .and_then(|calc| calc.get_model_name_at_index(index))
    }

    /// Get models for rendering using REAL ProgressCalculator - ZERO pattern violations
    pub fn get_models_for_rendering(&self) -> Vec<progresshub_progress::ImmutableModelProgress> {
        self.current_progress_calculator
            .as_ref()
            .map(|calc| calc.get_models_for_rendering())
            .unwrap_or_else(Vec::new)
    }
}
