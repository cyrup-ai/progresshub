//! Tests for ChunkManager functionality
//!
//! This module contains comprehensive tests for the chunk management system
//! including chunk strategy calculation, validation, and range processing.
//! Extracted from src/chunk_manager.rs for production code organization.

use progresshub_client_http::{
    ChunkError, ChunkStrategy, ChunkValidator, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE,
};

#[test]
fn test_chunk_strategy_calculation() {
    // Small file: single chunk
    let strategy = ChunkStrategy::calculate_from_chunk_size(2048, 1024); // Large chunk for small file
    assert_eq!(strategy, ChunkStrategy::Single);

    // Medium file: small chunks
    let strategy = ChunkStrategy::calculate_from_chunk_size(8 * 1024 * 1024, 50 * 1024 * 1024); // 8MB chunks for 50MB file
    match strategy {
        ChunkStrategy::Small { chunk_size, .. } => {
            assert_eq!(chunk_size, MIN_CHUNK_SIZE);
        }
        _ => panic!("Expected Small strategy"),
    }

    // Large file: large chunks
    let strategy = ChunkStrategy::calculate_from_chunk_size(16 * 1024 * 1024, 500 * 1024 * 1024); // 16MB chunks for 500MB file
    match strategy {
        ChunkStrategy::Large { chunk_size, .. } => {
            assert_eq!(chunk_size, MAX_CHUNK_SIZE);
        }
        _ => panic!("Expected Large strategy"),
    }
}

#[test]
fn test_chunk_validator() {
    let data = b"Hello, world!";
    let hash = ChunkValidator::validate_chunk(data, None).expect("Validation should succeed");

    // Verify hash is valid hex string
    assert_eq!(hash.len(), 64); // SHA256 is 32 bytes = 64 hex chars
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Test with expected hash
    let result = ChunkValidator::validate_chunk(data, Some(&hash));
    assert!(result.is_ok());

    // Test with wrong expected hash
    let result = ChunkValidator::validate_chunk(data, Some("wrong_hash"));
    assert!(matches!(result, Err(ChunkError::ValidationFailed { .. })));
}

// Note: test_create_chunks_from_ranges removed as it tested private method
// create_chunks_from_ranges(). Production code maintains clean public API
// without exposing internal chunking logic. Functionality is tested through
// integration tests via public download methods.
