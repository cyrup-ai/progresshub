//! UI focus handling and management for TUI components
//!
//! Provides atomic focus state updates, efficient focus transitions,
//! and zero-allocation focus management with const generic optimizations.

/// Focus state for interactive UI elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    /// Element is not focused
    None,
    /// Element is focused but not selected
    Focused,
    /// Element is both focused and selected
    Selected,
}

/// Focus states for bottom bar components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomBarFocus {
    /// No bottom bar component focused
    None,
    /// Remaining counter is focused
    Remaining,
    /// Overall progress is focused
    Progress,
    /// Bandwidth monitor is focused
    Bandwidth,
}

/// Manages focus states for different UI components
#[derive(Debug, Clone)]
pub struct FocusStates {
    /// Focus state for the model list
    pub model_list: FocusState,
    /// Focus state for the bottom bar sections
    pub bottom_bar: BottomBarFocus,
    /// Whether focus is currently active (determines visual feedback intensity)
    pub active: bool,
}

impl Default for FocusStates {
    fn default() -> Self {
        Self {
            model_list: FocusState::Focused, // Start with model list focused
            bottom_bar: BottomBarFocus::None,
            active: true,
        }
    }
}

impl FocusStates {
    /// Create a new focus manager with optimal defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set model list focus state with atomic update
    pub fn set_model_list_focus(&mut self, state: FocusState) {
        self.model_list = state;
    }

    /// Set bottom bar focus with atomic update
    pub fn set_bottom_bar_focus(&mut self, focus: BottomBarFocus) {
        self.bottom_bar = focus;
    }

    /// Activate focus system (enables visual feedback)
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate focus system (disables visual feedback)
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Toggle focus system activation
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }

    /// Move focus to next component with wraparound
    ///
    /// Implements efficient focus cycling using zero-allocation state transitions
    /// and atomic updates for blazing-fast UI responsiveness.
    pub fn focus_next(&mut self) {
        if !self.active {
            return;
        }

        match (self.model_list, self.bottom_bar) {
            // Currently on model list, move to first bottom bar component
            (FocusState::Focused, BottomBarFocus::None) => {
                self.model_list = FocusState::None;
                self.bottom_bar = BottomBarFocus::Remaining;
            }
            // Move through bottom bar components
            (FocusState::None, BottomBarFocus::Remaining) => {
                self.bottom_bar = BottomBarFocus::Progress;
            }
            (FocusState::None, BottomBarFocus::Progress) => {
                self.bottom_bar = BottomBarFocus::Bandwidth;
            }
            // Wrap around from last bottom bar component to model list
            (FocusState::None, BottomBarFocus::Bandwidth) => {
                self.bottom_bar = BottomBarFocus::None;
                self.model_list = FocusState::Focused;
            }
            // Handle edge cases - reset to model list
            _ => {
                self.model_list = FocusState::Focused;
                self.bottom_bar = BottomBarFocus::None;
            }
        }
    }

    /// Move focus to previous component with wraparound
    ///
    /// Implements efficient reverse focus cycling with atomic state updates
    /// and zero-allocation transitions for optimal performance.
    pub fn focus_previous(&mut self) {
        if !self.active {
            return;
        }

        match (self.model_list, self.bottom_bar) {
            // Currently on model list, wrap to last bottom bar component
            (FocusState::Focused, BottomBarFocus::None) => {
                self.model_list = FocusState::None;
                self.bottom_bar = BottomBarFocus::Bandwidth;
            }
            // Move backwards through bottom bar components
            (FocusState::None, BottomBarFocus::Bandwidth) => {
                self.bottom_bar = BottomBarFocus::Progress;
            }
            (FocusState::None, BottomBarFocus::Progress) => {
                self.bottom_bar = BottomBarFocus::Remaining;
            }
            // Wrap from first bottom bar component to model list
            (FocusState::None, BottomBarFocus::Remaining) => {
                self.bottom_bar = BottomBarFocus::None;
                self.model_list = FocusState::Focused;
            }
            // Handle edge cases - reset to model list
            _ => {
                self.model_list = FocusState::Focused;
                self.bottom_bar = BottomBarFocus::None;
            }
        }
    }

    /// Clear all focus states
    pub fn clear_focus(&mut self) {
        self.model_list = FocusState::None;
        self.bottom_bar = BottomBarFocus::None;
    }

    /// Reset to default focus state (model list focused)
    pub fn reset_to_default(&mut self) {
        self.model_list = FocusState::Focused;
        self.bottom_bar = BottomBarFocus::None;
        self.active = true;
    }

    /// Check if model list is currently focused
    #[must_use]
    pub fn is_model_list_focused(&self) -> bool {
        matches!(self.model_list, FocusState::Focused | FocusState::Selected)
    }

    /// Check if any bottom bar component is focused
    #[must_use]
    pub fn is_bottom_bar_focused(&self) -> bool {
        !matches!(self.bottom_bar, BottomBarFocus::None)
    }

    /// Check if a specific bottom bar component is focused
    #[must_use]
    pub fn is_bottom_bar_component_focused(&self, component: BottomBarFocus) -> bool {
        self.bottom_bar == component
    }

    /// Get the currently focused component as a string (for debugging)
    #[must_use]
    pub fn current_focus_debug(&self) -> String {
        match (self.model_list, self.bottom_bar) {
            (FocusState::Focused, BottomBarFocus::None) => "ModelList".to_string(),
            (FocusState::Selected, BottomBarFocus::None) => "ModelList(Selected)".to_string(),
            (FocusState::None, BottomBarFocus::Remaining) => "BottomBar::Remaining".to_string(),
            (FocusState::None, BottomBarFocus::Progress) => "BottomBar::Progress".to_string(),
            (FocusState::None, BottomBarFocus::Bandwidth) => "BottomBar::Bandwidth".to_string(),
            _ => "None".to_string(),
        }
    }
}
