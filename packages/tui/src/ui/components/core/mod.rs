//! Core UI components for ProgressHub
//!
//! This module contains foundational UI components that provide
//! basic functionality used throughout the application.

pub mod collapsible;

// Re-export core components for easier access
pub use crate::ui::components::core::collapsible::{Collapsible, CollapsibleState};
