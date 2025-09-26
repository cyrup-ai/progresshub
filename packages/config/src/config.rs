use std::path::PathBuf;
use std::sync::OnceLock;

use super::environment;

/// Configuration trait for managing HuggingFace cache settings
pub trait ConfigTrait {
    /// Get the HuggingFace home directory
    fn hf_home(&self) -> &PathBuf;

    /// Get the HuggingFace hub cache directory
    fn hub_cache(&self) -> &PathBuf;

    /// Get the HuggingFace authentication token
    fn token(&self) -> Option<&str>;
}

/// Singleton access to global configuration
pub fn get() -> &'static dyn ConfigTrait {
    static INSTANCE: OnceLock<ConfigImpl> = OnceLock::new();
    INSTANCE.get_or_init(ConfigImpl::new)
}

/// Private configuration implementation
#[derive(Debug, Clone)]
struct ConfigImpl {
    hf_home: PathBuf,
    hub_cache: PathBuf,
    token: Option<String>,
}

impl ConfigTrait for ConfigImpl {
    fn hf_home(&self) -> &PathBuf {
        &self.hf_home
    }

    fn hub_cache(&self) -> &PathBuf {
        &self.hub_cache
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

impl ConfigImpl {
    /// Create a new configuration instance with intelligent defaults
    fn new() -> Self {
        // Resolve directories using environment functions
        let hf_home = environment::get_hf_home();
        let hub_cache = environment::get_hf_hub_cache();

        // Resolve token with fallback mechanisms
        let token = environment::get_hf_token();

        Self {
            hf_home,
            hub_cache,
            token,
        }
    }
}
