//! Connection management modules for QUIC
//!
//! Decomposed connection management with focused, zero-allocation modules:
//! - `connection_pool`: Lock-free connection pooling with atomic operations
//! - `connection_lifecycle`: Connection management and lifecycle handling  
//! - `server_resolver`: Server name resolution and URL construction

pub mod connection_lifecycle;
pub mod connection_pool;
pub mod server_resolver;

// Re-export core types for backward compatibility
pub use connection_lifecycle::ConnectionManager;
pub use connection_pool::ConnectionPoolConfig;
pub use server_resolver::ServerResolver;
