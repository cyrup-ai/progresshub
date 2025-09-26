use std::fmt;

/// Stack-allocated error types for hang detection and timeout management
///
/// Uses Copy + Clone semantics with &'static str messages to avoid heap allocation.
/// All error context is stored in const arrays for blazing-fast stack allocation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HangError {
    /// Orchestrator creation timed out or failed
    OrchestratorTimeout,
    /// Download setup timed out or failed  
    DownloadTimeout,
    /// TUI initialization timed out or failed
    TuiTimeout,
    /// Background monitoring thread failed
    MonitoringFailure,
    /// Disk space check failed
    DiskSpaceCheckFailed,
    /// Network connectivity check failed
    ConnectivityCheckFailed,
    /// Progress monitoring detected stall
    ProgressStalled,
    /// App heartbeat check failed
    AppHeartbeatFailed,
    /// Retry limit exceeded
    RetryLimitExceeded,
}

impl HangError {
    /// Error messages using &'static str to avoid String allocation
    const ERROR_MESSAGES: [&'static str; 9] = [
        "Orchestrator creation timed out after 30 seconds",
        "Download setup timed out after 60 seconds",
        "TUI initialization timed out after 10 seconds",
        "Background monitoring thread encountered fatal error",
        "Disk space check failed - unable to access filesystem",
        "Network connectivity check failed - unable to reach HuggingFace",
        "Progress monitoring detected stalled download operation",
        "App heartbeat check failed - UI may be unresponsive",
        "Retry limit exceeded - operation failed after maximum attempts",
    ];

    /// Recovery suggestions using const arrays for stack allocation
    const RECOVERY_SUGGESTIONS: [&'static str; 9] = [
        "Check system resources and try again with console mode",
        "Verify network connectivity and model accessibility",
        "Ensure terminal supports TUI mode or use console fallback",
        "Check system permissions and resource availability",
        "Verify sufficient disk space and write permissions",
        "Check internet connection and DNS resolution",
        "Check bandwidth and retry download operation",
        "Force quit and restart application if unresponsive",
        "Wait before retrying or check error logs for details",
    ];

    /// Error codes for programmatic handling
    const ERROR_CODES: [u16; 9] = [
        1001, // OrchestratorTimeout
        1002, // DownloadTimeout
        1003, // TuiTimeout
        1004, // MonitoringFailure
        1005, // DiskSpaceCheckFailed
        1006, // ConnectivityCheckFailed
        1007, // ProgressStalled
        1008, // AppHeartbeatFailed
        1009, // RetryLimitExceeded
    ];

    /// Get error message with zero allocation
    #[inline]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::OrchestratorTimeout => Self::ERROR_MESSAGES[0],
            Self::DownloadTimeout => Self::ERROR_MESSAGES[1],
            Self::TuiTimeout => Self::ERROR_MESSAGES[2],
            Self::MonitoringFailure => Self::ERROR_MESSAGES[3],
            Self::DiskSpaceCheckFailed => Self::ERROR_MESSAGES[4],
            Self::ConnectivityCheckFailed => Self::ERROR_MESSAGES[5],
            Self::ProgressStalled => Self::ERROR_MESSAGES[6],
            Self::AppHeartbeatFailed => Self::ERROR_MESSAGES[7],
            Self::RetryLimitExceeded => Self::ERROR_MESSAGES[8],
        }
    }

    /// Get recovery suggestion with zero allocation
    #[inline]
    pub const fn recovery_suggestion(&self) -> &'static str {
        match self {
            Self::OrchestratorTimeout => Self::RECOVERY_SUGGESTIONS[0],
            Self::DownloadTimeout => Self::RECOVERY_SUGGESTIONS[1],
            Self::TuiTimeout => Self::RECOVERY_SUGGESTIONS[2],
            Self::MonitoringFailure => Self::RECOVERY_SUGGESTIONS[3],
            Self::DiskSpaceCheckFailed => Self::RECOVERY_SUGGESTIONS[4],
            Self::ConnectivityCheckFailed => Self::RECOVERY_SUGGESTIONS[5],
            Self::ProgressStalled => Self::RECOVERY_SUGGESTIONS[6],
            Self::AppHeartbeatFailed => Self::RECOVERY_SUGGESTIONS[7],
            Self::RetryLimitExceeded => Self::RECOVERY_SUGGESTIONS[8],
        }
    }

    /// Get error code for programmatic handling
    #[inline]
    pub const fn error_code(&self) -> u16 {
        match self {
            Self::OrchestratorTimeout => Self::ERROR_CODES[0],
            Self::DownloadTimeout => Self::ERROR_CODES[1],
            Self::TuiTimeout => Self::ERROR_CODES[2],
            Self::MonitoringFailure => Self::ERROR_CODES[3],
            Self::DiskSpaceCheckFailed => Self::ERROR_CODES[4],
            Self::ConnectivityCheckFailed => Self::ERROR_CODES[5],
            Self::ProgressStalled => Self::ERROR_CODES[6],
            Self::AppHeartbeatFailed => Self::ERROR_CODES[7],
            Self::RetryLimitExceeded => Self::ERROR_CODES[8],
        }
    }

    /// Check if error is recoverable with retry
    #[inline]
    pub const fn is_recoverable(&self) -> bool {
        match self {
            Self::OrchestratorTimeout => true,
            Self::DownloadTimeout => true,
            Self::TuiTimeout => false, // TUI issues require fallback, not retry
            Self::MonitoringFailure => false, // Monitoring failures are usually permanent
            Self::DiskSpaceCheckFailed => false, // Disk issues need manual intervention
            Self::ConnectivityCheckFailed => true,
            Self::ProgressStalled => true,
            Self::AppHeartbeatFailed => false, // App heartbeat failure requires restart
            Self::RetryLimitExceeded => false, // Already exceeded retries
        }
    }

    /// Check if error should trigger console mode fallback
    #[inline]
    pub const fn should_fallback_to_console(&self) -> bool {
        match self {
            Self::OrchestratorTimeout => true,
            Self::DownloadTimeout => false, // Download issues affect both TUI and console
            Self::TuiTimeout => true,
            Self::MonitoringFailure => false,
            Self::DiskSpaceCheckFailed => false,
            Self::ConnectivityCheckFailed => false,
            Self::ProgressStalled => false,
            Self::AppHeartbeatFailed => true,
            Self::RetryLimitExceeded => false,
        }
    }
}

impl fmt::Display for HangError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {}",
            self.error_code(),
            self.message(),
            self.recovery_suggestion()
        )
    }
}

impl std::error::Error for HangError {}

/// Convert from anyhow::Error to HangError for seamless integration
impl From<anyhow::Error> for HangError {
    #[inline]
    fn from(error: anyhow::Error) -> Self {
        // Check error string to categorize the anyhow error
        let error_str = error.to_string();

        if error_str.contains("timeout") || error_str.contains("Timeout") {
            if error_str.contains("orchestrator") || error_str.contains("Orchestrator") {
                Self::OrchestratorTimeout
            } else if error_str.contains("download") || error_str.contains("Download") {
                Self::DownloadTimeout
            } else if error_str.contains("tui") || error_str.contains("TUI") {
                Self::TuiTimeout
            } else {
                Self::OrchestratorTimeout // Default timeout error
            }
        } else if error_str.contains("network") || error_str.contains("connection") {
            Self::ConnectivityCheckFailed
        } else if error_str.contains("disk") || error_str.contains("space") {
            Self::DiskSpaceCheckFailed
        } else if error_str.contains("monitoring") || error_str.contains("monitor") {
            Self::MonitoringFailure
        } else {
            Self::OrchestratorTimeout // Default fallback
        }
    }
}

/// Stack-allocated error context for detailed debugging
#[derive(Copy, Clone, Debug)]
pub struct ErrorContext {
    /// Error type
    pub error: HangError,
    /// Timestamp when error occurred (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Operation that was being performed
    pub operation: &'static str,
    /// Additional context data
    pub context_data: [u64; 4],
}

impl ErrorContext {
    /// Create new error context with zero allocation
    #[inline]
    pub const fn new(error: HangError, operation: &'static str) -> Self {
        Self {
            error,
            timestamp_ms: 0, // Will be set by caller
            operation,
            context_data: [0; 4],
        }
    }

    /// Create with timestamp using current time
    #[inline]
    pub fn with_timestamp(error: HangError, operation: &'static str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            error,
            timestamp_ms,
            operation,
            context_data: [0; 4],
        }
    }

    /// Set context data for debugging
    #[inline]
    pub const fn with_context_data(mut self, data: [u64; 4]) -> Self {
        self.context_data = data;
        self
    }
}

impl fmt::Display for ErrorContext {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}ms] {} during '{}' - context: {:?}",
            self.timestamp_ms, self.error, self.operation, self.context_data
        )
    }
}

/// Result type alias for hang operations with zero allocation
pub type HangResult<T> = Result<T, HangError>;

/// Result type alias with error context for detailed debugging  
pub type HangResultContext<T> = Result<T, ErrorContext>;

/// Inline function to convert HangError to anyhow::Error for compatibility
#[inline]
pub fn hang_error_to_anyhow(error: HangError) -> anyhow::Error {
    anyhow::anyhow!("{}", error)
}

/// Inline function to create error context from operation
#[inline]
pub fn create_error_context(error: HangError, operation: &'static str) -> ErrorContext {
    ErrorContext::with_timestamp(error, operation)
}

/// Macro for creating error context with automatic operation name
#[macro_export]
macro_rules! hang_error_context {
    ($error:expr) => {
        $crate::errors::ErrorContext::with_timestamp($error, stringify!($error))
    };
    ($error:expr, $operation:expr) => {
        $crate::errors::ErrorContext::with_timestamp($error, $operation)
    };
    ($error:expr, $operation:expr, $context:expr) => {
        $crate::errors::ErrorContext::with_timestamp($error, $operation).with_context_data($context)
    };
}

/// Inline helper for timeout error creation
#[inline]
pub const fn timeout_error(operation: &'static str) -> ErrorContext {
    ErrorContext::new(HangError::OrchestratorTimeout, operation)
}

/// Inline helper for download error creation
#[inline]
pub const fn download_error(operation: &'static str) -> ErrorContext {
    ErrorContext::new(HangError::DownloadTimeout, operation)
}

/// Inline helper for TUI error creation
#[inline]
pub const fn tui_error(operation: &'static str) -> ErrorContext {
    ErrorContext::new(HangError::TuiTimeout, operation)
}
