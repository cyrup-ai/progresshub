//! Download effects module for TachyonFX visual effects.
//!
//! This module provides sophisticated visual effects for download progress tracking,
//! organized into focused sub-modules for maintainability and performance.

pub mod advanced_effects;
pub mod core_effects;
pub mod effect_stack;
pub mod state_effects;
pub mod theme_utils;

// Re-export main types for backward compatibility
pub use advanced_effects::AdvancedEffects;
pub use core_effects::CoreEffects;
pub use effect_stack::DownloadEffectStack;
pub use state_effects::StateEffects;
pub use theme_utils::ThemeUtils;
