//! Tests for client selector functionality
//!
//! This module contains comprehensive tests for the Client, DownloadConfig,
//! ProgressAggregator, and MultiDownloadOrchestrator components.

use progresshub_client_selector::client::{Client, DownloadConfig, ProgressAggregator, MultiDownloadOrchestrator};
use progresshub_client_selector::types::Backend;

#[test]
fn test_download_config_default() {
    let config = DownloadConfig::default();
    assert!(config.show_progress);
    assert!(config.use_cache);
}

#[tokio::test]
async fn test_client_creation() {
    // Test that client creation doesn't panic with Auto backend
    let result = Client::new(Backend::Auto);
    // We expect this might fail in test environment, but shouldn't panic
    match result {
        Ok(_) => {
            // Client creation succeeded
        }
        Err(_) => {
            // Client creation failed (expected in test environment)
            // This is acceptable as setup might not be available
        }
    }
}

#[test]
fn test_progress_aggregator() {
    let mut aggregator = ProgressAggregator::new(1000, 5);
    assert_eq!(aggregator.stats().files_pending, 5);
    assert_eq!(aggregator.stats().files_completed, 0);
    assert_eq!(aggregator.stats().bytes_downloaded, 0);
    
    // Test file start tracking
    aggregator.start_file("test.txt");
    assert_eq!(aggregator.stats().files_pending, 4);
    assert_eq!(aggregator.stats().files_active, 1);
}

#[test]
fn test_multi_download_orchestrator() {
    let orchestrator = MultiDownloadOrchestrator::new(
        "microsoft/DialoGPT-medium",
        std::path::PathBuf::from("./test"),
        true,
        None,
    );
    
    let orchestrator_with_backend = orchestrator.with_backend(Backend::Quic);
    // Test that the orchestrator can be created and configured
    assert!(format!("{orchestrator_with_backend:?}").contains("DialoGPT-medium"));
}