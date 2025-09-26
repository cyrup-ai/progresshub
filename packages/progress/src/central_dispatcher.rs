//! Central intelligence dispatcher for Pure Flume Channel Event-Driven Architecture
//!
//! This module provides the central hub that:
//! 1. Receives RawDownloadEvent from XET/QUIC clients via flume channels
//! 2. Integrates with FilesystemProgressEvaluator for monotonic progress guarantees
//! 3. Uses ProgressCalculator for ALL formatting and calculations
//! 4. Implements debounced dispatch (100ms throttling with immediate after quietude)
//! 5. Sends ProgressCalculator snapshots to CLI/TUI via flume channels
//!
//! This is the ONLY location for progress intelligence in the system.

use crate::calculator::{ImmutableProgressState, ModelData, ProgressCalculator, TimingData};
use crate::filesystem_evaluator::FilesystemProgressEvaluator;
use flume::{Receiver, Sender};
use progresshub_common::{FinalizationLevel, ManifestEvent, ProgressEvent, RawDownloadEvent, SimpleRepoManifest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};

/// Central dispatcher configuration
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Debounce interval - maximum one event per interval during active periods
    pub debounce_interval: Duration,
    /// Timeout for filesystem evaluator operations
    pub filesystem_timeout: Duration,
    /// State directory for filesystem evaluator
    pub state_dir: PathBuf,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            debounce_interval: Duration::from_millis(100),
            filesystem_timeout: Duration::from_secs(5),
            state_dir: crate::filesystem_evaluator::get_default_state_dir(),
        }
    }
}

/// File progress state for calculations and formatting
#[derive(Debug, Clone)]
struct FileProgressState {
    /// Model identifier
    model_id: String,
    /// Quantization specification
    quant: String,
    /// Local filepath
    local_filepath: String,
    /// Bytes downloaded (validated by filesystem evaluator)
    bytes_downloaded: u64,
    /// Total bytes expected
    total_bytes: u64,
    /// When this download started (never changes after creation)
    start_time: Instant,
    /// Last update timestamp for speed calculations
    last_update: Instant,
    /// Whether file is served from cache
    from_cache: bool,
    /// Completed ranges for this file (start_byte, end_byte)
    ranges_completed: Vec<(u64, u64)>,
    /// Whether this file has been finalized (.part moved to final)
    is_file_finalized: bool,
}

/// Central progress dispatcher - the intelligence hub of the system
pub struct CentralProgressDispatcher {
    /// Configuration
    config: DispatcherConfig,
    /// Filesystem evaluator for monotonic progress guarantees
    filesystem_evaluator: FilesystemProgressEvaluator,
    /// Current file progress states
    file_states: HashMap<String, FileProgressState>,
    /// Last dispatch time for debouncing
    last_dispatch: Option<Instant>,
    /// ProgressCalculator snapshot sender to CLI/TUI
    progress_sender: Sender<ProgressCalculator>,
    /// Manifest information for accurate progress calculation
    manifest_info: Option<ManifestEvent>,
    /// Detailed repository manifest with per-file sizes for accurate progress calculation
    repo_manifest: Option<SimpleRepoManifest>,
    /// Last filesystem scan time for debounced scanning
    last_scan_time: Option<Instant>,
    /// Whether a scan is pending for the next debounced interval
    pending_scan: bool,
    /// Scan interval for debounced filesystem scanning
    scan_interval: Duration,
    /// Latest comprehensive filesystem scan results (single source of truth)
    latest_scan_results: Option<crate::filesystem_evaluator::ComprehensiveProgressScan>,
    /// Count of consecutive scan failures for circuit breaker logic
    consecutive_scan_failures: u32,
    /// Maximum consecutive failures before circuit breaker activates
    max_scan_failures: u32,
    /// List of requested model IDs to scan (filters out other cached models)
    requested_models: Vec<String>,
}

impl CentralProgressDispatcher {
    /// Create new central progress dispatcher
    ///
    /// # Arguments
    /// * `config` - Dispatcher configuration
    /// * `progress_sender` - Flume sender for ProgressCalculator snapshots to CLI/TUI
    /// * `requested_models` - List of model IDs to scan (filters filesystem scanning to requested models only)
    ///
    /// # Returns
    /// New dispatcher ready to process raw events
    pub fn new(config: DispatcherConfig, progress_sender: Sender<ProgressCalculator>, requested_models: Vec<String>) -> Self {
        let filesystem_evaluator = FilesystemProgressEvaluator::new();

        info!(
            "Creating central progress dispatcher with debounce interval: {:?}, requested models: {:?}",
            config.debounce_interval,
            requested_models
        );

        Self {
            config,
            filesystem_evaluator,
            file_states: HashMap::new(),
            last_dispatch: None,
            progress_sender,
            manifest_info: None,
            repo_manifest: None,
            last_scan_time: None,
            pending_scan: false,
            scan_interval: Duration::from_millis(100),
            latest_scan_results: None,
            consecutive_scan_failures: 0,
            max_scan_failures: 5, // Circuit breaker activates after 5 consecutive failures
            requested_models,
        }
    }

    /// Start the central dispatcher event loop
    ///
    /// Processes manifest and raw download events from orchestration and XET/QUIC clients,
    /// validates progress via filesystem evaluator, calculates all formatting, and dispatches
    /// formatted events to CLI/TUI with debounced throttling.
    ///
    /// # Arguments
    /// * `event_receiver` - Flume receiver for ProgressEvents (manifest and download events)
    ///
    /// # Errors
    /// Returns error if event processing fails or channels are disconnected
    pub async fn run_event_loop(
        &mut self,
        event_receiver: Receiver<ProgressEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 CENTRAL DISPATCHER: Starting central progress dispatcher event loop");

        // Send initial empty progress snapshot so CLI has something to display
        self.dispatch_progress_snapshots().await;

        loop {
            // Receive progress event (manifest or download) from orchestration/clients
            tracing::debug!("🔄 CENTRAL DISPATCHER: Waiting for ProgressEvent...");
            match event_receiver.recv_async().await {
                Ok(event) => {
                    tracing::debug!(
                        "📨 CENTRAL DISPATCHER: Received ProgressEvent: {:?}",
                        std::mem::discriminant(&event)
                    );
                    match event {
                        ProgressEvent::Manifest(manifest_event) => {
                            let total_bytes = manifest_event.total_bytes;
                            tracing::debug!(
                                "Manifest received: model={}, files={}, bytes={}",
                                manifest_event.model_id,
                                manifest_event.total_files,
                                total_bytes
                            );

                            // Store manifest information for accurate progress calculation
                            self.manifest_info = Some(manifest_event);

                            tracing::debug!(
                                "Manifest stored - progress calculation ready with {} total bytes",
                                total_bytes
                            );

                            // Dispatch initial progress snapshot with manifest information
                            self.dispatch_progress_snapshots().await;
                        }
                        ProgressEvent::RepoManifest(repo_manifest) => {
                            let total_bytes = repo_manifest.total_size;
                            let file_count = repo_manifest.files.len();
                            tracing::info!(
                                "📋 RepoManifest received: model={}, files={}, bytes={}, quant={}",
                                repo_manifest.repo_id,
                                file_count,
                                total_bytes,
                                repo_manifest.quant
                            );

                            // Store detailed repository manifest for per-file progress calculation
                            self.repo_manifest = Some(repo_manifest);

                            tracing::info!(
                                "✅ RepoManifest stored - per-file progress calculation ready with {} files totaling {} bytes",
                                file_count,
                                total_bytes
                            );

                            // Dispatch initial progress snapshot with detailed manifest information
                            self.dispatch_progress_snapshots().await;
                        }
                        ProgressEvent::Download(raw_event) => {
                            // Check for AllModelsComplete finalization
                            if let Some(progresshub_common::FinalizationLevel::AllModelsComplete) =
                                &raw_event.finalization
                            {
                                info!("🎉 AllModelsComplete received - terminating event loop");
                                // Process this final event then break
                                if let Err(e) = self.update_progress_state_only(&raw_event).await {
                                    error!("Failed to update progress state for final raw event: {}", e);
                                }
                                if let Some(finalization) = &raw_event.finalization {
                                    let file_key = raw_event.file_key();
                                    if let Err(e) = self
                                        .process_finalization_event(
                                            finalization,
                                            &file_key,
                                            &raw_event,
                                        )
                                        .await
                                    {
                                        error!("Failed to process final finalization event: {}", e);
                                    }
                                }
                                self.apply_debounced_dispatch().await;
                                break;
                            }

                            tracing::info!(
                                "🔵 CENTRAL DISPATCHER: Processing download event: model={}, file={}, bytes={}/{}, finalization={:?}",
                                raw_event.model_id,
                                raw_event.local_filepath,
                                raw_event.bytes_downloaded,
                                raw_event.total_bytes,
                                raw_event
                                    .finalization
                                    .as_ref()
                                    .map(|f| std::mem::discriminant(f))
                            );

                            // ALWAYS process progress state updates first - never block these
                            if let Err(e) = self.update_progress_state_only(&raw_event).await {
                                error!("Failed to update progress state: {}", e);
                                // Continue anyway - progress updates should not fail
                            }

                            // Process finalization events separately - failures don't block progress
                            if let Some(finalization) = &raw_event.finalization {
                                let file_key = raw_event.file_key();
                                if let Err(e) = self
                                    .process_finalization_event(finalization, &file_key, &raw_event)
                                    .await
                                {
                                    error!("Finalization failed (not blocking progress): {}", e);
                                    // Log but don't block progress dispatch
                                }
                            }

                            // ALWAYS apply debounced dispatch regardless of validation results
                            self.apply_debounced_dispatch().await;
                        }
                    }
                }
                Err(flume::RecvError::Disconnected) => {
                    info!("Event channel disconnected - terminating event loop");
                    break;
                }
            }
        }

        info!("Central progress dispatcher event loop terminated");
        Ok(())
    }

    /// Update progress state only (separated from completion validation for lock-tight design)
    ///
    /// This method ONLY updates progress state and never fails due to validation issues.
    /// It's separated from completion validation to ensure progress updates always flow.
    async fn update_progress_state_only(
        &mut self,
        raw_event: &RawDownloadEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file_key = raw_event.file_key();

        // TODO: Validation will be moved to debounced scanning - use raw event data directly for now
        let validated_progress = crate::central_dispatcher::ValidatedProgress {
            bytes_downloaded: raw_event.bytes_downloaded,
            total_bytes: raw_event.total_bytes,
            from_cache: false, // Will be determined by debounced scanning
        };

        debug!(
            "Using event data directly for {}: {}/{} bytes (debounced validation pending)",
            file_key, validated_progress.bytes_downloaded, validated_progress.total_bytes
        );

        // Update or create file progress state
        let now = Instant::now();

        // Only create new state if file doesn't exist - preserves ranges_completed and start_time
        self.file_states
            .entry(file_key.clone())
            .or_insert_with(|| FileProgressState {
                model_id: raw_event.model_id.clone(),
                quant: raw_event.quant.clone(),
                local_filepath: raw_event.local_filepath.clone(),
                bytes_downloaded: 0,
                total_bytes: raw_event.total_bytes,
                start_time: now, // Only set on initial creation
                last_update: now,
                from_cache: validated_progress.from_cache,
                ranges_completed: Vec::new(), // Only empty on initial creation
                is_file_finalized: false,
            });

        // Update state with validated progress (monotonic guarantee)
        // This preserves ranges_completed and start_time from existing state
        if let Some(file_state) = self.file_states.get_mut(&file_key) {
            file_state.bytes_downloaded = validated_progress.bytes_downloaded;
            file_state.total_bytes = validated_progress.total_bytes; // Filesystem evaluator is authoritative
            file_state.last_update = now;
            file_state.from_cache = validated_progress.from_cache;
            // NOTE: Intentionally NOT updating start_time or ranges_completed here

            // Mark cached files as finalized since they don't go through download completion
            if validated_progress.from_cache {
                file_state.is_file_finalized = true;
            }

            // Check for file completion and trigger validation/rename
            let needs_completion = !file_state.is_file_finalized
                && file_state.bytes_downloaded >= file_state.total_bytes
                && file_state.total_bytes > 0;

            tracing::debug!(
                "🔍 COMPLETION CHECK: {} - finalized={}, downloaded={}, total={}, needs_completion={}",
                file_key,
                file_state.is_file_finalized,
                file_state.bytes_downloaded,
                file_state.total_bytes,
                needs_completion
            );

            if needs_completion {
                tracing::info!(
                    "🔍 File download complete, starting validation: {} ({}/{} bytes)",
                    file_key,
                    file_state.bytes_downloaded,
                    file_state.total_bytes
                );
            }
        } else {
            warn!("File state not found for key: {}", file_key);
        }

        // REMOVED: Automatic completion detection based on size comparison
        // This was causing premature hash validation during downloads.
        // Completion is now only triggered by explicit FileComplete finalization events
        // from the HTTP client when all chunks are actually written and synced.

        // Note: Finalization events are now processed separately in the main event loop
        // to prevent blocking progress updates when validation fails

        Ok(())
    }



    /// Apply debounced dispatch logic
    ///
    /// Implements 100ms throttling with immediate dispatch after quietude
    /// for lightning-fast responsiveness while preventing UI spam.
    /// Apply debounced dispatch logic with filesystem scanning
    ///
    /// Implements proper debounced logic with 4 guarantees:
    /// 1. No ticking when no events (reactive only)
    /// 2. Maximum 1 scan per interval period (prevents flooding)
    /// 3. Immediate scan after quietude (zero delay responsiveness)
    /// 4. Every event guarantees at least 1 future scan (reliability)
    async fn apply_debounced_dispatch(&mut self) {
        let now = Instant::now();

        // Determine if we should scan the filesystem
        let should_scan = match self.last_scan_time {
            None => {
                // First event - trigger immediate scan for zero delay
                debug!("🚀 DEBOUNCED: First event - triggering immediate filesystem scan");
                true
            }
            Some(last_scan) => {
                let scan_elapsed = now.duration_since(last_scan);
                if scan_elapsed >= self.scan_interval {
                    // Scan interval passed - trigger immediate scan
                    debug!(
                        "⏰ DEBOUNCED: Scan interval passed ({:?}) - triggering filesystem scan", 
                        scan_elapsed
                    );
                    true
                } else {
                    // Within scan interval - mark scan as pending but don't scan yet
                    let remaining = self.scan_interval - scan_elapsed;
                    debug!(
                        "⏸️ DEBOUNCED: Within scan interval - marking scan pending (remaining: {:?})",
                        remaining
                    );
                    self.pending_scan = true;
                    false
                }
            }
        };

        // Perform filesystem scan if needed (with circuit breaker logic)
        if should_scan {
            // Check circuit breaker - skip scan if too many consecutive failures
            if self.consecutive_scan_failures >= self.max_scan_failures {
                warn!(
                    "🚨 CIRCUIT BREAKER: Skipping filesystem scan due to {} consecutive failures (max: {})",
                    self.consecutive_scan_failures, self.max_scan_failures
                );
                debug!("🚨 CIRCUIT BREAKER: System will continue using last known scan results or manifest data");
                
                // Don't update last_scan_time to allow retry on next interval after cooldown
                self.pending_scan = true;
            } else {
                debug!(
                    "🔍 DEBOUNCED: Starting comprehensive filesystem scan (failure count: {}/{})",
                    self.consecutive_scan_failures, self.max_scan_failures
                );
                
                // Perform the comprehensive filesystem scan for requested models only
                match self.filesystem_evaluator.scan_all_progress(&self.requested_models).await {
                    Ok(scan_results) => {
                        info!(
                            "✅ DEBOUNCED: Filesystem scan successful - {} models, {}/{} bytes",
                            scan_results.models.len(),
                            scan_results.overall_bytes_downloaded,
                            scan_results.overall_total_bytes
                        );
                        
                        // Reset circuit breaker on successful scan
                        self.consecutive_scan_failures = 0;
                        
                        // Store scan results as single source of truth and update state
                        self.latest_scan_results = Some(scan_results);
                        self.last_scan_time = Some(now);
                        self.pending_scan = false;
                    },
                    Err(e) => {
                        // Increment failure count for circuit breaker
                        self.consecutive_scan_failures += 1;
                        
                        error!(
                            "❌ DEBOUNCED: Filesystem scan failed ({}/{}): {} - Error type: {}, Will retry on next interval",
                            self.consecutive_scan_failures, self.max_scan_failures, e, 
                            if e.to_string().contains("permission") { "PERMISSION_DENIED" }
                            else if e.to_string().contains("not found") { "PATH_NOT_FOUND" }
                            else if e.to_string().contains("timeout") { "TIMEOUT" }
                            else { "UNKNOWN" }
                        );
                        
                        // Provide actionable debugging information
                        if self.consecutive_scan_failures == 1 {
                            warn!(
                                "💡 SCAN TROUBLESHOOTING: Check filesystem permissions and HuggingFace cache directory access"
                            );
                        } else if self.consecutive_scan_failures == self.max_scan_failures {
                            error!(
                                "🚨 CIRCUIT BREAKER ACTIVATED: Filesystem scanning disabled after {} failures. System will use fallback progress calculation.",
                                self.max_scan_failures
                            );
                        }
                        
                        // Don't update last_scan_time on failure - allows retry on next event
                        self.pending_scan = true;
                    }
                }
            }
        }

        // Always dispatch UI updates (separate from filesystem scanning)
        let should_dispatch_ui = match self.last_dispatch {
            None => {
                // First event - dispatch immediately
                debug!("🚀 DEBOUNCED: First event - dispatching UI immediately");
                true
            }
            Some(last_dispatch) => {
                let dispatch_elapsed = now.duration_since(last_dispatch);
                if dispatch_elapsed >= self.config.debounce_interval {
                    // UI dispatch interval passed - dispatch immediately  
                    debug!("⏰ DEBOUNCED: UI dispatch interval passed - dispatching");
                    true
                } else {
                    // Within UI dispatch interval - skip to prevent spam
                    let remaining = self.config.debounce_interval - dispatch_elapsed;
                    debug!(
                        "⏸️ DEBOUNCED: Within UI dispatch interval - skipping (remaining: {:?})",
                        remaining
                    );
                    false
                }
            }
        };

        if should_dispatch_ui {
            self.dispatch_progress_snapshots().await;
            self.last_dispatch = Some(now);
        }

        debug!(
            "🎯 DEBOUNCED: Event processed - scanned: {}, dispatched: {}, pending_scan: {}",
            should_scan, should_dispatch_ui, self.pending_scan
        );
    }

    /// Check if all downloads are complete

    /// Dispatch ProgressCalculator snapshots to CLI/TUI
    ///
    /// Creates ProgressCalculator snapshots with ALL formatting methods and dispatches
    /// via flume channels. CLI/TUI perform ZERO calculations.
    async fn dispatch_progress_snapshots(&self) {
        // Create ProgressCalculator snapshot from current state
        let progress_snapshot = self.create_progress_snapshot();

        tracing::info!(
            "🚀 CENTRAL DISPATCHER: Creating and dispatching ProgressCalculator snapshot: {}% complete",
            progress_snapshot.percentage_formatted()
        );

        // Dispatch ProgressCalculator snapshot to CLI/TUI
        match self
            .progress_sender
            .send_async(progress_snapshot.clone())
            .await
        {
            Ok(()) => {
                tracing::debug!(
                    "✅ CENTRAL DISPATCHER: Successfully dispatched ProgressCalculator snapshot: downloaded={}, speed={}",
                    progress_snapshot.bytes_formatted(),
                    progress_snapshot.speed_formatted()
                );
            }
            Err(e) => {
                tracing::error!(
                    "❌ CENTRAL DISPATCHER: Failed to send ProgressCalculator snapshot: {}",
                    e
                );
            }
        }
    }

    /// Create ProgressCalculator snapshot from current state
    ///
    /// Creates immutable ProgressCalculator with ALL formatting methods.
    /// ProgressCalculator IS the event that flows through flume channels.
    fn create_progress_snapshot(&self) -> ProgressCalculator {
        // Create ImmutableProgressState from current file states
        let progress_state = self.build_immutable_progress_state();

        // Calculate current speed across all downloads
        let current_speed = self.calculate_overall_speed();

        // Create timing data
        let timing_data = TimingData::new(self.get_earliest_start_time(), current_speed);

        // Create model data for current state
        let model_data = self.build_model_data();

        // Extract quantization from first file state (or default)
        let quantization = self
            .file_states
            .values()
            .next()
            .map(|state| state.quant.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        ProgressCalculator::new(progress_state, quantization, timing_data, model_data)
    }

    /// Build ImmutableProgressState from comprehensive filesystem scan results
    ///
    /// Uses latest_scan_results as single source of truth for all 4 progress levels.
    /// Falls back to empty state if no scan results are available (scan not yet performed).
    fn build_immutable_progress_state(&self) -> ImmutableProgressState {
        use crate::calculator::{ImmutableFileProgress, ImmutableModelProgress};
        use im::{OrdMap, Vector};
        use std::time::SystemTime;

        // Use latest filesystem scan results as single source of truth
        if let Some(scan_results) = &self.latest_scan_results {
            tracing::debug!(
                "🔍 SCAN RESULTS: Building progress state from filesystem scan with {} models, {} overall bytes downloaded, {} overall total bytes",
                scan_results.models.len(),
                scan_results.overall_bytes_downloaded,
                scan_results.overall_total_bytes
            );

            let mut models = Vector::new();

            // Convert each ModelProgressData to ImmutableModelProgress
            for (model_id, model_data) in &scan_results.models {
                let mut files = Vector::new();

                tracing::debug!(
                    "Processing model from scan: {}, {} files, {} bytes downloaded, {} total bytes, cached: {}",
                    model_id,
                    model_data.files.len(),
                    model_data.model_bytes_downloaded,
                    model_data.model_total_bytes,
                    model_data.is_model_cached
                );

                // Convert each FileProgressData to ImmutableFileProgress
                for (file_path, file_data) in &model_data.files {
                    // Use expected file size from SimpleRepoManifest for accurate progress calculation
                    let expected_file_size = if let Some(ref repo_manifest) = self.repo_manifest {
                        if let Some(expected_size) = repo_manifest.get_file_size(file_path) {
                            tracing::debug!(
                                "✅ Using manifest expected size for {}: {} bytes (scan found: {} bytes)", 
                                file_path, expected_size, file_data.total_bytes
                            );
                            expected_size
                        } else {
                            tracing::warn!(
                                "⚠️ File {} not found in manifest, using scan data: {} bytes", 
                                file_path, file_data.total_bytes
                            );
                            file_data.total_bytes
                        }
                    } else {
                        tracing::warn!(
                            "⚠️ No RepoManifest available, using scan data for {}: {} bytes", 
                            file_path, file_data.total_bytes
                        );
                        file_data.total_bytes
                    };

                    let mut file_metadata = OrdMap::new();
                    let absolute_path = model_data.model_cache_dir.join(&file_path);
                    file_metadata.insert("local_path".to_string(), absolute_path.to_string_lossy().to_string());
                    
                    let file_progress = ImmutableFileProgress {
                        file_id: file_metadata,
                        file_name: file_path.clone(),
                        bytes_downloaded: file_data.bytes_downloaded,
                        total_bytes: expected_file_size,
                        is_cached: file_data.from_cache,
                        chunks_downloaded: Vector::new(),
                    };

                    tracing::debug!(
                        "File from scan: {}, downloaded: {}, total: {}, cached: {}, complete: {}",
                        file_path,
                        file_data.bytes_downloaded,
                        file_data.total_bytes,
                        file_data.from_cache,
                        file_data.is_complete
                    );

                    files.push_back(file_progress);
                }

                // Use manifest total bytes if available and model IDs match, otherwise use scan data
                let model_total_bytes = if let Some(manifest) = &self.manifest_info {
                    if manifest.model_id == *model_id {
                        tracing::debug!(
                            "📋 Using manifest total_bytes for model {}: {} bytes (scan had: {} bytes)",
                            model_id,
                            manifest.total_bytes,
                            model_data.model_total_bytes
                        );
                        manifest.total_bytes
                    } else {
                        tracing::debug!(
                            "📋 Model ID mismatch: manifest={}, scan={}. Using scan total: {} bytes",
                            manifest.model_id,
                            model_id,
                            model_data.model_total_bytes
                        );
                        model_data.model_total_bytes
                    }
                } else {
                    tracing::debug!(
                        "📋 No manifest available. Using scan total for model {}: {} bytes",
                        model_id,
                        model_data.model_total_bytes
                    );
                    model_data.model_total_bytes
                };

                let model_progress = ImmutableModelProgress {
                    model_id: model_id.clone(),
                    bytes_downloaded: model_data.model_bytes_downloaded,
                    total_bytes: model_total_bytes,
                    files,
                    is_cached: model_data.is_model_cached,
                    metadata: OrdMap::new(),
                };

                models.push_back(model_progress);
            }

            // Use overall totals from filesystem scan, with manifest override if available
            let overall_total_bytes = if let Some(manifest) = &self.manifest_info {
                tracing::debug!(
                    "📋 Using manifest overall total_bytes: {} bytes (scan had: {} bytes)",
                    manifest.total_bytes,
                    scan_results.overall_total_bytes
                );
                manifest.total_bytes
            } else {
                tracing::debug!(
                    "📋 Using scan overall total_bytes: {} bytes",
                    scan_results.overall_total_bytes
                );
                scan_results.overall_total_bytes
            };

            tracing::debug!(
                "🎯 FINAL SCAN-BASED TOTALS: downloaded={}, total={}",
                scan_results.overall_bytes_downloaded,
                overall_total_bytes
            );

            ImmutableProgressState {
                models,
                overall_bytes_downloaded: scan_results.overall_bytes_downloaded,
                overall_total_bytes,
                manifest_data: OrdMap::new(),
                timestamp: SystemTime::now(),
            }
        } else {
            // Enhanced fallback when scan results are not available
            let reason = if self.consecutive_scan_failures >= self.max_scan_failures {
                "circuit breaker active due to persistent scan failures"
            } else if self.consecutive_scan_failures > 0 {
                "recent scan failures - waiting for retry"
            } else {
                "scan not yet performed"
            };
            
            warn!(
                "⚠️ FALLBACK PROGRESS: No filesystem scan results available - {} (failure count: {}/{})",
                reason, self.consecutive_scan_failures, self.max_scan_failures
            );

            // Attempt to provide useful progress data from file_states as emergency fallback
            if !self.file_states.is_empty() && self.consecutive_scan_failures >= self.max_scan_failures {
                warn!(
                    "🔄 EMERGENCY FALLBACK: Using file_states data due to persistent scan failures (may be less accurate)"
                );
                
                // Group file states by model for emergency fallback
                let mut models = Vector::new();
                let mut model_map: std::collections::HashMap<String, Vec<&FileProgressState>> = std::collections::HashMap::new();
                
                for file_state in self.file_states.values() {
                    model_map.entry(file_state.model_id.clone()).or_default().push(file_state);
                }
                
                let mut overall_downloaded = 0u64;
                let mut overall_total = 0u64;
                
                for (model_id, file_states) in model_map {
                    let mut files = Vector::new();
                    let mut model_downloaded = 0u64;
                    let mut model_total = 0u64;
                    
                    for file_state in &file_states {
                        let mut file_metadata = OrdMap::new();
                        let cache_dir = progresshub_config::environment::get_hf_hub_cache();
                        let absolute_path = cache_dir.join(&file_state.local_filepath);
                        file_metadata.insert("local_path".to_string(), absolute_path.to_string_lossy().to_string());
                        
                        let file_progress = ImmutableFileProgress {
                            file_id: file_metadata,
                            file_name: file_state.local_filepath.clone(),
                            bytes_downloaded: file_state.bytes_downloaded,
                            total_bytes: file_state.total_bytes,
                            is_cached: file_state.from_cache,
                            chunks_downloaded: Vector::new(),
                        };
                        
                        model_downloaded += file_state.bytes_downloaded;
                        model_total += file_state.total_bytes;
                        files.push_back(file_progress);
                    }
                    
                    let model_progress = ImmutableModelProgress {
                        model_id: model_id.clone(),
                        bytes_downloaded: model_downloaded,
                        total_bytes: model_total,
                        files,
                        is_cached: false, // Conservative assumption
                        metadata: OrdMap::new(),
                    };
                    
                    overall_downloaded += model_downloaded;
                    overall_total += model_total;
                    models.push_back(model_progress);
                }
                
                // Use manifest total if available
                if let Some(manifest) = &self.manifest_info {
                    overall_total = manifest.total_bytes;
                }
                
                warn!(
                    "🔄 EMERGENCY FALLBACK TOTALS: {} models, downloaded={}, total={}",
                    models.len(), overall_downloaded, overall_total
                );
                
                ImmutableProgressState {
                    models,
                    overall_bytes_downloaded: overall_downloaded,
                    overall_total_bytes: overall_total,
                    manifest_data: OrdMap::new(),
                    timestamp: SystemTime::now(),
                }
            } else {
                // Standard empty state fallback
                ImmutableProgressState {
                    models: Vector::new(),
                    overall_bytes_downloaded: 0,
                    overall_total_bytes: if let Some(manifest) = &self.manifest_info {
                        manifest.total_bytes
                    } else {
                        0
                    },
                    manifest_data: OrdMap::new(),
                    timestamp: SystemTime::now(),
                }
            }
        }
    }

    /// Calculate overall download speed across all files
    fn calculate_overall_speed(&self) -> f64 {
        let total_downloaded: u64 = self.file_states.values().map(|f| f.bytes_downloaded).sum();
        let earliest_start = self.get_earliest_start_time();
        let elapsed = earliest_start.elapsed().unwrap_or_default().as_secs_f64();

        if elapsed > 0.0 && total_downloaded > 0 {
            total_downloaded as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get earliest start time across all downloads
    fn get_earliest_start_time(&self) -> std::time::SystemTime {
        // Find the earliest start time across all file states
        let earliest_instant = self
            .file_states
            .values()
            .map(|state| state.start_time)
            .min()
            .unwrap_or_else(Instant::now);

        // Convert Instant to SystemTime for elapsed() calculation
        // Note: This approximation works for speed calculations within the same session
        std::time::SystemTime::now() - earliest_instant.elapsed()
    }

    /// Build model data for current state
    fn build_model_data(&self) -> ModelData {
        let models: std::collections::HashSet<String> = self
            .file_states
            .values()
            .map(|f| f.model_id.clone())
            .collect();

        let current_model = self
            .file_states
            .values()
            .next()
            .map(|f| f.model_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let current_filename = self
            .file_states
            .values()
            .next()
            .map(|f| f.local_filepath.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Count completed models by checking file completion status
        let completed_models = self
            .file_states
            .values()
            .map(|f| f.model_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .filter(|model_id| {
                // Model is completed if all its files are completed
                let model_files: Vec<_> = self
                    .file_states
                    .values()
                    .filter(|f| f.model_id == **model_id)
                    .collect();

                !model_files.is_empty() && model_files.iter().all(|f| f.is_file_finalized)
            })
            .count();

        ModelData::new(
            current_model,
            current_filename,
            models.len(),
            completed_models,
        )
    }

    /// Process finalization events from raw download events
    ///
    /// Handles the 4-level finalization system by validating each completion level
    /// against the filesystem and updating internal state accordingly.
    async fn process_finalization_event(
        &mut self,
        finalization: &FinalizationLevel,
        file_key: &str,
        raw_event: &RawDownloadEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match finalization {
            FinalizationLevel::RangeComplete {
                range_start,
                range_end,
            } => {
                if self
                    .filesystem_evaluator
                    .validate_range_completion(
                        &std::path::Path::new(&raw_event.local_filepath),
                        *range_start,
                        *range_end,
                    )
                    .await?
                {
                    if let Some(file_state) = self.file_states.get_mut(file_key) {
                        file_state.ranges_completed.push((*range_start, *range_end));
                    }
                    info!(
                        "✅ Range {}-{} completed for {}",
                        range_start, range_end, file_key
                    );
                }
            }
            FinalizationLevel::FileComplete { final_file_path } => {
                // Check if this file is from cache - cached files don't need .part finalization
                if let Some(file_state) = self.file_states.get(file_key) {
                    if file_state.from_cache {
                        // Cached files are already complete - just mark as finalized
                        if let Some(file_state) = self.file_states.get_mut(file_key) {
                            file_state.is_file_finalized = true;
                        }
                        info!("✅ Cached file marked as finalized: {}", final_file_path);

                        // Check for model completion
                        self.check_and_emit_model_completion(&raw_event.model_id)
                            .await?;
                        return Ok(());
                    }
                }

                // FileComplete events indicate the file download is complete
                // Use FilesystemProgressEvaluator to handle actual finalization
                let partial_path = std::path::Path::new(&raw_event.local_filepath);
                let final_path = std::path::Path::new(final_file_path);

                tracing::info!(
                    "🔍 FINALIZATION DEBUG: Processing FileComplete event: {:?} -> {:?}",
                    partial_path,
                    final_path
                );

                // Use FilesystemProgressEvaluator for atomic finalization
                if crate::filesystem_evaluator::FilesystemProgressEvaluator::validate_and_finalize_file_static(
                    &partial_path,
                    final_path,
                    raw_event.total_bytes,
                ).await? {
                    if let Some(file_state) = self.file_states.get_mut(file_key) {
                        file_state.is_file_finalized = true;
                    }
                    info!("✅ File completed and finalized: {}", final_file_path);

                    // Check for model completion
                    self.check_and_emit_model_completion(&raw_event.model_id)
                        .await?;
                } else {
                    // Update file state to reflect finalization failure
                    if let Some(file_state) = self.file_states.get_mut(file_key) {
                        file_state.is_file_finalized = false;
                    }

                    error!(
                        "❌ File finalization failed for {}: .part file missing or size mismatch",
                        final_file_path
                    );

                    // Return error to propagate failure up the call chain
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "File finalization failed for {}: .part file missing or size mismatch",
                            final_file_path
                        ),
                    )));
                }
            }
            FinalizationLevel::ModelComplete { model_id } => {
                info!("✅ Model {} marked complete", model_id);
                self.check_and_emit_all_models_completion().await?;
            }
            FinalizationLevel::AllModelsComplete => {
                info!("🎉 ALL MODELS COMPLETED");
            }
        }

        Ok(())
    }

    /// Check if model is complete and emit completion event if so
    async fn check_and_emit_model_completion(
        &mut self,
        model_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let model_files: Vec<_> = self
            .file_states
            .values()
            .filter(|f| f.model_id == model_id)
            .collect();

        if !model_files.is_empty() && model_files.iter().all(|f| f.is_file_finalized) {
            info!(
                "✅ Model {} completed - all {} files finalized",
                model_id,
                model_files.len()
            );

            // Direct completion logging - no circular event processing
            let total_bytes: u64 = model_files.iter().map(|f| f.total_bytes).sum();
            tracing::info!(
                "🎯 MODEL COMPLETION: {} ({} files, {} bytes)",
                model_id,
                model_files.len(),
                total_bytes
            );

            // Check if all models are now complete
            self.check_and_emit_all_models_completion().await?;
        }

        Ok(())
    }

    /// Check if all models are complete and emit completion event if so
    async fn check_and_emit_all_models_completion(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let all_models: std::collections::HashSet<String> = self
            .file_states
            .values()
            .map(|f| f.model_id.clone())
            .collect();

        let all_complete = all_models.iter().all(|model_id| {
            let model_files: Vec<_> = self
                .file_states
                .values()
                .filter(|f| f.model_id == *model_id)
                .collect();
            !model_files.is_empty() && model_files.iter().all(|f| f.is_file_finalized)
        });

        if all_complete && !all_models.is_empty() {
            let total_files = self.file_states.len();
            let total_bytes: u64 = self.file_states.values().map(|f| f.total_bytes).sum();

            tracing::info!(
                "🎉 ALL MODELS COMPLETED: {} models, {} files, {} bytes",
                all_models.len(),
                total_files,
                total_bytes
            );
        }

        Ok(())
    }
}

/// Validated progress result from filesystem evaluator
#[derive(Debug, Clone)]
pub struct ValidatedProgress {
    /// Validated bytes downloaded (monotonic)
    pub bytes_downloaded: u64,
    /// Validated total bytes
    pub total_bytes: u64,
    /// Whether file is from cache
    pub from_cache: bool,
}

/// Create and start central progress dispatcher
///  
/// Convenience function to create dispatcher and start event loop.
///
/// # Arguments
/// * `event_receiver` - Receiver for ProgressEvents (manifest and download events)
/// * `progress_sender` - Sender for ProgressCalculator snapshots to CLI/TUI
/// * `config` - Optional dispatcher configuration (uses default if None)
/// * `requested_models` - List of model IDs to scan (filters filesystem scanning to requested models only)
///
/// # Returns
/// Result indicating success or error
pub async fn start_central_dispatcher(
    event_receiver: Receiver<ProgressEvent>,
    progress_sender: Sender<ProgressCalculator>,
    config: Option<DispatcherConfig>,
    requested_models: Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config.unwrap_or_default();
    let mut dispatcher = CentralProgressDispatcher::new(config, progress_sender, requested_models);
    dispatcher.run_event_loop(event_receiver).await
}
