//! Advanced environment variable handling for HuggingFace cache locations
//! Implements a comprehensive, cross-platform strategy for resolving cache directories

use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

// Default path constants to eliminate repeated allocations
const DEFAULT_CACHE_DIR: &str = ".cache";
const DEFAULT_HF_DIR: &str = "huggingface";
const DEFAULT_HUB_DIR: &str = "hub";
const DEFAULT_ASSETS_DIR: &str = "assets";
const DEFAULT_DATASETS_DIR: &str = "datasets";
const DEFAULT_DOWNLOADS_DIR: &str = "downloads";
const DEFAULT_EXTRACTED_DIR: &str = "extracted";
const DEFAULT_TRANSFORMERS_DIR: &str = "transformers";
const DEFAULT_TOKEN_FILE: &str = "token";
const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";
const DEFAULT_HF_VERBOSITY: &str = "info";

/// Determine the HuggingFace home directory with intelligent fallback mechanisms
pub fn get_hf_home() -> PathBuf {
    // Priority 1: Explicit HF_HOME environment variable
    if let Ok(hf_home) = env::var("HF_HOME") {
        return PathBuf::from(hf_home);
    }

    // Priority 2: XDG_CACHE_HOME for Linux/Unix-like systems
    if let Ok(xdg_cache) = env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg_cache).join(DEFAULT_HF_DIR);
    }

    // Priority 3: Platform-specific home directory fallback
    get_home_dir()
        .map(|home| home.join(DEFAULT_CACHE_DIR).join(DEFAULT_HF_DIR))
        .unwrap_or_else(|| {
            // Fallback to current directory if all else fails
            PathBuf::from(DEFAULT_CACHE_DIR).join(DEFAULT_HF_DIR)
        })
}

/// Cached HF hub cache directory - evaluated only once
static HF_HUB_CACHE: OnceLock<PathBuf> = OnceLock::new();

/// Resolve the HuggingFace hub cache directory
pub fn get_hf_hub_cache() -> PathBuf {
    HF_HUB_CACHE
        .get_or_init(|| {
            // Priority 1: Explicit HF_HUB_CACHE environment variable
            if let Ok(hub_cache) = env::var("HF_HUB_CACHE") {
                return PathBuf::from(hub_cache);
            }

            // Check for legacy variable (no warning - both are fine)
            if let Ok(hub_cache) = env::var("HUGGINGFACE_HUB_CACHE") {
                return PathBuf::from(hub_cache);
            }

            // Fallback: Hub cache in HF home directory
            // Follow standard HuggingFace behavior: when HF_HOME is set, use $HF_HOME/huggingface/hub/
            if env::var("HF_HOME").is_ok() {
                get_hf_home().join(DEFAULT_HF_DIR).join(DEFAULT_HUB_DIR)
            } else {
                get_hf_home().join(DEFAULT_HUB_DIR)
            }
        })
        .clone()
}

/// Get the HuggingFace token with legacy support and migration warning
pub fn get_hf_token() -> Option<String> {
    // Priority 1: Modern HF_TOKEN
    if let Ok(token) = env::var("HF_TOKEN") {
        return Some(token);
    }

    // Priority 2: Legacy token (no warning - both are fine)
    if let Ok(token) = env::var("HUGGING_FACE_HUB_TOKEN") {
        return Some(token);
    }

    None
}

/// Get the token path with intelligent fallback
pub fn get_hf_token_path() -> PathBuf {
    env::var("HF_TOKEN_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_home().join(DEFAULT_TOKEN_FILE))
}

/// Platform-independent home directory retrieval
pub fn get_home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var("USERPROFILE").ok().map(PathBuf::from)
    } else {
        env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Check if Hub should operate in offline mode (only use cached files)
pub fn is_hf_hub_offline() -> bool {
    env::var("HF_HUB_OFFLINE")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Get download timeout in seconds
pub fn get_hf_hub_download_timeout() -> u64 {
    env::var("HF_HUB_DOWNLOAD_TIMEOUT")
        .map(|timeout| timeout.parse().unwrap_or(86400)) // Default 24 hours (86400 seconds) for production
        .unwrap_or(86400)
}

/// Check if Rust-based hf_transfer should be enabled for faster downloads
pub fn is_hf_transfer_enabled() -> bool {
    env::var("HF_HUB_ENABLE_HF_TRANSFER")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if progress bars should be disabled
pub fn is_progress_bars_disabled() -> bool {
    env::var("HF_HUB_DISABLE_PROGRESS_BARS")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if telemetry should be disabled
pub fn is_telemetry_disabled() -> bool {
    env::var("HF_HUB_DISABLE_TELEMETRY")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if debug logging should be enabled
pub fn is_hf_debug_enabled() -> bool {
    env::var("HF_DEBUG")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Get HF Hub verbosity level
pub fn get_hf_hub_verbosity() -> String {
    env::var("HF_HUB_VERBOSITY").unwrap_or_else(|_| DEFAULT_HF_VERBOSITY.to_string())
}

/// Get HF assets cache directory (for preprocessed assets)
pub fn get_hf_assets_cache() -> PathBuf {
    env::var("HF_ASSETS_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_home().join(DEFAULT_ASSETS_DIR))
}

/// Get HuggingFace Hub endpoint URL
pub fn get_hf_endpoint() -> String {
    env::var("HF_ENDPOINT").unwrap_or_else(|_| DEFAULT_HF_ENDPOINT.to_string())
}

/// Get ETag timeout in seconds (for cache validation)
pub fn get_hf_hub_etag_timeout() -> u64 {
    env::var("HF_HUB_ETAG_TIMEOUT")
        .map(|timeout| timeout.parse().unwrap_or(10)) // Default 10 seconds
        .unwrap_or(10)
}

/// Check if implicit token usage should be disabled
pub fn is_hf_hub_implicit_token_disabled() -> bool {
    env::var("HF_HUB_DISABLE_IMPLICIT_TOKEN")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if symlinks warning should be disabled
pub fn is_hf_hub_symlinks_warning_disabled() -> bool {
    env::var("HF_HUB_DISABLE_SYMLINKS_WARNING")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if experimental feature warnings should be disabled
pub fn is_hf_hub_experimental_warning_disabled() -> bool {
    env::var("HF_HUB_DISABLE_EXPERIMENTAL_WARNING")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Check if local folder scanning should be disabled
pub fn is_hf_hub_local_folder_disabled() -> bool {
    env::var("HF_HUB_DISABLE_LOCAL_FOLDER")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Get datasets cache directory
pub fn get_hf_datasets_cache() -> PathBuf {
    env::var("HF_DATASETS_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_home().join(DEFAULT_DATASETS_DIR))
}

/// Get datasets downloaded datasets path
pub fn get_hf_datasets_downloaded_datasets_path() -> PathBuf {
    env::var("HF_DATASETS_DOWNLOADED_DATASETS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_datasets_cache().join(DEFAULT_DOWNLOADS_DIR))
}

/// Get datasets extracted path
pub fn get_hf_datasets_extracted_path() -> PathBuf {
    env::var("HF_DATASETS_EXTRACTED_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_datasets_cache().join(DEFAULT_EXTRACTED_DIR))
}

/// Check if datasets offline mode is enabled
pub fn is_hf_datasets_offline() -> bool {
    env::var("HF_DATASETS_OFFLINE")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Get datasets in-memory max size
pub fn get_hf_datasets_in_memory_max_size() -> Option<u64> {
    env::var("HF_DATASETS_IN_MEMORY_MAX_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
}

/// Check if datasets progress bars should be disabled
pub fn is_hf_datasets_disable_progress_bars() -> bool {
    env::var("HF_DATASETS_DISABLE_PROGRESS_BARS")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Get transformers cache directory
pub fn get_transformers_cache() -> PathBuf {
    env::var("TRANSFORMERS_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_hf_home().join(DEFAULT_TRANSFORMERS_DIR))
}

/// Check if transformers offline mode is enabled
pub fn is_transformers_offline() -> bool {
    env::var("TRANSFORMERS_OFFLINE")
        .map(|val| matches!(val.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
