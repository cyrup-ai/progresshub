// Re-export OneOrMany from the one_or_many module
pub use crate::OneOrMany;

/// Configuration for the downloader
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Whether to show progress in the terminal
    pub show_progress: bool,
    /// Whether to use the cache if files already exist
    pub use_cache: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            show_progress: true,
            use_cache: true,
        }
    }
}

/// Builder for DownloadConfig
#[derive(Debug, Default)]
pub struct DownloadConfigBuilder {
    config: DownloadConfig,
}

impl DownloadConfigBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to show progress
    pub fn show_progress(mut self, show: bool) -> Self {
        self.config.show_progress = show;
        self
    }

    /// Set whether to use the cache
    pub fn use_cache(mut self, use_cache: bool) -> Self {
        self.config.use_cache = use_cache;
        self
    }

    /// Build the config
    pub fn build(self) -> DownloadConfig {
        self.config
    }
}
