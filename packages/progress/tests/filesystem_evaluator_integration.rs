//! Integration tests for filesystem evaluator validation services

use progresshub_progress::{
    DownloadStateFile, FilesystemProgressEvaluator, get_default_state_dir, write_download_state,
};
use std::time::SystemTime;

#[tokio::test]
async fn test_filesystem_evaluator_validates_progress() {
    let evaluator = FilesystemProgressEvaluator::new();

    // Test basic progress validation
    let validated = evaluator
        .validate_progress("microsoft/DialoGPT-small", "pytorch_model.bin", 1024, 2048)
        .await
        .expect("Failed to validate progress");

    assert_eq!(validated.bytes_downloaded, 1024);
    assert_eq!(validated.total_bytes, 2048);
    // Note: from_cache is currently false as validation logic is simplified
}

#[tokio::test]
async fn test_get_default_state_dir() {
    let state_dir = get_default_state_dir();

    // Should be under HF cache directory
    assert!(state_dir.to_string_lossy().contains("progresshub_state"));

    // Directory should be creatable
    tokio::fs::create_dir_all(&state_dir)
        .await
        .expect("Failed to create state directory");

    // Should be able to write state files
    let test_state = DownloadStateFile {
        model_id: "test/model".to_string(),
        file_path: "test.bin".to_string(),
        bytes_downloaded: 100,
        total_bytes: 200,
        is_cached: false,
        client_type: "test".to_string(),
        timestamp: SystemTime::now(),
    };

    write_download_state(&state_dir, &test_state)
        .await
        .expect("Failed to write download state");

    // Cleanup
    if state_dir.exists() {
        tokio::fs::remove_dir_all(&state_dir).await.ok();
    }
}
