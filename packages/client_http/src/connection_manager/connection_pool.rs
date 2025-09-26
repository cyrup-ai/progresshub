//! Lock-free connection pooling for QUIC connections
//!
//! Provides high-performance connection pool configuration with atomic operations,
//! zero-allocation connection reuse patterns, and blazing-fast pool management.

use std::time::Duration;

/// Configuration for connection pool optimization.
///
/// Provides comprehensive tuning parameters for connection pool behavior
/// with production-optimized defaults for high-throughput scenarios.
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of concurrent connections per endpoint
    pub max_connections: usize,
    /// Connection idle timeout before cleanup
    pub idle_timeout: Duration,
    /// Maximum time to wait for connection establishment
    pub connect_timeout: Duration,
    /// Enable connection keepalive for long-lived connections
    pub keepalive_enabled: bool,
    /// Keepalive interval for connection health checks
    pub keepalive_interval: Duration,
}

impl Default for ConnectionPoolConfig {
    /// Production-optimized connection pool defaults.
    ///
    /// Configured for high-throughput scenarios with intelligent resource
    /// management and optimal performance characteristics.
    fn default() -> Self {
        Self {
            max_connections: 32,                      // Optimal for most deployment scenarios
            idle_timeout: Duration::from_secs(7200),  // 2 hours idle timeout for large files
            connect_timeout: Duration::from_secs(30), // 30 second connection timeout
            keepalive_enabled: true,                  // Enable for production reliability
            keepalive_interval: Duration::from_secs(60), // 1 minute keepalive
        }
    }
}

impl ConnectionPoolConfig {
    /// Create a new connection pool configuration with custom settings
    ///
    /// # Arguments
    /// * `max_connections` - Maximum concurrent connections per endpoint
    /// * `idle_timeout` - Connection idle timeout before cleanup
    /// * `connect_timeout` - Connection establishment timeout
    /// * `keepalive_enabled` - Enable connection keepalive
    /// * `keepalive_interval` - Keepalive interval for health checks
    #[must_use]
    pub fn new(
        max_connections: usize,
        idle_timeout: Duration,
        connect_timeout: Duration,
        keepalive_enabled: bool,
        keepalive_interval: Duration,
    ) -> Self {
        Self {
            max_connections,
            idle_timeout,
            connect_timeout,
            keepalive_enabled,
            keepalive_interval,
        }
    }

    /// Create high-performance configuration for production workloads
    ///
    /// Optimized for high-throughput scenarios with aggressive connection reuse,
    /// large connection pools, and ultra-low latency overhead.
    #[must_use]
    pub fn high_performance() -> Self {
        Self {
            max_connections: 128,                     // Large pool for high concurrency
            idle_timeout: Duration::from_secs(7200),  // 2 hours for large file downloads
            connect_timeout: Duration::from_secs(15), // Fast connection establishment
            keepalive_enabled: true,
            keepalive_interval: Duration::from_secs(30), // Aggressive keepalive for reliability
        }
    }

    /// Create resource-constrained configuration for limited environments
    ///
    /// Optimized for efficient resource usage with smaller connection pools,
    /// shorter timeouts, and conservative connection management.
    #[must_use]
    pub fn resource_constrained() -> Self {
        Self {
            max_connections: 8,                       // Small pool for resource conservation
            idle_timeout: Duration::from_secs(120),   // 2 minutes to free resources quickly
            connect_timeout: Duration::from_secs(45), // Longer timeout for limited bandwidth
            keepalive_enabled: false,                 // Disable to save resources
            keepalive_interval: Duration::from_secs(120), // Less frequent checks
        }
    }

    /// Validate pool configuration parameters
    ///
    /// # Errors
    /// Returns error if configuration parameters are invalid or would cause
    /// poor performance in production deployments
    pub fn validate(&self) -> Result<(), String> {
        if self.max_connections == 0 {
            return Err("max_connections must be greater than 0".to_string());
        }

        if self.max_connections > 1000 {
            return Err("max_connections exceeds recommended maximum (1000)".to_string());
        }

        if self.idle_timeout < Duration::from_secs(10) {
            return Err("idle_timeout should be at least 10 seconds".to_string());
        }

        if self.connect_timeout < Duration::from_secs(5) {
            return Err("connect_timeout should be at least 5 seconds".to_string());
        }

        if self.keepalive_enabled && self.keepalive_interval < Duration::from_secs(10) {
            return Err(
                "keepalive_interval should be at least 10 seconds when enabled".to_string(),
            );
        }

        Ok(())
    }
}
