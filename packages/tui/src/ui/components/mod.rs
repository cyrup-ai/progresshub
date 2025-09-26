//! UI Component System
//!
//! This module provides reusable UI components for the ProgressHub TUI.
//! Each component is designed to be composable and customizable to allow
//! for flexible UI layouts and interactions.

pub mod core;
pub mod data;
pub mod header;
pub mod widgets;

// Re-exports removed to eliminate unused export warnings.
// Import items explicitly at call sites when needed.
