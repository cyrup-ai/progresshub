use progresshub_progress::central_dispatcher::{CentralProgressDispatcher, DispatcherConfig};
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_central_dispatcher_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
    let config = DispatcherConfig {
        debounce_interval: Duration::from_millis(50),
        filesystem_timeout: Duration::from_secs(1),
        state_dir: temp_dir.path().to_path_buf(),
    };

    let (progress_sender, _) = flume::unbounded();
    let requested_models = vec!["microsoft/DialoGPT-small".to_string()];
    let _dispatcher = CentralProgressDispatcher::new(config.clone(), progress_sender, requested_models);
    
    // Test passes if dispatcher is created without panic
    // Note: Fields are private, so we can't directly test them, but creation success indicates proper initialization
}
