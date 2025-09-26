//! Tests for QUIC connection manager functionality
//!
//! Comprehensive test suite covering endpoint validation, server name extraction,
//! URL construction, and connection pool configuration.

use progresshub_client_http::connection_manager::{
    ConnectionManager, ConnectionPoolConfig, ServerResolver,
};
use std::time::Duration;

#[test]
fn test_endpoint_validation() {
    // Valid endpoints
    assert!(ServerResolver::validate_endpoint("example.com:443").is_ok());
    assert!(ServerResolver::validate_endpoint("api.example.com:8443").is_ok());

    // Invalid endpoints
    assert!(ServerResolver::validate_endpoint("").is_err());
    assert!(ServerResolver::validate_endpoint("example.com").is_err()); // Missing port
    assert!(ServerResolver::validate_endpoint("example.com:0").is_err()); // Port 0
    assert!(ServerResolver::validate_endpoint("example.com:notaport").is_err()); // Invalid port
}

#[test]
fn test_server_name_extraction() {
    assert_eq!(
        ServerResolver::extract_server_name("github.com:443")
            .expect("Failed to extract server name"),
        "github.com"
    );
    assert_eq!(
        ServerResolver::extract_server_name("https://api.github.com:8443")
            .expect("Failed to extract server name"),
        "api.github.com"
    );
    assert_eq!(
        ServerResolver::extract_server_name("quic://github.com:4433")
            .expect("Failed to extract server name"),
        "github.com"
    );
}

#[test]
fn test_url_construction() {
    // Test various path formats
    let url = ServerResolver::build_download_url("microsoft/DialoGPT-medium/pytorch_model.bin")
        .expect("Failed to build download URL");
    assert!(url.contains(
        "https://huggingface.co/microsoft/DialoGPT-medium/resolve/main/pytorch_model.bin"
    ));

    // Test already complete URL
    let complete_url = "https://github.com/file.bin";
    assert_eq!(
        ServerResolver::build_download_url(complete_url).expect("Failed to build download URL"),
        complete_url
    );

    // Test invalid paths
    assert!(ServerResolver::build_download_url("").is_err());
    assert!(ServerResolver::build_download_url("single_segment").is_err());
}

#[test]
fn test_connection_pool_config() {
    let config = ConnectionPoolConfig::default();
    assert!(config.max_connections > 0);
    assert!(config.idle_timeout > Duration::ZERO);
    assert!(config.connect_timeout > Duration::ZERO);
    assert!(config.keepalive_enabled);
}

#[test]
fn test_connection_pool_config_validation() {
    // Valid configuration
    let valid_config = ConnectionPoolConfig::default();
    assert!(valid_config.validate().is_ok());

    // Invalid max connections
    let invalid_config = ConnectionPoolConfig::new(
        0, // Invalid: zero connections
        Duration::from_secs(300),
        Duration::from_secs(30),
        true,
        Duration::from_secs(60),
    );
    assert!(invalid_config.validate().is_err());

    // Invalid idle timeout
    let invalid_config = ConnectionPoolConfig::new(
        32,
        Duration::from_secs(5), // Invalid: too short
        Duration::from_secs(30),
        true,
        Duration::from_secs(60),
    );
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_connection_pool_config_presets() {
    // High performance configuration
    let high_perf = ConnectionPoolConfig::high_performance();
    assert!(high_perf.max_connections >= 64);
    assert!(high_perf.keepalive_enabled);
    assert!(high_perf.validate().is_ok());

    // Resource constrained configuration
    let constrained = ConnectionPoolConfig::resource_constrained();
    assert!(constrained.max_connections <= 16);
    assert!(constrained.validate().is_ok());
}

#[test]
fn test_connection_manager_creation() {
    // Valid endpoint
    let manager =
        ConnectionManager::new("github.com:443").expect("Failed to create connection manager");
    assert_eq!(manager.endpoint(), "github.com:443");
    assert_eq!(manager.server_name(), "github.com");

    // Invalid endpoint should fail
    assert!(ConnectionManager::new("invalid_endpoint").is_err());
}

#[test]
fn test_connection_manager_builder_pattern() {
    let manager = ConnectionManager::new("github.com:443")
        .expect("Failed to create connection manager")
        .with_max_connections(64)
        .with_idle_timeout(Duration::from_secs(600))
        .with_keepalive(false);

    assert_eq!(manager.pool_config().max_connections, 64);
    assert_eq!(manager.pool_config().idle_timeout, Duration::from_secs(600));
    assert!(!manager.pool_config().keepalive_enabled);
}

#[test]
fn test_connection_manager_default() {
    let manager = ConnectionManager::default();
    assert_eq!(manager.endpoint(), "huggingface.co:443");
    assert_eq!(manager.server_name(), "huggingface.co");
    assert!(manager.pool_config().max_connections > 0);
}

#[test]
fn test_connection_manager_clone() {
    let original = ConnectionManager::new("github.com:443")
        .expect("Failed to create connection manager")
        .with_max_connections(32);

    let cloned = original.clone();
    assert_eq!(original.endpoint(), cloned.endpoint());
    assert_eq!(original.server_name(), cloned.server_name());
    assert_eq!(
        original.pool_config().max_connections,
        cloned.pool_config().max_connections
    );
}

#[test]
fn test_url_construction_edge_cases() {
    // Test path with multiple segments
    let url =
        ServerResolver::build_download_url("microsoft/DialoGPT-medium/models/pytorch_model.bin")
            .expect("Failed to build download URL");
    assert!(url.contains(
        "https://huggingface.co/microsoft/DialoGPT-medium/resolve/main/models/pytorch_model.bin"
    ));

    // Test path with dangerous characters (should fail)
    assert!(ServerResolver::build_download_url("../../../etc/passwd").is_err());
    assert!(ServerResolver::build_download_url("path\\with\\backslashes").is_err());

    // Test empty segments (should fail)
    assert!(ServerResolver::build_download_url("repo//filename").is_err());

    // Test very long path (should fail)
    let long_path = "a".repeat(2000);
    assert!(ServerResolver::build_download_url(&long_path).is_err());
}
