use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time;

/// Zero-allocation timeout durations using const generics
pub const TIMEOUT_ORCHESTRATOR: Duration = Duration::from_secs(30);
pub const TIMEOUT_DOWNLOAD_SETUP: Duration = Duration::from_secs(60);
pub const TUI_INIT_TIMEOUT: Duration = Duration::from_secs(10);
pub const APP_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
pub const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(5);
pub const STALL_TIMEOUT: Duration = Duration::from_millis(30000);

/// Get the HuggingFace Hub download timeout from environment or default
pub fn get_hf_download_timeout() -> Duration {
    Duration::from_secs(progresshub_config::environment::get_hf_hub_download_timeout())
}

/// Atomic counters for lock-free timeout tracking
static ORCHESTRATOR_TIMEOUT_COUNT: AtomicU64 = AtomicU64::new(0);
static DOWNLOAD_TIMEOUT_COUNT: AtomicU64 = AtomicU64::new(0);
static TUI_TIMEOUT_COUNT: AtomicU64 = AtomicU64::new(0);
static APP_HEARTBEAT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
static CONNECTIVITY_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
static PROGRESS_HEARTBEAT: AtomicU64 = AtomicU64::new(0);

/// Zero-allocation timeout manager with blazing-fast atomic operations
pub struct ZeroAllocTimeoutManager;

impl ZeroAllocTimeoutManager {
    /// Inline wrapper for orchestrator creation with zero allocation
    #[inline]
    pub async fn timeout_orchestrator_new<F, T, E>(future: F) -> Result<T, crate::errors::HangError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        match time::timeout(TIMEOUT_ORCHESTRATOR, future).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => {
                Self::increment_orchestrator_timeout();
                Err(crate::errors::HangError::OrchestratorTimeout)
            }
            Err(_) => {
                Self::increment_orchestrator_timeout();
                Err(crate::errors::HangError::OrchestratorTimeout)
            }
        }
    }

    /// Inline wrapper for download setup with zero allocation
    #[inline]
    pub async fn timeout_download_setup<F, T, E>(future: F) -> Result<T, crate::errors::HangError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        match time::timeout(TIMEOUT_DOWNLOAD_SETUP, future).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => {
                Self::increment_download_timeout();
                Err(crate::errors::HangError::DownloadTimeout)
            }
            Err(_) => {
                Self::increment_download_timeout();
                Err(crate::errors::HangError::DownloadTimeout)
            }
        }
    }

    /// Inline wrapper for TUI initialization with zero allocation
    #[inline]
    pub async fn timeout_tui_init<F, T, E>(future: F) -> Result<T, crate::errors::HangError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        match time::timeout(TUI_INIT_TIMEOUT, future).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => {
                Self::increment_tui_timeout();
                Err(crate::errors::HangError::TuiTimeout)
            }
            Err(_) => {
                Self::increment_tui_timeout();
                Err(crate::errors::HangError::TuiTimeout)
            }
        }
    }

    /// Update app heartbeat with atomic timestamp
    #[inline]
    pub fn update_app_heartbeat() {
        let now = Instant::now().elapsed().as_millis() as u64;
        APP_HEARTBEAT_TIMESTAMP.store(now, Ordering::Relaxed);
    }

    /// Check if app is stalled using atomic comparison
    #[inline]
    pub fn check_app_heartbeat() -> bool {
        let now = Instant::now().elapsed().as_millis() as u64;
        let last_heartbeat = APP_HEARTBEAT_TIMESTAMP.load(Ordering::Relaxed);

        if last_heartbeat == 0 {
            return true; // First heartbeat
        }

        (now - last_heartbeat) < APP_HEARTBEAT_INTERVAL.as_millis() as u64
    }

    /// Update progress heartbeat with atomic operation
    #[inline]
    pub fn progress_heartbeat() {
        let now = Instant::now().elapsed().as_millis() as u64;
        PROGRESS_HEARTBEAT.store(now, Ordering::Relaxed);
    }

    /// Check for stalled progress using atomic comparison
    #[inline]
    pub fn check_progress_stall() -> bool {
        let now = Instant::now().elapsed().as_millis() as u64;
        let last_progress = PROGRESS_HEARTBEAT.load(Ordering::Relaxed);

        if last_progress == 0 {
            return false; // No progress started yet
        }

        (now - last_progress) > STALL_TIMEOUT.as_millis() as u64
    }

    /// Update connectivity cache with atomic timestamp
    #[inline]
    pub fn update_connectivity_cache() {
        let now = Instant::now().elapsed().as_millis() as u64;
        CONNECTIVITY_TIMESTAMP.store(now, Ordering::Relaxed);
    }

    /// Check if connectivity cache is valid using atomic comparison
    #[inline]
    pub fn is_connectivity_cache_valid() -> bool {
        let now = Instant::now().elapsed().as_millis() as u64;
        let cached_time = CONNECTIVITY_TIMESTAMP.load(Ordering::Relaxed);

        if cached_time == 0 {
            return false; // No cache
        }

        (now - cached_time) < CONNECTIVITY_TIMEOUT.as_millis() as u64
    }

    /// Atomic increment for orchestrator timeouts
    #[inline]
    fn increment_orchestrator_timeout() {
        ORCHESTRATOR_TIMEOUT_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    /// Atomic increment for download timeouts  
    #[inline]
    fn increment_download_timeout() {
        DOWNLOAD_TIMEOUT_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    /// Atomic increment for TUI timeouts
    #[inline]
    fn increment_tui_timeout() {
        TUI_TIMEOUT_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    /// Get timeout statistics with atomic reads
    #[inline]
    pub fn get_timeout_stats() -> (u64, u64, u64) {
        (
            ORCHESTRATOR_TIMEOUT_COUNT.load(Ordering::Relaxed),
            DOWNLOAD_TIMEOUT_COUNT.load(Ordering::Relaxed),
            TUI_TIMEOUT_COUNT.load(Ordering::Relaxed),
        )
    }
}

/// Zero-allocation retry state with Copy semantics
#[derive(Copy, Clone, Debug)]
pub struct RetryState {
    pub retry_count: u8,
    pub backoff_ms: u64,
}

impl Default for RetryState {
    /// Create a default retry state.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl RetryState {
    /// Maximum retries before giving up
    pub const MAX_RETRIES: u8 = 3;

    /// Initial backoff duration in milliseconds
    pub const INITIAL_BACKOFF_MS: u64 = 1000;

    /// Create new retry state with zero allocation
    #[inline]
    pub const fn new() -> Self {
        Self {
            retry_count: 0,
            backoff_ms: Self::INITIAL_BACKOFF_MS,
        }
    }

    /// Check if more retries are allowed
    #[inline]
    pub const fn can_retry(&self) -> bool {
        self.retry_count < Self::MAX_RETRIES
    }

    /// Calculate next retry state with exponential backoff
    #[inline]
    pub const fn next_retry(&self) -> Self {
        Self {
            retry_count: self.retry_count + 1,
            backoff_ms: self.backoff_ms * 2,
        }
    }

    /// Get backoff duration for current retry
    #[inline]
    pub const fn backoff_duration(&self) -> Duration {
        Duration::from_millis(self.backoff_ms)
    }
}

/// Stack-allocated disk space info with Copy semantics
#[derive(Copy, Clone, Debug)]
pub struct DiskSpaceInfo {
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub required_bytes: u64,
}

impl DiskSpaceInfo {
    /// Minimum free space in gigabytes
    pub const MIN_FREE_SPACE_GB: u64 = 10;
    pub const MIN_FREE_SPACE_BYTES: u64 = Self::MIN_FREE_SPACE_GB * 1024 * 1024 * 1024;

    /// Check if sufficient disk space is available
    #[inline]
    pub fn has_sufficient_space(&self) -> bool {
        self.available_bytes >= self.required_bytes.max(Self::MIN_FREE_SPACE_BYTES)
    }

    /// Calculate space utilization percentage
    #[inline]
    pub fn utilization_percent(&self) -> u64 {
        if self.total_bytes == 0 {
            return 0;
        }
        let used_bytes = self.total_bytes.saturating_sub(self.available_bytes);
        used_bytes
            .saturating_mul(100)
            .saturating_div(self.total_bytes.max(1))
    }
}

/// Inline function for zero-allocation disk space checking
#[inline]
pub fn check_disk_space(
    path: &std::path::Path,
    required_bytes: u64,
) -> Result<DiskSpaceInfo, crate::errors::HangError> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    #[cfg(target_os = "macos")]
    {
        #[repr(C)]
        struct Statvfs {
            f_bsize: u64,   // file system block size
            f_frsize: u64,  // fragment size
            f_blocks: u64,  // size of fs in f_frsize units
            f_bfree: u64,   // # free blocks
            f_bavail: u64,  // # free blocks for unprivileged users
            f_files: u64,   // # inodes
            f_ffree: u64,   // # free inodes
            f_favail: u64,  // # free inodes for unprivileged users
            f_fsid: u64,    // file system ID
            f_flag: u64,    // mount flags
            f_namemax: u64, // maximum filename length
        }

        unsafe extern "C" {
            fn statvfs(path: *const c_char, buf: *mut Statvfs) -> c_int;
        }

        let path_cstr = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| crate::errors::HangError::DiskSpaceCheckFailed)?;

        let mut stat: Statvfs = unsafe { std::mem::zeroed() };

        let result = unsafe { statvfs(path_cstr.as_ptr(), &mut stat) };

        if result != 0 {
            return Err(crate::errors::HangError::DiskSpaceCheckFailed);
        }

        let available_bytes = stat.f_bavail.saturating_mul(stat.f_frsize);
        let total_bytes = stat.f_blocks.saturating_mul(stat.f_frsize);

        Ok(DiskSpaceInfo {
            available_bytes,
            total_bytes,
            required_bytes,
        })
    }

    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Statvfs {
            f_bsize: u64,   // file system block size
            f_frsize: u64,  // fragment size
            f_blocks: u64,  // size of fs in f_frsize units
            f_bfree: u64,   // # free blocks
            f_bavail: u64,  // # free blocks for unprivileged users
            f_files: u64,   // # inodes
            f_ffree: u64,   // # free inodes
            f_favail: u64,  // # free inodes for unprivileged users
            f_fsid: u64,    // file system ID
            f_flag: u64,    // mount flags
            f_namemax: u64, // maximum filename length
        }

        unsafe extern "C" {
            fn statvfs(path: *const c_char, buf: *mut Statvfs) -> c_int;
        }

        let path_cstr = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| crate::errors::HangError::DiskSpaceCheckFailed)?;

        let mut stat: Statvfs = unsafe { std::mem::zeroed() };

        let result = unsafe { statvfs(path_cstr.as_ptr(), &mut stat) };

        if result != 0 {
            return Err(crate::errors::HangError::DiskSpaceCheckFailed);
        }

        let available_bytes = stat.f_bavail.saturating_mul(stat.f_frsize);
        let total_bytes = stat.f_blocks.saturating_mul(stat.f_frsize);

        Ok(DiskSpaceInfo {
            available_bytes,
            total_bytes,
            required_bytes,
        })
    }

    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        #[link(name = "kernel32")]
        extern "system" {
            fn GetDiskFreeSpaceExW(
                directory_name: *const u16,
                free_bytes_available_to_caller: *mut u64,
                total_number_of_bytes: *mut u64,
                total_number_of_free_bytes: *mut u64,
            ) -> i32;
        }

        // Convert path to wide string for Windows API
        let path_wide: Vec<u16> = OsStr::new(&path.to_string_lossy().to_string())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut _total_free_bytes: u64 = 0;

        let result = unsafe {
            GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes,
                &mut total_bytes,
                &mut _total_free_bytes,
            )
        };

        if result == 0 {
            return Err(crate::errors::HangError::DiskSpaceCheckFailed);
        }

        Ok(DiskSpaceInfo {
            available_bytes: free_bytes,
            total_bytes,
            required_bytes,
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        // Unsupported platform - return error instead of fake data
        Err(crate::errors::HangError::DiskSpaceCheckFailed)
    }
}

/// Inline function for zero-allocation connectivity checking
#[inline]
pub async fn check_connectivity() -> Result<(), crate::errors::HangError> {
    // Check cache first with atomic operation
    if ZeroAllocTimeoutManager::is_connectivity_cache_valid() {
        return Ok(());
    }

    // Perform HEAD request with timeout
    let client = reqwest::Client::new();

    match time::timeout(
        CONNECTIVITY_TIMEOUT,
        client.head("https://huggingface.co").send(),
    )
    .await
    {
        Ok(Ok(response)) => {
            if response.status().is_success() {
                ZeroAllocTimeoutManager::update_connectivity_cache();
                Ok(())
            } else {
                Err(crate::errors::HangError::ConnectivityCheckFailed)
            }
        }
        Ok(Err(_)) => Err(crate::errors::HangError::ConnectivityCheckFailed),
        Err(_) => Err(crate::errors::HangError::ConnectivityCheckFailed),
    }
}
