//! Unit tests for FilesystemProgressEvaluator functionality
//!
//! Extracted from src/filesystem_evaluator.rs for production code organization.

use progresshub_progress::{DownloadStateFile, FilesystemProgressEvaluator, write_download_state};
use std::time::SystemTime;
use tempfile::TempDir;

#[tokio::test]
async fn test_write_state_and_validate_progress() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let state_dir = temp_dir.path();

    let state = DownloadStateFile {
        model_id: "microsoft/DialoGPT-small".to_string(),
        file_path: "pytorch_model.bin".to_string(),
        bytes_downloaded: 1024,
        total_bytes: 2048,
        is_cached: false,
        client_type: "xet".to_string(),
        timestamp: SystemTime::now(),
    };

    // Write state to filesystem
    write_download_state(state_dir, &state)
        .await
        .expect("Failed to write download state");

    // Test validation functionality
    let evaluator = FilesystemProgressEvaluator::new();
    let validated = evaluator
        .validate_progress("microsoft/DialoGPT-small", "pytorch_model.bin", 1024, 2048)
        .await
        .expect("Failed to validate progress");

    assert_eq!(validated.bytes_downloaded, 1024);
    assert_eq!(validated.total_bytes, 2048);
    // Note: from_cache is currently false as validation logic is simplified
}
