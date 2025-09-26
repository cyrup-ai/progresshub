//! Tests for disk progress tracking functionality
//!
//! This module contains comprehensive tests for the DiskProgressTracker
//! including basic progress tracking and flapping prevention.

use progresshub_progress::disk_progress::DiskProgressTracker;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_disk_progress_tracker_basic() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    
    let mut tracker = DiskProgressTracker::new(
        file_path.clone(),
        "test-model".to_string(),
        1000,
    );

    // Initially no progress
    assert!(!tracker.is_complete().await);
    
    // Write some data
    temp_file.write_all(b"hello").expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");
    
    // Check progress
    let progress = tracker.check_progress_if_unlocked().await;
    assert!(progress.is_some());
    
    let data = progress.expect("Progress data should be available");
    assert_eq!(data.bytes_downloaded, 5);
    assert_eq!(data.total_bytes, 1000);
    assert_eq!(data.model_id, "test-model");
}

#[tokio::test]
async fn test_progress_flapping_prevention() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    
    let mut tracker = DiskProgressTracker::new(
        file_path.clone(),
        "test-model".to_string(),
        1000,
    );

    // Write initial data (0.5% of total)
    temp_file.write_all(&[0u8; 5]).expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");
    
    // First check should report progress
    let progress1 = tracker.check_progress_if_unlocked().await;
    assert!(progress1.is_some());
    
    // Add tiny amount more data (still < 0.1% increase)
    temp_file.write_all(b"x").expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");
    
    // Second check should NOT report progress (< 0.1% increase)
    let progress2 = tracker.check_progress_if_unlocked().await;
    assert!(progress2.is_none());
    
    // Add significant data (> 0.1% increase)
    temp_file.write_all(&[0u8; 5]).expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");
    
    // Third check should report progress
    let progress3 = tracker.check_progress_if_unlocked().await;
    assert!(progress3.is_some());
}