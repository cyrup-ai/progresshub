//! Hash utilities for model collision prevention
//!
//! This module provides shared hash generation functions used by both
//! the HTTP client (for creating .part files) and the progress tracker
//! (for recognizing and aggregating .part files).

use sha1::{Digest, Sha1};

/// Generate a model-specific hash for file collision prevention
///
/// Creates an 8-character hex hash from a model ID that is used to prevent
/// filename collisions when downloading multiple models simultaneously.
/// The same model ID will always generate the same hash.
///
/// # Arguments
/// * `model_id` - Model identifier (e.g., "microsoft/DialoGPT-small")
///
/// # Returns
/// 8-character hex hash (e.g., "a1b2c3d4")
///
/// # Examples
/// ```
/// use progresshub_common::generate_model_hash;
///
/// let hash = generate_model_hash("microsoft/DialoGPT-small");
/// assert_eq!(hash.len(), 8);
/// ```
#[must_use]
pub fn generate_model_hash(model_id: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(model_id.as_bytes());
    let hash_bytes = hasher.finalize();
    
    // Take first 4 bytes (32 bits) and encode as hex (8 characters)
    hex::encode(&hash_bytes[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_model_hash_consistent() {
        let model_id = "microsoft/DialoGPT-small";
        let hash1 = generate_model_hash(model_id);
        let hash2 = generate_model_hash(model_id);
        assert_eq!(hash1, hash2, "Same model should generate same hash");
    }

    #[test]
    fn test_generate_model_hash_length() {
        let hash = generate_model_hash("test/model");
        assert_eq!(hash.len(), 8, "Hash should be 8 characters");
    }

    #[test]
    fn test_generate_model_hash_different_models() {
        let hash1 = generate_model_hash("microsoft/DialoGPT-small");
        let hash2 = generate_model_hash("microsoft/DialoGPT-medium");
        assert_ne!(hash1, hash2, "Different models should generate different hashes");
    }
}