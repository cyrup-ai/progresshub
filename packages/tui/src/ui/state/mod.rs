//! TUI state management modules
//!
//! Decomposed TUI state management with focused, zero-allocation modules:
//! - `app_state`: Core application state with completion flow management
//! - `focus_manager`: UI focus handling with atomic state updates
//! - `ui_coordinator`: State coordination and progress types

pub mod app_state;
pub mod focus_manager;
pub mod ui_coordinator;

// Re-export core types for backward compatibility (minimized to used items)
pub use app_state::{AppState, CompletionState};
pub use focus_manager::FocusState;
pub use ui_coordinator::{DownloadStatus, ModelDownload};
