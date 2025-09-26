use std::collections::HashMap;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::ui::state::ModelDownload;

/// State for the model list widget
#[derive(Debug, Clone)]
pub struct ModelListState {
    /// The underlying list state for selection and scrolling
    pub(crate) list_state: ListState,
    /// Whether each model is expanded or collapsed
    expanded: HashMap<String, bool>,
    /// Last interaction time for auto-collapse prevention
    last_interaction: Instant,
}

impl Default for ModelListState {
    fn default() -> Self {
        Self {
            list_state: ListState::default(),
            expanded: HashMap::new(),
            last_interaction: Instant::now(),
        }
    }
}

impl ModelListState {
    /// Create a new model list state
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the next item in the list
    pub fn next(&mut self, length: usize) {
        self.last_interaction = Instant::now();

        if length == 0 {
            self.list_state.select(None);
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= length - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select the previous item in the list
    pub fn previous(&mut self, length: usize) {
        self.last_interaction = Instant::now();

        if length == 0 {
            self.list_state.select(None);
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    length - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Toggle the expanded state of the specified model
    pub fn toggle_expanded(&mut self, model_id: &str) {
        self.last_interaction = Instant::now();
        let expanded = self.expanded.entry(model_id.to_string()).or_insert(true);
        *expanded = !*expanded;
    }

    /// Get the currently selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Set the selected index
    pub fn select(&mut self, index: Option<usize>) {
        self.last_interaction = Instant::now();
        self.list_state.select(index);
    }

    /// Get whether a model is expanded
    pub fn is_expanded(&self, model_id: &str) -> bool {
        self.expanded.get(model_id).copied().unwrap_or(true)
    }

    /// Auto-collapse completed models if no recent interaction
    pub fn auto_collapse_completed(&mut self, models: &[&ModelDownload], threshold_secs: u64) {
        let now = Instant::now();
        let threshold = Duration::from_secs(threshold_secs);

        // Only auto-collapse if there's been no interaction for the threshold period
        if now.duration_since(self.last_interaction) < threshold {
            return;
        }

        for model in models {
            if model.progress.is_complete() {
                self.expanded.insert(model.model_name.clone(), false);
            }
        }
    }

    /// Reset the interaction timer
    pub fn reset_interaction_timer(&mut self) {
        self.last_interaction = Instant::now();
    }

    /// Expand all models
    pub fn expand_all(&mut self, models: &[&ModelDownload]) {
        self.last_interaction = Instant::now();
        for model in models {
            self.expanded.insert(model.model_name.clone(), true);
        }
    }

    /// Collapse all models
    pub fn collapse_all(&mut self, models: &[&ModelDownload]) {
        self.last_interaction = Instant::now();
        for model in models {
            self.expanded.insert(model.model_name.clone(), false);
        }
    }
}

use super::model_list::ModelListStateAccess;

impl ModelListStateAccess for ModelListState {
    /// Check if a model is currently expanded
    fn is_expanded(&self, model_id: &str) -> bool {
        self.expanded.get(model_id).copied().unwrap_or(true)
    }
}
