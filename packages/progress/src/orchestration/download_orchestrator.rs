//! Crossbeam/DashMap-based download orchestrator with priority queue
//!
//! Elegant actor-based architecture using crossbeam channels for bounded concurrency
//! and priority-based job scheduling with intelligent resource management.
//! Includes comprehensive worker thread health monitoring with automatic recovery.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicBool, Ordering as AtomicOrdering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use dashmap::DashMap;
use tokio::sync::Semaphore;

use crate::{types::DownloadConfig, manifest::{RepoManifest, FileInfo}};
use crate::memory_ordering::{
    MemoryOrderingValidator, CounterDirection, FetchOperationPurpose, SynchronizationPairType,
    documentation
};
use progresshub_common::RawDownloadEvent;
use std::sync::OnceLock;

static GLOBAL_ORCHESTRATOR: OnceLock<Arc<tokio::sync::Mutex<DownloadOrchestrator>>> = OnceLock::new();

/// Worker thread context containing shared state
#[derive(Clone)]
struct WorkerContext {
    download_states: Arc<DashMap<String, DownloadState>>,
    http_semaphore: Arc<Semaphore>,
    active_downloads: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
}

/// Configuration for download orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Number of worker threads (1-64)
    pub worker_count: usize,
    /// Maximum concurrent HTTP connections (1-256) 
    pub max_concurrent_http: usize,
    /// Job queue size for backpressure (1-10000)
    pub job_queue_size: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            worker_count: 16,
            max_concurrent_http: 32,
            job_queue_size: 1000,
        }
    }
}

impl OrchestratorConfig {
    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.worker_count == 0 || self.worker_count > 64 {
            return Err(anyhow::anyhow!("worker_count must be 1-64, got {}", self.worker_count));
        }
        if self.max_concurrent_http == 0 || self.max_concurrent_http > 256 {
            return Err(anyhow::anyhow!("max_concurrent_http must be 1-256, got {}", self.max_concurrent_http));
        }
        if self.job_queue_size == 0 || self.job_queue_size > 10000 {
            return Err(anyhow::anyhow!("job_queue_size must be 1-10000, got {}", self.job_queue_size));
        }
        Ok(())
    }
}

/// Get or create the global orchestrator instance
pub async fn get_global_orchestrator() -> anyhow::Result<Arc<tokio::sync::Mutex<DownloadOrchestrator>>> {
    if let Some(orchestrator) = GLOBAL_ORCHESTRATOR.get() {
        Ok(orchestrator.clone())
    } else {
        let config = OrchestratorConfig::default();
        let mut orchestrator = DownloadOrchestrator::new(config)?;
        orchestrator.start()?;
        
        let orchestrator = Arc::new(tokio::sync::Mutex::new(orchestrator));
        GLOBAL_ORCHESTRATOR.set(orchestrator.clone())
            .map_err(|_| anyhow::anyhow!("Failed to set global orchestrator"))?;
        
        tracing::info!("🚀 Global orchestrator initialized");
        Ok(orchestrator)
    }
}

/// Priority levels for download jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Small config files (< 1MB) - lowest priority
    Low = 0,
    /// Medium files (1MB - 100MB)
    Medium = 1,
    /// Large files (100MB - 1GB)
    High = 2,
    /// Huge files (> 1GB) - highest priority
    Critical = 3,
}

impl Priority {
    /// Determine priority based on file size
    pub fn from_file_size(size: u64) -> Self {
        match size {
            0..=1_048_576 => Priority::Low,        // < 1MB
            1_048_577..=104_857_600 => Priority::Medium,  // 1MB - 100MB
            104_857_601..=1_073_741_824 => Priority::High,  // 100MB - 1GB
            _ => Priority::Critical,                // > 1GB
        }
    }
}

/// Download job with prioritization
#[derive(Debug, Clone)]
pub struct DownloadJob {
    /// Job priority based on file size
    pub priority: Priority,
    /// Repository identifier
    pub repo_id: String,
    /// File information
    pub file_info: FileInfo,
    /// Local destination path
    pub destination: PathBuf,
    /// Download configuration
    pub config: DownloadConfig,
    /// Raw event sender for progress updates
    pub raw_sender: flume::Sender<RawDownloadEvent>,
    /// Optional completion sender for tracking download results
    pub completion_sender: Option<flume::Sender<DownloadResult>>,
    /// Job creation timestamp for scheduling
    pub created_at: Instant,
}

impl PartialEq for DownloadJob {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.file_info.size == other.file_info.size
    }
}

impl Eq for DownloadJob {}

impl PartialOrd for DownloadJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DownloadJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: Higher priority first
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // Secondary: Larger files first within same priority
                other.file_info.size.cmp(&self.file_info.size)
            }
            other_order => other_order
        }
    }
}

/// Download result from worker
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Job that was processed
    pub job: DownloadJob,
    /// Whether download succeeded
    pub success: bool,
    /// Number of bytes downloaded
    pub bytes_downloaded: u64,
    /// Download duration
    pub duration: Duration,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Whether the download was resumed from a previous attempt
    pub was_resumed: bool,
    /// Whether checksum validation was successful
    pub checksum_valid: bool,
}

/// Concurrent download state tracking
#[derive(Debug, Clone)]
pub struct DownloadState {
    /// Current status
    pub status: DownloadStatus,
    /// Bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total bytes expected
    pub total_bytes: u64,
    /// Download start time
    pub started_at: Option<Instant>,
    /// Worker thread ID processing this download
    pub worker_id: Option<usize>,
}

/// Download status enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    /// Job is queued waiting for worker
    Queued,
    /// Download is in progress
    InProgress,
    /// Download completed successfully
    Completed,
    /// Download failed with error
    Failed,
    /// Download was skipped (already exists)
    Skipped,
}

/// Worker thread health status with atomic tracking
#[derive(Debug)]
pub struct WorkerHealthStatus {
    /// Worker ID
    pub worker_id: usize,
    /// Last heartbeat timestamp (Unix timestamp in milliseconds)
    pub last_heartbeat: AtomicU64,
    /// Number of jobs processed by this worker
    pub jobs_processed: AtomicUsize,
    /// Number of jobs failed by this worker
    pub jobs_failed: AtomicUsize,
    /// Worker thread handle (None if worker has panicked and needs restart)
    pub thread_handle: parking_lot::Mutex<Option<thread::JoinHandle<()>>>,
    /// Worker is currently healthy
    pub is_healthy: AtomicBool,
    /// Worker restart count (increments on panic recovery)
    pub restart_count: AtomicUsize,
    /// Total uptime in milliseconds
    pub uptime_ms: AtomicU64,
    /// Worker start time
    pub started_at: parking_lot::Mutex<Instant>,
}

impl WorkerHealthStatus {
    /// Create new worker health status
    pub fn new(worker_id: usize) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            worker_id,
            last_heartbeat: AtomicU64::new(now_ms),
            jobs_processed: AtomicUsize::new(0),
            jobs_failed: AtomicUsize::new(0),
            thread_handle: parking_lot::Mutex::new(None),
            is_healthy: AtomicBool::new(true),
            restart_count: AtomicUsize::new(0),
            uptime_ms: AtomicU64::new(0),
            started_at: parking_lot::Mutex::new(Instant::now()),
        }
    }

    /// Update heartbeat with current timestamp
    /// 
    /// **Memory Ordering**: Uses `Relaxed` ordering because heartbeat timestamps are
    /// independent monitoring data that don't require synchronization with other
    /// memory operations. Only atomicity is needed for timestamp updates.
    #[inline]
    pub fn heartbeat(&self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        // Validate timestamp operation with relaxed ordering
        MemoryOrderingValidator::validate_relaxed_timestamp_operation(
            &self.last_heartbeat,
            "worker_heartbeat",
            now_ms,
        );
        
        // Store timestamp with relaxed ordering - see memory_ordering::documentation::heartbeat_timestamp_ordering()
        self.last_heartbeat.store(now_ms, documentation::heartbeat_timestamp_ordering());
    }

    /// Increment jobs processed counter
    /// 
    /// **Memory Ordering**: Uses `Relaxed` ordering because job counters are statistical
    /// data that don't affect control flow or synchronization. Only atomicity is needed.
    #[inline]
    pub fn job_completed(&self) {
        // Validate counter operation with relaxed ordering
        MemoryOrderingValidator::validate_relaxed_counter_operation(
            &self.jobs_processed,
            "job_completed",
            CounterDirection::Increment,
        );
        
        // Validate fetch operation ordering choice
        MemoryOrderingValidator::validate_fetch_operation_ordering(
            "jobs_processed_increment",
            documentation::job_counter_ordering(),
            FetchOperationPurpose::SimpleCounter,
        );
        
        // Increment with relaxed ordering - see memory_ordering::documentation::job_counter_ordering()
        self.jobs_processed.fetch_add(1, documentation::job_counter_ordering());
    }

    /// Increment jobs failed counter
    /// 
    /// **Memory Ordering**: Uses `Relaxed` ordering because job counters are statistical
    /// data that don't affect control flow or synchronization. Only atomicity is needed.
    #[inline]
    pub fn job_failed(&self) {
        // Validate counter operation with relaxed ordering
        MemoryOrderingValidator::validate_relaxed_counter_operation(
            &self.jobs_failed,
            "job_failed",
            CounterDirection::Increment,
        );
        
        // Validate fetch operation ordering choice
        MemoryOrderingValidator::validate_fetch_operation_ordering(
            "jobs_failed_increment",
            documentation::job_counter_ordering(),
            FetchOperationPurpose::SimpleCounter,
        );
        
        // Increment with relaxed ordering - see memory_ordering::documentation::job_counter_ordering()
        self.jobs_failed.fetch_add(1, documentation::job_counter_ordering());
    }

    /// Get current health metrics
    pub fn get_metrics(&self) -> WorkerHealthMetrics {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let last_heartbeat_ms = self.last_heartbeat.load(AtomicOrdering::Relaxed);
        let time_since_heartbeat = now_ms.saturating_sub(last_heartbeat_ms);

        WorkerHealthMetrics {
            worker_id: self.worker_id,
            // Health status uses relaxed ordering - exact ordering not critical for metrics snapshot
            is_healthy: self.is_healthy.load(AtomicOrdering::Relaxed),
            // Counter loads use relaxed ordering - statistical data doesn't require synchronization
            jobs_processed: self.jobs_processed.load(AtomicOrdering::Relaxed),
            jobs_failed: self.jobs_failed.load(AtomicOrdering::Relaxed),
            restart_count: self.restart_count.load(AtomicOrdering::Relaxed),
            time_since_heartbeat_ms: time_since_heartbeat,
            uptime_ms: self.uptime_ms.load(AtomicOrdering::Relaxed),
        }
    }

    /// Mark worker as unhealthy and prepare for restart
    /// 
    /// **Memory Ordering**: Uses `Release` ordering to ensure all prior operations
    /// (including health metrics updates) complete before the unhealthy status
    /// becomes visible to monitoring threads.
    pub fn mark_unhealthy(&self) {
        // Validate release store operation for state publication
        MemoryOrderingValidator::validate_release_store_operation(
            &self.is_healthy,
            "mark_worker_unhealthy",
            false,
        );
        
        // Validate synchronization relationship
        MemoryOrderingValidator::validate_synchronization_pair(
            SynchronizationPairType::ReleaseAcquire,
            "worker_health_status_update",
        );
        
        // Store unhealthy status with release ordering - see memory_ordering::documentation::health_status_publication_ordering()
        self.is_healthy.store(false, documentation::health_status_publication_ordering());
        
        // Take the thread handle to indicate it needs replacement
        let mut handle_guard = self.thread_handle.lock();
        if let Some(handle) = handle_guard.take() {
            // Thread handle taken - worker will be restarted
            drop(handle); // JoinHandle dropped, thread will be detached
        }
    }

    /// Mark worker as restarted and update counters
    /// 
    /// **Memory Ordering**: Uses `Relaxed` for counter increment and `Release` for
    /// health status to ensure restart completion is visible to monitoring threads.
    pub fn mark_restarted(&self, new_handle: thread::JoinHandle<()>) {
        // Validate counter operation for restart tracking
        MemoryOrderingValidator::validate_relaxed_counter_operation(
            &self.restart_count,
            "worker_restart",
            CounterDirection::Increment,
        );
        
        // Increment restart counter with relaxed ordering
        self.restart_count.fetch_add(1, documentation::job_counter_ordering());
        
        // Validate release store for health status publication
        MemoryOrderingValidator::validate_release_store_operation(
            &self.is_healthy,
            "mark_worker_healthy",
            true,
        );
        
        // Store healthy status with release ordering to publish restart completion
        self.is_healthy.store(true, documentation::health_status_publication_ordering());
        
        *self.started_at.lock() = Instant::now();
        *self.thread_handle.lock() = Some(new_handle);
        self.heartbeat(); // Initial heartbeat for restarted worker
    }

    /// Update uptime counter
    /// 
    /// **Memory Ordering**: Uses `Relaxed` ordering because uptime is monitoring
    /// data that doesn't require synchronization with other operations.
    pub fn update_uptime(&self) {
        let started_at = *self.started_at.lock();
        let uptime = started_at.elapsed().as_millis() as u64;
        
        // Validate timestamp operation for uptime tracking
        MemoryOrderingValidator::validate_relaxed_timestamp_operation(
            &self.uptime_ms,
            "worker_uptime_update",
            uptime,
        );
        
        // Store uptime with relaxed ordering - monitoring data only
        self.uptime_ms.store(uptime, AtomicOrdering::Relaxed);
    }
}

/// Worker health metrics snapshot
#[derive(Debug, Clone)]
pub struct WorkerHealthMetrics {
    pub worker_id: usize,
    pub is_healthy: bool,
    pub jobs_processed: usize,
    pub jobs_failed: usize,
    pub restart_count: usize,
    pub time_since_heartbeat_ms: u64,
    pub uptime_ms: u64,
}

impl WorkerHealthMetrics {
    /// Calculate job success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total_jobs = self.jobs_processed + self.jobs_failed;
        if total_jobs == 0 {
            100.0  // No jobs = perfect success rate
        } else {
            (self.jobs_processed as f64) / (total_jobs as f64) * 100.0
        }
    }

    /// Check if worker is responsive (heartbeat within threshold)
    pub fn is_responsive(&self, threshold_ms: u64) -> bool {
        self.time_since_heartbeat_ms <= threshold_ms
    }
}

/// Central worker health monitoring system
#[derive(Debug)]
pub struct WorkerHealthMonitor {
    /// Per-worker health status
    worker_health: Vec<Arc<WorkerHealthStatus>>,
    /// Health monitoring thread handle
    monitor_handle: parking_lot::Mutex<Option<thread::JoinHandle<()>>>,
    /// Health monitoring running flag
    monitoring_active: Arc<AtomicBool>,
    /// Health check interval
    check_interval: Duration,
    /// Heartbeat timeout threshold
    heartbeat_timeout: Duration,
}

impl WorkerHealthMonitor {
    /// Create new worker health monitor
    pub fn new(worker_count: usize) -> Self {
        let mut worker_health = Vec::with_capacity(worker_count);
        for worker_id in 0..worker_count {
            worker_health.push(Arc::new(WorkerHealthStatus::new(worker_id)));
        }

        Self {
            worker_health,
            monitor_handle: parking_lot::Mutex::new(None),
            monitoring_active: Arc::new(AtomicBool::new(false)),
            check_interval: Duration::from_secs(5), // Check every 5 seconds
            heartbeat_timeout: Duration::from_secs(30), // 30 second timeout
        }
    }

    /// Start health monitoring
    pub fn start_monitoring(&self) -> anyhow::Result<()> {
        // Check if monitoring is already active using acquire ordering
        // **Memory Ordering**: Acquire load ensures we see all initialization that
        // happened-before any previous activation signal
        if MemoryOrderingValidator::validate_acquire_load_operation(
            &self.monitoring_active,
            "health_monitoring_check"
        ) {
            return Ok(()); // Already running
        }

        // Validate release store for monitoring activation
        MemoryOrderingValidator::validate_release_store_operation(
            &self.monitoring_active,
            "health_monitoring_start",
            true,
        );
        
        // Validate synchronization relationship for monitoring startup
        MemoryOrderingValidator::validate_synchronization_pair(
            SynchronizationPairType::ReleaseAcquire,
            "health_monitoring_activation",
        );

        // Activate monitoring with release ordering to publish startup completion
        self.monitoring_active.store(true, documentation::health_status_publication_ordering());
        
        let worker_health = self.worker_health.clone();
        let monitoring_active = Arc::clone(&self.monitoring_active);
        let check_interval = self.check_interval;
        let heartbeat_timeout = self.heartbeat_timeout;

        let handle = thread::Builder::new()
            .name("worker-health-monitor".to_string())
            .spawn(move || {
                Self::health_monitoring_thread(
                    worker_health,
                    monitoring_active,
                    check_interval,
                    heartbeat_timeout,
                );
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn health monitoring thread: {}", e))?;

        *self.monitor_handle.lock() = Some(handle);
        tracing::info!("🏥 Worker health monitoring started");
        Ok(())
    }

    /// Stop health monitoring
    /// 
    /// **Memory Ordering**: Uses `Release` ordering to ensure all cleanup operations
    /// complete before the monitoring stop signal becomes visible to monitoring threads.
    pub fn stop_monitoring(&self) {
        // Validate release store for monitoring shutdown
        MemoryOrderingValidator::validate_release_store_operation(
            &self.monitoring_active,
            "health_monitoring_stop",
            false,
        );
        
        // Validate synchronization relationship for shutdown
        MemoryOrderingValidator::validate_synchronization_pair(
            SynchronizationPairType::ThreadShutdown,
            "health_monitoring_shutdown",
        );
        
        // Signal monitoring stop with release ordering to ensure proper shutdown coordination
        self.monitoring_active.store(false, documentation::shutdown_signal_ordering());
        
        if let Some(handle) = self.monitor_handle.lock().take()
            && let Err(e) = handle.join() {
                tracing::error!("❌ Health monitoring thread failed to join: {:?}", e);
            }
        
        tracing::info!("🛑 Worker health monitoring stopped");
    }

    /// Get health status for specific worker
    pub fn get_worker_health(&self, worker_id: usize) -> Option<Arc<WorkerHealthStatus>> {
        self.worker_health.get(worker_id).cloned()
    }

    /// Get overall health report
    pub fn get_health_report(&self) -> HealthReportCard {
        let mut total_workers = 0;
        let mut healthy_workers = 0;
        let mut total_jobs_processed = 0;
        let mut total_jobs_failed = 0;
        let mut total_restarts = 0;
        let mut workers_needing_restart = 0;

        let worker_metrics: Vec<WorkerHealthMetrics> = self.worker_health
            .iter()
            .map(|status| {
                let metrics = status.get_metrics();
                total_workers += 1;
                
                if metrics.is_healthy && metrics.is_responsive(self.heartbeat_timeout.as_millis() as u64) {
                    healthy_workers += 1;
                } else {
                    workers_needing_restart += 1;
                }
                
                total_jobs_processed += metrics.jobs_processed;
                total_jobs_failed += metrics.jobs_failed;
                total_restarts += metrics.restart_count;
                
                // Update uptime for this worker
                status.update_uptime();
                
                metrics
            })
            .collect();

        HealthReportCard {
            total_workers,
            healthy_workers,
            workers_needing_restart,
            total_jobs_processed,
            total_jobs_failed,
            total_restarts,
            worker_metrics,
        }
    }

    /// Health monitoring thread implementation
    fn health_monitoring_thread(
        worker_health: Vec<Arc<WorkerHealthStatus>>,
        monitoring_active: Arc<AtomicBool>,
        check_interval: Duration,
        heartbeat_timeout: Duration,
    ) {
        tracing::info!("🏥 Worker health monitoring thread started");
        
        while monitoring_active.load(AtomicOrdering::Acquire) {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            // Check each worker's health
            for status in &worker_health {
                let last_heartbeat = status.last_heartbeat.load(AtomicOrdering::Relaxed);
                let time_since_heartbeat = now_ms.saturating_sub(last_heartbeat);
                let timeout_ms = heartbeat_timeout.as_millis() as u64;
                
                if time_since_heartbeat > timeout_ms {
                    // Worker is unresponsive
                    if status.is_healthy.load(AtomicOrdering::Relaxed) {
                        tracing::warn!(
                            "🚨 Worker {} unresponsive ({}ms since heartbeat, timeout: {}ms)",
                            status.worker_id,
                            time_since_heartbeat,
                            timeout_ms
                        );
                        status.mark_unhealthy();
                    }
                } else if !status.is_healthy.load(AtomicOrdering::Relaxed) {
                    // Worker has recovered (should be handled by restart logic)
                    tracing::info!(
                        "💚 Worker {} heartbeat detected after restart",
                        status.worker_id
                    );
                }
                
                // Update uptime
                status.update_uptime();
            }

            // Sleep until next check
            thread::sleep(check_interval);
        }
        
        tracing::info!("🛑 Worker health monitoring thread stopped");
    }
}

/// Overall health report card
#[derive(Debug, Clone)]
pub struct HealthReportCard {
    pub total_workers: usize,
    pub healthy_workers: usize,
    pub workers_needing_restart: usize,
    pub total_jobs_processed: usize,
    pub total_jobs_failed: usize,
    pub total_restarts: usize,
    pub worker_metrics: Vec<WorkerHealthMetrics>,
}

impl HealthReportCard {
    /// Calculate overall system health percentage
    pub fn overall_health_percentage(&self) -> f64 {
        if self.total_workers == 0 {
            0.0
        } else {
            (self.healthy_workers as f64) / (self.total_workers as f64) * 100.0
        }
    }

    /// Calculate overall job success rate
    pub fn overall_success_rate(&self) -> f64 {
        let total_jobs = self.total_jobs_processed + self.total_jobs_failed;
        if total_jobs == 0 {
            100.0  // No jobs = perfect success rate
        } else {
            (self.total_jobs_processed as f64) / (total_jobs as f64) * 100.0
        }
    }

    /// Check if system is in critical state
    pub fn is_critical(&self) -> bool {
        self.overall_health_percentage() < 50.0 || self.workers_needing_restart >= self.total_workers / 2
    }

    /// Get health status summary
    pub fn status_summary(&self) -> String {
        if self.is_critical() {
            "CRITICAL".to_string()
        } else if self.workers_needing_restart > 0 {
            "WARNING".to_string()
        } else {
            "HEALTHY".to_string()
        }
    }
}

/// Download orchestrator with crossbeam channels and priority queue
pub struct DownloadOrchestrator {
    /// Job submission channel (bounded for backpressure)
    job_sender: Sender<DownloadJob>,
    /// Job receiver for workers
    job_receiver: Receiver<DownloadJob>,
    /// Result reporting channel
    result_sender: Sender<DownloadResult>,
    /// Result receiver for orchestrator
    result_receiver: Receiver<DownloadResult>,
    /// Priority queue for job scheduling
    priority_queue: Arc<parking_lot::Mutex<BinaryHeap<DownloadJob>>>,
    /// Concurrent state tracking
    download_states: Arc<DashMap<String, DownloadState>>,
    /// Model progress tracking
    model_progress: Arc<DashMap<String, ModelProgress>>,
    /// HTTP connection semaphore
    http_semaphore: Arc<Semaphore>,
    /// Worker thread handles
    worker_handles: Vec<thread::JoinHandle<()>>,
    /// Worker count
    worker_count: usize,
    /// Active downloads counter
    active_downloads: Arc<AtomicUsize>,
    /// Orchestrator running flag
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Worker health monitoring system
    health_monitor: Arc<WorkerHealthMonitor>,
}

/// Model progress tracking
#[derive(Debug, Clone)]
pub struct ModelProgress {
    /// Model identifier
    pub model_id: String,
    /// Total files in model
    pub total_files: usize,
    /// Completed files
    pub completed_files: usize,
    /// Total bytes in model
    pub total_bytes: u64,
    /// Downloaded bytes
    pub downloaded_bytes: u64,
    /// Model start time
    pub started_at: Instant,
}

impl DownloadOrchestrator {
    /// Create new download orchestrator with configuration
    pub fn new(config: OrchestratorConfig) -> anyhow::Result<Self> {
        config.validate()?;
        
        let (job_sender, job_receiver) = bounded(config.job_queue_size);
        let (result_sender, result_receiver) = unbounded();

        // Create health monitor for worker threads
        let health_monitor = Arc::new(WorkerHealthMonitor::new(config.worker_count));

        tracing::info!(
            "🚀 Creating DownloadOrchestrator: {} workers, {} max HTTP, {} queue size, health monitoring enabled",
            config.worker_count, config.max_concurrent_http, config.job_queue_size
        );

        Ok(Self {
            job_sender,
            job_receiver,
            result_sender,
            result_receiver,
            priority_queue: Arc::new(parking_lot::Mutex::new(BinaryHeap::new())),
            download_states: Arc::new(DashMap::new()),
            model_progress: Arc::new(DashMap::new()),
            http_semaphore: Arc::new(Semaphore::new(config.max_concurrent_http)),
            worker_handles: Vec::with_capacity(config.worker_count),
            worker_count: config.worker_count,
            active_downloads: Arc::new(AtomicUsize::new(0)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_monitor,
        })
    }

    /// Start the orchestrator and worker threads
    pub fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!("🚀 Starting DownloadOrchestrator with {} workers and health monitoring", self.worker_count);
        
        // Validate release store for orchestrator startup
        MemoryOrderingValidator::validate_release_store_operation(
            &self.running,
            "orchestrator_start",
            true,
        );
        
        // Start orchestrator with release ordering to publish startup completion
        self.running.store(true, documentation::shutdown_signal_ordering());

        // Start health monitoring first
        self.health_monitor.start_monitoring()?;

        // Start result processing task
        let result_receiver = self.result_receiver.clone();
        let running = Arc::clone(&self.running);
        let result_handle = thread::Builder::new()
            .name("download-result-processor".to_string())
            .spawn(move || {
                Self::result_processor_thread(result_receiver, running);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn result processor thread: {}", e))?;
        self.worker_handles.push(result_handle);

        // Start job scheduler task
        let priority_queue = Arc::clone(&self.priority_queue);
        let job_sender = self.job_sender.clone();
        let running = Arc::clone(&self.running);
        let scheduler_handle = thread::Builder::new()
            .name("job-scheduler".to_string())
            .spawn(move || {
                Self::job_scheduler_thread(priority_queue, job_sender, running);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn job scheduler thread: {}", e))?;
        self.worker_handles.push(scheduler_handle);

        // Start worker health management task
        let health_monitor = Arc::clone(&self.health_monitor);
        let job_receiver_for_restart = self.job_receiver.clone();
        let result_sender_for_restart = self.result_sender.clone();
        let download_states_for_restart = Arc::clone(&self.download_states);
        let http_semaphore_for_restart = Arc::clone(&self.http_semaphore);
        let active_downloads_for_restart = Arc::clone(&self.active_downloads);
        let running_for_restart = Arc::clone(&self.running);

        let health_management_handle = thread::Builder::new()
            .name("worker-health-management".to_string())
            .spawn(move || {
                Self::worker_health_management_thread(
                    health_monitor,
                    job_receiver_for_restart,
                    result_sender_for_restart,
                    download_states_for_restart,
                    http_semaphore_for_restart,
                    active_downloads_for_restart,
                    running_for_restart,
                );
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn worker health management thread: {}", e))?;
        self.worker_handles.push(health_management_handle);

        // Start worker threads with health monitoring integration
        for worker_id in 0..self.worker_count {
            let worker_health = self.health_monitor.get_worker_health(worker_id)
                .ok_or_else(|| anyhow::anyhow!("Failed to get health status for worker {}", worker_id))?;

            let handle = self.spawn_worker_thread(
                worker_id,
                Arc::clone(&worker_health),
            )?;

            // Register the handle with the health monitor
            *worker_health.thread_handle.lock() = Some(handle);
        }

        tracing::info!("✅ DownloadOrchestrator started with {} worker threads and health monitoring", self.worker_count);
        Ok(())
    }

    /// Spawn a single worker thread with health monitoring
    fn spawn_worker_thread(
        &self,
        worker_id: usize,
        worker_health: Arc<WorkerHealthStatus>,
    ) -> anyhow::Result<thread::JoinHandle<()>> {
        let job_receiver = self.job_receiver.clone();
        let result_sender = self.result_sender.clone();
        let context = WorkerContext {
            download_states: Arc::clone(&self.download_states),
            http_semaphore: Arc::clone(&self.http_semaphore),
            active_downloads: Arc::clone(&self.active_downloads),
            running: Arc::clone(&self.running),
        };

        let handle = thread::Builder::new()
            .name(format!("download-worker-{}", worker_id))
            .spawn(move || {
                Self::worker_thread_with_health_monitoring(
                    worker_id,
                    job_receiver,
                    result_sender,
                    context,
                    worker_health,
                );
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn worker thread {}: {}", worker_id, e))?;

        tracing::info!("🔧 Started worker {} with health monitoring", worker_id);
        Ok(handle)
    }

    /// Submit download jobs for a model
    pub fn submit_model_jobs(
        &self,
        repo_id: &str,
        manifest: &RepoManifest, 
        destination: &std::path::Path,
        config: &DownloadConfig,
        raw_sender: flume::Sender<RawDownloadEvent>
    ) -> anyhow::Result<()> {
        tracing::info!(
            "📋 Submitting {} download jobs for model: {}",
            manifest.files.len(),
            repo_id
        );

        // Initialize model progress tracking
        let model_progress = ModelProgress {
            model_id: repo_id.to_string(),
            total_files: manifest.files.len(),
            completed_files: 0,
            total_bytes: manifest.total_size,
            downloaded_bytes: 0,
            started_at: Instant::now(),
        };
        self.model_progress.insert(repo_id.to_string(), model_progress);

        // Create and submit jobs with priority
        let mut jobs: Vec<DownloadJob> = Vec::with_capacity(manifest.files.len());

        for file_info in &manifest.files {
            // Apply quantization filtering
            if let Some(ref quant) = config.quantization {
                let is_gguf_model = file_info.path.ends_with(".gguf") || file_info.path.contains(".gguf");
                if is_gguf_model && !file_info.path.contains(quant) {
                    tracing::debug!("⏭️ Skipping GGUF file {} (quantization filter: {})", file_info.path, quant);
                    continue;
                }
            }

            let file_destination = destination.join(&file_info.path);
            let priority = Priority::from_file_size(file_info.size);

            // DIAGNOSTIC: Track large file job creation
            if file_info.size > 1_000_000_000 {  // > 1GB
                tracing::info!(
                    "🔍 LARGE FILE JOB: Creating download job for {} ({} bytes, priority: {:?})",
                    file_info.path,
                    file_info.size,
                    priority
                );
            }

            let job = DownloadJob {
                priority,
                repo_id: repo_id.to_string(),
                file_info: file_info.clone(),
                destination: file_destination.clone(),
                config: config.clone(),
                raw_sender: raw_sender.clone(),
                completion_sender: None,
                created_at: Instant::now(),
            };

            // Initialize download state
            let state = DownloadState {
                status: DownloadStatus::Queued,
                bytes_downloaded: 0,
                total_bytes: file_info.size,
                started_at: None,
                worker_id: None,
            };
            
            let file_key = format!("{}:{}", repo_id, file_info.path);
            self.download_states.insert(file_key, state);

            jobs.push(job);
        }

        // Insert jobs into priority queue for proper scheduling
        let submitted = jobs.len();
        let mut large_files_queued = 0;
        {
            let mut queue = self.priority_queue.lock();
            for job in jobs {
                // DIAGNOSTIC: Track large file job queue insertion
                if job.file_info.size > 1_000_000_000 {  // > 1GB
                    large_files_queued += 1;
                    tracing::info!(
                        "🔍 LARGE FILE QUEUE: Inserting {} into priority queue (priority: {:?}, queue size before: {})",
                        job.file_info.path,
                        job.priority,
                        queue.len()
                    );
                }
                queue.push(job);
            }
        }

        tracing::info!(
            "✅ Queued {} prioritized download jobs for model: {} ({} large files >1GB, scheduler will process by priority)",
            submitted, repo_id, large_files_queued
        );

        Ok(())
    }

    /// Submit download jobs for a model with completion tracking
    pub fn submit_model_jobs_with_completion(
        &self,
        repo_id: &str,
        manifest: &RepoManifest, 
        destination: &std::path::Path,
        config: &DownloadConfig,
        raw_sender: flume::Sender<RawDownloadEvent>
    ) -> anyhow::Result<flume::Receiver<DownloadResult>> {
        tracing::info!(
            "📋 Submitting {} download jobs with completion tracking for model: {}",
            manifest.files.len(),
            repo_id
        );

        // Create completion channel for tracking results
        let (completion_sender, completion_receiver) = flume::unbounded();

        // Initialize model progress tracking
        let model_progress = ModelProgress {
            model_id: repo_id.to_string(),
            total_files: manifest.files.len(),
            completed_files: 0,
            total_bytes: manifest.total_size,
            downloaded_bytes: 0,
            started_at: Instant::now(),
        };
        self.model_progress.insert(repo_id.to_string(), model_progress);

        // Create and submit jobs with priority and completion tracking
        let mut jobs: Vec<DownloadJob> = Vec::with_capacity(manifest.files.len());

        for file_info in &manifest.files {
            // Apply quantization filtering
            if let Some(ref quant) = config.quantization {
                let is_gguf_model = file_info.path.ends_with(".gguf") || file_info.path.contains(".gguf");
                if is_gguf_model && !file_info.path.contains(quant) {
                    tracing::debug!("⏭️ Skipping GGUF file {} (quantization filter: {})", file_info.path, quant);
                    continue;
                }
            }

            let file_destination = destination.join(&file_info.path);
            let priority = Priority::from_file_size(file_info.size);

            let job = DownloadJob {
                priority,
                repo_id: repo_id.to_string(),
                file_info: file_info.clone(),
                destination: file_destination.clone(),
                config: config.clone(),
                raw_sender: raw_sender.clone(),
                completion_sender: Some(completion_sender.clone()),
                created_at: Instant::now(),
            };

            // Initialize download state
            let state = DownloadState {
                status: DownloadStatus::Queued,
                bytes_downloaded: 0,
                total_bytes: file_info.size,
                started_at: None,
                worker_id: None,
            };
            
            let file_key = format!("{}:{}", repo_id, file_info.path);
            self.download_states.insert(file_key, state);

            jobs.push(job);
        }

        // Sort jobs by priority (largest files first within priority levels)
        jobs.sort_by(|a, b| b.cmp(a));

        // Submit jobs through bounded channel (provides backpressure)
        let mut submitted = 0;
        for job in jobs {
            match self.job_sender.try_send(job.clone()) {
                Ok(()) => {
                    submitted += 1;
                }
                Err(crossbeam_channel::TrySendError::Full(job)) => {
                    tracing::warn!("📊 Job queue full, applying backpressure");
                    // Use blocking send for backpressure
                    if let Err(e) = self.job_sender.send(job) {
                        tracing::error!("❌ Failed to submit job: {}", e);
                        break;
                    }
                    submitted += 1;
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    tracing::error!("❌ Job channel disconnected");
                    break;
                }
            }
        }

        tracing::info!(
            "✅ Submitted {} prioritized download jobs with completion tracking for model: {} (priority: large files first)",
            submitted, repo_id
        );

        Ok(completion_receiver)
    }

    /// Get current download statistics with health information
    pub fn get_stats(&self) -> OrchestratorStats {
        let active_count = self.active_downloads.load(AtomicOrdering::Acquire);
        let total_jobs = self.download_states.len();
        
        let (completed, failed, in_progress, queued) = self.download_states
            .iter()
            .fold((0, 0, 0, 0), |(completed, failed, in_progress, queued), entry| {
                match entry.status {
                    DownloadStatus::Completed => (completed + 1, failed, in_progress, queued),
                    DownloadStatus::Failed => (completed, failed + 1, in_progress, queued),
                    DownloadStatus::InProgress => (completed, failed, in_progress + 1, queued),
                    DownloadStatus::Queued => (completed, failed, in_progress, queued + 1),
                    DownloadStatus::Skipped => (completed + 1, failed, in_progress, queued), // Count skipped as completed
                }
            });

        let health_report = self.get_health_report();

        OrchestratorStats {
            total_jobs,
            queued,
            in_progress,
            completed,
            failed,
            active_workers: active_count,
            total_workers: self.worker_count,
            health_report: Some(health_report),
        }
    }

    /// Get worker health report
    pub fn get_health_report(&self) -> HealthReportCard {
        self.health_monitor.get_health_report()
    }

    /// Get health status for specific worker
    pub fn get_worker_health(&self, worker_id: usize) -> Option<WorkerHealthMetrics> {
        self.health_monitor.get_worker_health(worker_id)
            .map(|status| status.get_metrics())
    }

    /// Worker thread implementation with health monitoring
    fn worker_thread_with_health_monitoring(
        worker_id: usize,
        job_receiver: Receiver<DownloadJob>,
        result_sender: Sender<DownloadResult>,
        context: WorkerContext,
        health_status: Arc<WorkerHealthStatus>,
    ) {
        tracing::info!("🔧 Worker {} started with health monitoring", worker_id);

        // Initial heartbeat
        health_status.heartbeat();

        // Create tokio runtime for async operations in worker thread
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("❌ Worker {} failed to create runtime: {}", worker_id, e);
                health_status.mark_unhealthy();
                return;
            }
        };

        // Heartbeat interval (send heartbeat every 10 seconds)
        let mut last_heartbeat = Instant::now();
        let heartbeat_interval = Duration::from_secs(10);

        // **Memory Ordering**: Acquire load ensures we observe shutdown signal with proper
        // ordering to see all cleanup operations that happened-before the shutdown
        while MemoryOrderingValidator::validate_acquire_load_operation(&context.running, "worker_shutdown_check") {
            // Send periodic heartbeat
            if last_heartbeat.elapsed() >= heartbeat_interval {
                health_status.heartbeat();
                last_heartbeat = Instant::now();
            }

            match job_receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(job) => {
                    let file_key = format!("{}:{}", job.repo_id, job.file_info.path);
                    
                    // DIAGNOSTIC: Track large file job reception by workers
                    if job.file_info.size > 1_000_000_000 {  // > 1GB
                        tracing::info!(
                            "🔍 LARGE FILE WORKER: Worker {} received large file job {} (priority: {:?}, size: {} bytes)",
                            worker_id,
                            job.file_info.path,
                            job.priority,
                            job.file_info.size
                        );
                    }
                    
                    // Update state to in-progress
                    if let Some(mut state) = context.download_states.get_mut(&file_key) {
                        state.status = DownloadStatus::InProgress;
                        state.started_at = Some(Instant::now());
                        state.worker_id = Some(worker_id);
                    }

                    // Increment active downloads counter with release ordering to publish state change
                    // **Memory Ordering**: Release ordering ensures download start is visible to monitoring
                    MemoryOrderingValidator::validate_fetch_operation_ordering(
                        "active_downloads_increment",
                        AtomicOrdering::Release,
                        FetchOperationPurpose::PublishingState,
                    );
                    context.active_downloads.fetch_add(1, AtomicOrdering::Release);

                    tracing::info!(
                        "🔄 Worker {} processing: {} (priority: {:?}, size: {} bytes)",
                        worker_id, job.file_info.path, job.priority, job.file_info.size
                    );

                    // DIAGNOSTIC: Track large file HTTP execution start
                    if job.file_info.size > 1_000_000_000 {  // > 1GB
                        tracing::info!(
                            "🔍 LARGE FILE HTTP: Worker {} starting HTTP download for {} (size: {} bytes)",
                            worker_id,
                            job.file_info.path,
                            job.file_info.size
                        );
                    }

                    // Process the download job using the runtime with inline HTTP client call
                    let result = rt.block_on(async {
                        let start_time = Instant::now();
                        
                        // Acquire HTTP semaphore permit
                        let _permit = match context.http_semaphore.acquire().await {
                            Ok(permit) => permit,
                            Err(e) => {
                                tracing::error!("❌ Worker {} failed to acquire HTTP permit: {}", worker_id, e);
                                return DownloadResult {
                                    job: job.clone(),
                                    success: false,
                                    bytes_downloaded: 0,
                                    duration: start_time.elapsed(),
                                    error_message: Some(format!("Failed to acquire HTTP permit: {}", e)),
                                    was_resumed: false,
                                    checksum_valid: false,
                                };
                            }
                        };

                        // Check if file already exists with correct size
                        if let Ok(metadata) = tokio::fs::metadata(&job.destination).await
                            && metadata.len() == job.file_info.size {
                                tracing::info!(
                                    "💾 Worker {} skipping existing file: {} ({} bytes)",
                                    worker_id, job.file_info.path, job.file_info.size
                                );
                                return DownloadResult {
                                    job: job.clone(),
                                    success: true,
                                    bytes_downloaded: job.file_info.size,
                                    duration: start_time.elapsed(),
                                    error_message: None,
                                    was_resumed: false,
                                    checksum_valid: true,
                                };
                            }

                        // Create parent directory
                        if let Some(parent) = job.destination.parent()
                            && let Err(e) = tokio::fs::create_dir_all(parent).await {
                                tracing::error!("❌ Worker {} failed to create directory {:?}: {}", worker_id, parent, e);
                                return DownloadResult {
                                    job: job.clone(),
                                    success: false,
                                    bytes_downloaded: 0,
                                    duration: start_time.elapsed(),
                                    error_message: Some(format!("Failed to create directory: {}", e)),
                                    was_resumed: false,
                                    checksum_valid: false,
                                };
                            }

                        // Perform actual download using HTTP client
                        let http_client = progresshub_client_http::HttpClient::default();
                        let quantization_owned = job.config.quantization.clone().unwrap_or_else(|| "unknown".to_string());

                        let http_config = progresshub_client_http::DownloadFileConfig {
                            path: &job.file_info.path,
                            destination: &job.destination,
                            expected_file_size: job.file_info.size,
                            expected_hash: job.file_info.hash.clone(),
                            remote_url: job.file_info.remote_url.clone(),
                            manifest_version: job.file_info.version_info.version_string(),
                            expected_chunk_size: job.file_info.version_info.chunk_size_hint,
                            progress_handler: Arc::new(progresshub_common::NoOpProgressHandler),
                            model_id: Some(job.repo_id.clone()),
                            raw_event_sender: Some(job.raw_sender.clone()),
                            quantization: quantization_owned,
                        };

                        match http_client.download_file_resumable(http_config) {
                            Ok(bytes_downloaded) => {
                                tracing::info!(
                                    "✅ Worker {} completed download: {} ({} bytes in {:?})",
                                    worker_id, job.file_info.path, bytes_downloaded, start_time.elapsed()
                                );
                                
                                DownloadResult {
                                    job: job.clone(),
                                    success: true,
                                    bytes_downloaded,
                                    duration: start_time.elapsed(),
                                    error_message: None,
                                    was_resumed: false,
                                    checksum_valid: true,
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ Worker {} download failed: {} - {}",
                                    worker_id, job.file_info.path, e
                                );
                                
                                DownloadResult {
                                    job: job.clone(),
                                    success: false,
                                    bytes_downloaded: 0,
                                    duration: start_time.elapsed(),
                                    error_message: Some(e.to_string()),
                                    was_resumed: false,
                                    checksum_valid: false,
                                }
                            }
                        }
                    });

                    // Update health metrics based on result
                    if result.success {
                        health_status.job_completed();
                    } else {
                        health_status.job_failed();
                    }

                    // DIAGNOSTIC: Track large file HTTP execution completion
                    if job.file_info.size > 1_000_000_000 {  // > 1GB
                        tracing::info!(
                            "🔍 LARGE FILE RESULT: Worker {} completed HTTP download for {} (success: {}, bytes: {})",
                            worker_id,
                            job.file_info.path,
                            result.success,
                            result.bytes_downloaded
                        );
                    }

                    // Update final state
                    if let Some(mut state) = context.download_states.get_mut(&file_key) {
                        state.status = if result.success {
                            DownloadStatus::Completed
                        } else {
                            DownloadStatus::Failed
                        };
                        state.bytes_downloaded = result.bytes_downloaded;
                    }

                    // Decrement active downloads counter with release ordering to publish completion
                    // **Memory Ordering**: Release ordering ensures download completion is visible to monitoring
                    MemoryOrderingValidator::validate_fetch_operation_ordering(
                        "active_downloads_decrement",
                        AtomicOrdering::Release,
                        FetchOperationPurpose::PublishingState,
                    );
                    context.active_downloads.fetch_sub(1, AtomicOrdering::Release);

                    // Report result to orchestrator
                    if let Err(e) = result_sender.send(result.clone()) {
                        tracing::error!("❌ Worker {} failed to send result: {}", worker_id, e);
                        health_status.job_failed();
                    }

                    // Report completion result if completion tracking is enabled
                    if let Some(ref completion_sender) = job.completion_sender
                        && let Err(e) = completion_sender.send(result) {
                            tracing::error!("❌ Worker {} failed to send completion result: {}", worker_id, e);
                            health_status.job_failed();
                        }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Normal timeout, continue loop (heartbeat will be sent on next iteration if needed)
                    continue;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    tracing::info!("🛑 Worker {} shutting down - channel disconnected", worker_id);
                    break;
                }
            }
        }

        // Final heartbeat before shutdown
        health_status.heartbeat();
        tracing::info!("✅ Worker {} finished with health monitoring", worker_id);
    }



    /// Stop the orchestrator and wait for workers to finish
    pub fn stop(&mut self) {
        tracing::info!("🛑 Stopping DownloadOrchestrator...");
        
        // Validate release store for orchestrator shutdown
        MemoryOrderingValidator::validate_release_store_operation(
            &self.running,
            "orchestrator_stop",
            false,
        );
        
        // Validate synchronization relationship for shutdown
        MemoryOrderingValidator::validate_synchronization_pair(
            SynchronizationPairType::ThreadShutdown,
            "orchestrator_shutdown",
        );
        
        // Signal shutdown with release ordering to ensure all cleanup operations complete
        self.running.store(false, documentation::shutdown_signal_ordering());

        // Stop health monitoring
        self.health_monitor.stop_monitoring();

        // Wait for all workers to finish
        while let Some(handle) = self.worker_handles.pop() {
            if let Err(e) = handle.join() {
                tracing::error!("❌ Worker thread failed to join: {:?}", e);
            }
        }

        tracing::info!("✅ DownloadOrchestrator stopped");
    }

    /// Worker health management thread - handles worker restart and recovery
    fn worker_health_management_thread(
        health_monitor: Arc<WorkerHealthMonitor>,
        job_receiver: Receiver<DownloadJob>,
        result_sender: Sender<DownloadResult>,
        download_states: Arc<DashMap<String, DownloadState>>,
        http_semaphore: Arc<Semaphore>,
        active_downloads: Arc<AtomicUsize>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        tracing::info!("🏥 Worker health management thread started");
        
        while running.load(AtomicOrdering::Acquire) {
            let health_report = health_monitor.get_health_report();
            
            // Check for unhealthy workers that need restart
            for metrics in &health_report.worker_metrics {
                if !metrics.is_healthy || !metrics.is_responsive(30_000) { // 30 second timeout
                    if let Some(worker_health) = health_monitor.get_worker_health(metrics.worker_id) {
                        // Check if worker handle is missing (indicates need for restart)
                        let needs_restart = {
                            let handle_guard = worker_health.thread_handle.lock();
                            handle_guard.is_none()
                        };
                        
                        if needs_restart {
                            tracing::warn!(
                                "🔄 Restarting unhealthy worker {} (restarts: {}, jobs processed: {})",
                                metrics.worker_id,
                                metrics.restart_count,
                                metrics.jobs_processed
                            );
                            
                            // Spawn replacement worker thread
                            let worker_id = metrics.worker_id;
                            let job_receiver_clone = job_receiver.clone();
                            let result_sender_clone = result_sender.clone();
                            let context_clone = WorkerContext {
                                download_states: Arc::clone(&download_states),
                                http_semaphore: Arc::clone(&http_semaphore),
                                active_downloads: Arc::clone(&active_downloads),
                                running: Arc::clone(&running),
                            };
                            let worker_health_clone = Arc::clone(&worker_health);
                            
                            match thread::Builder::new()
                                .name(format!("download-worker-{}-restart-{}", worker_id, metrics.restart_count + 1))
                                .spawn(move || {
                                    Self::worker_thread_with_health_monitoring(
                                        worker_id,
                                        job_receiver_clone,
                                        result_sender_clone,
                                        context_clone,
                                        worker_health_clone,
                                    );
                                }) {
                                Ok(new_handle) => {
                                    worker_health.mark_restarted(new_handle);
                                    tracing::info!(
                                        "✅ Successfully restarted worker {} (restart #{}) with health monitoring",
                                        worker_id,
                                        metrics.restart_count + 1
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "❌ Failed to restart worker {}: {}",
                                        worker_id,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
            
            // Log health summary periodically
            if health_report.is_critical() {
                tracing::error!(
                    "🚨 CRITICAL: Worker health at {:.1}% ({}/{} healthy, {} restarts total)",
                    health_report.overall_health_percentage(),
                    health_report.healthy_workers,
                    health_report.total_workers,
                    health_report.total_restarts
                );
            } else if health_report.workers_needing_restart > 0 {
                tracing::warn!(
                    "⚠️ WARNING: Worker health at {:.1}% ({} workers need restart)",
                    health_report.overall_health_percentage(),
                    health_report.workers_needing_restart
                );
            }
            
            // Sleep before next health check
            thread::sleep(Duration::from_secs(15)); // Check every 15 seconds
        }
        
        tracing::info!("🛑 Worker health management thread stopped");
    }
}

/// Orchestrator statistics with health information
#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    pub total_jobs: usize,
    pub queued: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub active_workers: usize,
    pub total_workers: usize,
    pub health_report: Option<HealthReportCard>,
}

impl OrchestratorStats {
    /// Get overall system health percentage
    pub fn system_health_percentage(&self) -> f64 {
        self.health_report
            .as_ref()
            .map(|report| report.overall_health_percentage())
            .unwrap_or(0.0)
    }

    /// Check if system is in critical state
    pub fn is_system_critical(&self) -> bool {
        self.health_report
            .as_ref()
            .map(|report| report.is_critical())
            .unwrap_or(true)
    }

    /// Get system status summary
    pub fn system_status(&self) -> String {
        self.health_report
            .as_ref()
            .map(|report| report.status_summary())
            .unwrap_or_else(|| "UNKNOWN".to_string())
    }
}

impl DownloadOrchestrator {
    /// Job scheduler thread that manages priority queue and feeds workers
    fn job_scheduler_thread(
        priority_queue: Arc<parking_lot::Mutex<BinaryHeap<DownloadJob>>>,
        job_sender: Sender<DownloadJob>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        tracing::info!("📋 Starting job scheduler thread");
        
        while running.load(AtomicOrdering::Acquire) {
            // Pop highest priority job from queue
            let job = {
                let mut queue = priority_queue.lock();
                let queue_size_before = queue.len();
                let job = queue.pop();
                
                // DIAGNOSTIC: Track large file job dequeue operations
                if let Some(ref job) = job
                    && job.file_info.size > 1_000_000_000 {  // > 1GB
                        tracing::info!(
                            "🔍 LARGE FILE DEQUEUE: Job scheduler popped {} from priority queue (priority: {:?}, size: {} bytes, queue size: {} -> {})",
                            job.file_info.path,
                            job.priority,
                            job.file_info.size,
                            queue_size_before,
                            queue.len()
                        );
                    }
                
                job
            };
            
            if let Some(job) = job {
                // DIAGNOSTIC: Track large file job sending to workers
                if job.file_info.size > 1_000_000_000 {  // > 1GB
                    tracing::info!(
                        "🔍 LARGE FILE SEND: Job scheduler sending {} to workers (priority: {:?})",
                        job.file_info.path,
                        job.priority
                    );
                }
                
                // Send job to workers with backpressure handling
                match job_sender.send(job.clone()) {
                    Ok(()) => {
                        // DIAGNOSTIC: Confirm large file job sent successfully
                        if job.file_info.size > 1_000_000_000 {  // > 1GB
                            tracing::info!(
                                "🔍 LARGE FILE SUCCESS: Job scheduler successfully sent {} to workers",
                                job.file_info.path
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Job scheduler failed to send job: {}", e);
                        break;
                    }
                }
            } else {
                // No jobs available, sleep briefly
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        
        tracing::info!("🛑 Job scheduler thread stopped");
    }

    /// Result processor thread that collects and processes worker results
    fn result_processor_thread(
        result_receiver: Receiver<DownloadResult>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        tracing::info!("🔄 Starting result processor thread");
        
        while running.load(AtomicOrdering::Acquire) {
            match result_receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(result) => {
                    if result.success {
                        tracing::debug!(
                            "✅ Result processor: {} completed ({} bytes, {:?})",
                            result.job.file_info.path,
                            result.bytes_downloaded,
                            result.duration
                        );
                    } else {
                        tracing::warn!(
                            "❌ Result processor: {} failed - {}",
                            result.job.file_info.path,
                            result.error_message.unwrap_or_else(|| "Unknown error".to_string())
                        );
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Normal timeout, continue loop
                    continue;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    tracing::info!("📡 Result processor: Channel disconnected, stopping");
                    break;
                }
            }
        }
        
        tracing::info!("🛑 Result processor thread stopped");
    }
}

impl Drop for DownloadOrchestrator {
    fn drop(&mut self) {
        if !self.worker_handles.is_empty() {
            tracing::warn!("🚨 DownloadOrchestrator dropped with active workers - forcing stop");
            self.stop();
        }
    }
}