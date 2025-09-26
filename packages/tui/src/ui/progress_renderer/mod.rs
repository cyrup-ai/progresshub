//! Magical Progress Renderer with TachyonFX Effects
//!
//! Zero-allocation modular progress rendering system with blazing-fast
//! tachyonfx integration for production-grade magical visual effects.

pub mod config;
pub mod effects;
pub mod renderer;

// Re-export public API with clean ergonomic surface
pub use config::{ProgressColorScheme, ProgressRendererConfig};
pub use renderer::ProgressRenderer;
