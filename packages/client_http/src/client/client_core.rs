//! Core HTTP client implementation
//!
//! Provides the fundamental `HttpClient` struct with configuration, error types,
//! authentication, and builder patterns optimized for zero-allocation performance.

use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::Arc,
    task::{Context as TaskContext, Poll},
    time::Duration,
};

use anyhow::Result;
use pin_project::pin_project;
use progresshub_common::ProgressHandler;
use thiserror::Error as ThisError;
use tokio::sync::oneshot;

use crate::types::ConnectionInfo;

/// Configuration for resumable file download operations
pub struct DownloadFileConfig<'a> {
    pub path: &'a str,
    pub destination: &'a Path,
    pub expected_file_size: u64,
    pub expected_hash: Option<String>,
    /// Remote URL from manifest (used exactly as provided)
    pub remote_url: String,
    pub manifest_version: String,
    pub expected_chunk_size: u64,
    pub progress_handler: Arc<dyn ProgressHandler + Send + Sync>,
    /// Model identifier for event dispatch (`repo_id`)
    pub model_id: Option<String>,
    /// Flume sender for raw download events (Pure Flume Channel Event-Driven Architecture)
    pub raw_event_sender: Option<flume::Sender<progresshub_common::RawDownloadEvent>>,
    /// Quantization specification for this download (e.g., "`Q4_K_M`", "`Q8_0`", "`F16`")
    pub quantization: String,
}

/// A future that can be cancelled by dropping the associated `CancellationToken`
#[pin_project]
pub struct CancelableFuture<F> {
    #[pin]
    future: F,
    #[pin]
    cancel_rx: oneshot::Receiver<()>,
}

impl<F, T, E> Future for CancelableFuture<F>
where
    F: Future<Output = Result<T, E>>,
    E: From<HttpClientError>,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // Check if cancellation was requested
        match this.cancel_rx.poll(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Err(HttpClientError::Canceled.into())),
            Poll::Ready(Err(_)) => Poll::Ready(Err(HttpClientError::ConnectionError(
                "CancelableFuture: sender was dropped without sending".to_string(),
            )
            .into())),
            Poll::Pending => {
                // No cancellation, poll the inner future
                match this.future.poll(cx) {
                    Poll::Ready(result) => Poll::Ready(result),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

/// Token that can be used to cancel a request
#[derive(Debug)]
pub struct CancellationToken(oneshot::Sender<()>);

impl CancellationToken {
    /// Create a new cancellation token and its associated future
    pub fn new<F, T, E>(future: F) -> (Self, impl Future<Output = Result<T, E>>)
    where
        F: Future<Output = Result<T, E>>,
        E: From<HttpClientError>,
    {
        let (tx, rx) = oneshot::channel();
        let cancelable = CancelableFuture {
            future,
            cancel_rx: rx,
        };
        (Self(tx), cancelable)
    }

    /// Cancel the associated operation
    ///
    /// # Errors
    ///
    /// Returns `HttpClientError::Canceled` if the cancellation signal cannot be sent
    /// to the associated operation receiver.
    pub fn cancel(self) -> Result<(), HttpClientError> {
        self.0.send(()).map_err(|()| HttpClientError::Canceled)
    }
}

/// Custom error type for HTTP client operations
#[derive(Debug, ThisError)]
pub enum HttpClientError {
    #[error("HTTP connection error: {0}")]
    ConnectionError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Operation timed out")]
    Timeout,

    #[error("Maximum retry attempts ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("Operation canceled")]
    Canceled,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// HTTP authentication methods
#[derive(Debug, Clone)]
pub enum HttpAuth {
    /// Anonymous connection (testing only)
    Anonymous,
}

impl Default for HttpAuth {
    fn default() -> Self {
        Self::Anonymous
    }
}

/// Production-ready HTTP client with connection pooling and retry logic
#[derive(Debug)]
pub struct HttpClient {
    /// Base endpoint for connections (parsed format: host:port)
    pub(super) endpoint: String,
    /// Original endpoint/URL format as provided by user
    pub(super) original_endpoint: String,
    /// Authentication method
    pub(super) auth: HttpAuth,
    /// Default request timeout
    pub(super) request_timeout: Duration,
    /// Connection establishment timeout
    pub(super) connect_timeout: Duration,
    /// Server name for TLS verification
    pub(super) server_name: String,
}

impl HttpClient {
    /// Create a new QUIC client with custom endpoint
    ///
    /// Accepts both URL formats (<https://example.com>) and endpoint formats (example.com:443).
    /// URLs are automatically parsed to extract the appropriate endpoint and server name.
    ///
    /// # Errors
    ///
    /// Returns an error if the QUIC client cannot be built due to invalid configuration
    /// or TLS setup issues.
    pub fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();

        // Parse URL or endpoint format
        let (parsed_endpoint, server_name) =
            if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
                // Handle URL format
                let url = url::Url::parse(&endpoint)
                    .map_err(|e| anyhow::anyhow!("Invalid URL format: {}", e))?;
                let host = url
                    .host_str()
                    .ok_or_else(|| anyhow::anyhow!("URL must contain a host"))?;
                let port = url.port().unwrap_or(if endpoint.starts_with("https://") {
                    443
                } else {
                    80
                });
                let parsed_endpoint = format!("{host}:{port}");
                (parsed_endpoint, host.to_string())
            } else {
                // Handle endpoint format (server.com:port)
                if !endpoint.contains(':') {
                    return Err(anyhow::anyhow!(
                        "Endpoint must include port (e.g., 'server.com:4433') or be a valid URL"
                    ));
                }
                let server_name = if let Some(host) = endpoint.split(':').next() {
                    host.to_string()
                } else {
                    endpoint.clone()
                };
                (endpoint.clone(), server_name)
            };

        Ok(Self {
            endpoint: parsed_endpoint,
            original_endpoint: endpoint.clone(),
            auth: HttpAuth::default(),
            request_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(30),
            server_name,
        })
    }

    /// Create a new HTTP client for `HuggingFace` with defaults
    ///
    /// # Errors
    ///
    /// Returns an error if the QUIC client cannot be created for the `HuggingFace` endpoint.
    pub fn for_huggingface() -> Result<Self> {
        Self::new("huggingface.co:4433")
    }

    /// Set the authentication method
    ///
    /// # Arguments
    /// * `auth` - The authentication method to use
    #[must_use]
    pub fn with_auth(mut self, auth: HttpAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Set the default request timeout
    ///
    /// # Arguments
    /// * `timeout` - The timeout duration
    #[must_use]
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set the connection establishment timeout
    ///
    /// # Arguments
    /// * `timeout` - The connection timeout duration
    #[must_use]
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the request timeout in seconds
    ///
    /// # Arguments
    /// * `timeout_secs` - The timeout duration in seconds
    #[must_use]
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.request_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set the server name for TLS verification
    ///
    /// # Arguments
    /// * `server_name` - The server name to use for TLS verification
    #[must_use]
    pub fn with_server_name(mut self, server_name: impl Into<String>) -> Self {
        self.server_name = server_name.into();
        self
    }

    /// Get connection info for debugging
    #[must_use]
    pub fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            base_url: self.original_endpoint.clone(),
            timeout: self.request_timeout.as_secs(),
        }
    }

    /// Try to create a default QUIC client for `HuggingFace` with default settings
    ///
    /// # Errors
    /// Returns an error if the QUIC client cannot be created with the default configuration
    pub fn try_default() -> Result<Self> {
        Self::for_huggingface()
    }
}

// Note: Clone is needed for concurrent downloads
impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            original_endpoint: self.original_endpoint.clone(),
            auth: self.auth.clone(),
            request_timeout: self.request_timeout,
            connect_timeout: self.connect_timeout,
            server_name: self.server_name.clone(),
        }
    }
}

impl Default for HttpClient {
    /// Create a default HTTP client with safe, non-failing configuration
    ///
    /// Uses compile-time constants that cannot fail to ensure production safety.
    /// For HuggingFace-specific configuration, use `QuicClient::for_huggingface()` instead.
    fn default() -> Self {
        Self {
            endpoint: "huggingface.co:443".to_string(),
            original_endpoint: "https://huggingface.co".to_string(),
            auth: HttpAuth::default(),
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            server_name: "huggingface.co".to_string(),
        }
    }
}
