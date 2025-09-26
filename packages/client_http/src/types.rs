//! Data types for the QUIC client.

/// Connection information for debugging
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub base_url: String,
    pub timeout: u64,
}
