//! Internal high-level builder interface for Pure Flume Channel Event-Driven Architecture
//!
//! This module provides the internal builder that handles:
//! 1. Setting up flume channels for raw events and progress calculator events
//! 2. Starting the central dispatcher in background tasks
//! 3. Starting download orchestration in background tasks
//! 4. Returning flume Receiver<ProgressCalculator> for CLI/TUI consumption
//!
//! This is the INTERNAL interface - packages/progresshub provides the public API.

use crate::ProgressCalculator;
use crate::central_dispatcher::{DispatcherConfig, start_central_dispatcher};
use progresshub_config::environment::get_hf_hub_cache;
use std::path::PathBuf;

/// Get model cache directory path  
fn get_model_cache_dir(model_id: &str) -> anyhow::Result<PathBuf> {
    let cache_root = get_hf_hub_cache();
    let model_cache_dir = cache_root.join(format!("models--{}", model_id.replace('/', "--")));
    Ok(model_cache_dir)
}
use flume::{Receiver, Sender};
use progresshub_common::{ProgressEvent, RawDownloadEvent, SimpleRepoManifest, SimpleFileInfo};
use tokio::task::JoinHandle;

/// Type alias for orchestration function that executes downloads
/// Takes (models, quantization, force, raw_sender) and returns JoinHandle for background task
pub type OrchestrationFunction = Box<
    dyn FnOnce(Vec<String>, Option<String>, bool, Sender<RawDownloadEvent>) -> JoinHandle<()>
        + Send,
>;

/// Internal progress builder for event-driven architecture
pub struct InternalProgressBuilder {
    models: Vec<String>,
    quantization: Option<String>,
    destination: Option<PathBuf>,
    force: bool,
    verbose: bool,
    orchestration: Option<OrchestrationFunction>,
}

impl Default for InternalProgressBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalProgressBuilder {
    /// Create new internal builder
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            quantization: None,
            destination: None,
            force: false,
            verbose: false,
            orchestration: None,
        }
    }

    /// Add model to download
    pub fn model<S: AsRef<str>>(mut self, model: S) -> Self {
        self.models.push(model.as_ref().to_string());
        self
    }

    /// Add multiple models to download
    pub fn models<S: AsRef<str>>(mut self, models: Vec<S>) -> Self {
        for model in models {
            self.models.push(model.as_ref().to_string());
        }
        self
    }

    /// Set quantization filter
    pub fn quantization<S: AsRef<str>>(mut self, quant: S) -> Self {
        self.quantization = Some(quant.as_ref().to_string());
        self
    }

    /// Set destination directory
    pub fn destination<P: Into<PathBuf>>(mut self, dest: P) -> Self {
        self.destination = Some(dest.into());
        self
    }

    /// Force redownload by clearing cache
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Enable verbose logging output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set orchestration function for dependency injection
    pub fn with_orchestration<F>(mut self, orchestration: F) -> Self
    where
        F: FnOnce(Vec<String>, Option<String>, bool, Sender<RawDownloadEvent>) -> JoinHandle<()>
            + Send
            + 'static,
    {
        self.orchestration = Some(Box::new(orchestration));
        self
    }

    /// Get current models list
    ///
    /// Returns the currently configured models list
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Reference to current models vector
    pub fn get_models(&self) -> &Vec<String> {
        &self.models
    }

    /// Get current quantization setting
    ///
    /// Returns the currently configured quantization setting
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Reference to current quantization option
    pub fn get_quantization(&self) -> &Option<String> {
        &self.quantization
    }

    /// Get current destination setting
    ///
    /// Returns the currently configured destination directory
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Reference to current destination option
    pub fn get_destination(&self) -> &Option<PathBuf> {
        &self.destination
    }

    /// Get current force setting
    ///
    /// Returns the currently configured force setting
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Current force setting value
    pub fn get_force(&self) -> bool {
        self.force
    }

    /// Get current verbose setting
    ///
    /// Returns the currently configured verbose setting
    /// for inspection and testing purposes.
    ///
    /// # Returns
    /// Current verbose setting value
    pub fn get_verbose(&self) -> bool {
        self.verbose
    }

    /// Get ProgressCalculator receiver
    ///
    /// This method:
    /// 1. Creates flume channels for progress events and progress calculator events
    /// 2. Starts central dispatcher in background task
    /// 3. Starts download orchestration in background tasks (with force handling)
    /// 4. Returns receiver for CLI/TUI to consume ProgressCalculator events
    ///
    /// # Returns
    /// `Receiver<ProgressCalculator>` for consuming progress events
    pub fn receiver(self) -> Receiver<ProgressCalculator> {
        // Force deletion will be handled asynchronously in orchestration task
        // to prevent blocking the runtime

        // Create flume channels
        let (event_sender, event_receiver) = flume::unbounded::<ProgressEvent>();
        let (progress_sender, progress_receiver) = flume::unbounded::<ProgressCalculator>();

        // Start central dispatcher in background with requested models
        let dispatcher_config = DispatcherConfig::default();
        let requested_models = self.models.clone(); // Clone models before moving into tokio::spawn
        tokio::spawn(async move {
            if let Err(e) =
                start_central_dispatcher(event_receiver, progress_sender, Some(dispatcher_config), requested_models)
                    .await
            {
                tracing::error!("Central dispatcher failed: {}", e);
            }
        });

        // Create wrapper sender that converts RawDownloadEvent to ProgressEvent::Download
        let raw_sender = {
            let event_sender_clone = event_sender.clone();
            let (raw_sender, raw_receiver) = flume::unbounded::<RawDownloadEvent>();

            // Spawn robust converter task with backpressure handling and retry logic
            tokio::spawn(async move {
                tracing::info!(
                    "🔄 CONVERTER: Started resilient RawDownloadEvent → ProgressEvent::Download converter task"
                );

                // Circuit breaker state for handling persistent failures
                let mut consecutive_failures = 0u32;
                const MAX_CONSECUTIVE_FAILURES: u32 = 10;
                const BASE_RETRY_DELAY_MS: u64 = 10;

                while let Ok(raw_event) = raw_receiver.recv_async().await {
                    tracing::debug!(
                        "🟡 CONVERTER: Received RawDownloadEvent: model={}, file={}, bytes={}/{}",
                        raw_event.model_id,
                        raw_event.local_filepath,
                        raw_event.bytes_downloaded,
                        raw_event.total_bytes
                    );

                    let progress_event = ProgressEvent::Download(raw_event.clone());

                    // Retry logic with exponential backoff for temporary failures
                    let mut retry_count = 0u32;
                    const MAX_RETRIES: u32 = 5;

                    loop {
                        match event_sender_clone.send_async(progress_event.clone()).await {
                            Ok(()) => {
                                tracing::debug!(
                                    "✅ CONVERTER: Successfully converted and forwarded ProgressEvent::Download for {}",
                                    raw_event.local_filepath
                                );
                                consecutive_failures = 0; // Reset circuit breaker on success
                                break; // Success - move to next event
                            }
                            Err(e) => {
                                retry_count += 1;
                                consecutive_failures += 1;

                                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                                    tracing::error!(
                                        "🚨 CONVERTER: Circuit breaker triggered - {} consecutive failures",
                                        consecutive_failures
                                    );
                                    tracing::error!(
                                        "🚨 CONVERTER: Progress pipeline appears broken - shutting down converter"
                                    );
                                    return; // Exit converter task due to persistent failures
                                }

                                if retry_count >= MAX_RETRIES {
                                    tracing::error!(
                                        "❌ CONVERTER: Max retries exceeded for ProgressEvent::Download: {}",
                                        e
                                    );
                                    tracing::error!(
                                        "❌ CONVERTER: Dropping event for {} after {} retries",
                                        raw_event.local_filepath,
                                        MAX_RETRIES
                                    );
                                    break; // Drop this event and continue with next
                                }

                                // Exponential backoff with jitter
                                let delay_ms = BASE_RETRY_DELAY_MS * (2_u64.pow(retry_count - 1));
                                let jitter_ms =
                                    (delay_ms as f64 * 0.1 * fastrand::f64()).round() as u64;
                                let total_delay =
                                    std::time::Duration::from_millis(delay_ms + jitter_ms);

                                tracing::warn!(
                                    "⚠️ CONVERTER: Retry {}/{} for ProgressEvent failed: {} (retrying in {:?})",
                                    retry_count,
                                    MAX_RETRIES,
                                    e,
                                    total_delay
                                );

                                tokio::time::sleep(total_delay).await;
                            }
                        }
                    }
                }
                tracing::warn!(
                    "🛑 CONVERTER: RawDownloadEvent → ProgressEvent::Download converter task ended"
                );
            });

            raw_sender
        };

        // Start download orchestration in background using injected function or existing orchestration system
        if let Some(orchestration) = self.orchestration {
            orchestration(self.models, self.quantization, self.force, raw_sender);
        } else {
            let models = self.models;
            let quantization = self.quantization;
            let force = self.force;
            let event_sender_for_manifest = event_sender.clone();

            tokio::spawn(async move {
                use crate::DownloadConfig;
                // Orchestrator handles all download coordination - no need for download_tasks
                use progresshub_common::ManifestEvent;

                tracing::info!(
                    "🚀 ORCHESTRATION TASK: Starting PARALLEL orchestration for {} models",
                    models.len()
                );

                // Handle force deletion asynchronously BEFORE starting any downloads
                if force {
                    tracing::info!("🗑️ FORCE MODE: Asynchronously deleting model directories BEFORE starting downloads");
                    
                    for model in &models {
                        match get_model_cache_dir(model) {
                            Ok(model_cache_dir) => {
                                if model_cache_dir.exists() {
                                    match tokio::fs::remove_dir_all(&model_cache_dir).await {
                                        Ok(()) => {
                                            tracing::info!(
                                                "🗑️ FORCE: Successfully deleted model directory: {:?}",
                                                model_cache_dir
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "❌ FORCE: Failed to delete model directory {:?}: {}",
                                                model_cache_dir,
                                                e
                                            );
                                        }
                                    }
                                } else {
                                    tracing::debug!(
                                        "🗑️ FORCE: Model directory does not exist: {:?}",
                                        model_cache_dir
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ FORCE: Failed to get cache directory for model {}: {}",
                                    model,
                                    e
                                );
                            }
                        }
                    }
                    
                    tracing::info!("✅ FORCE MODE: All force deletions completed - now starting downloads");
                }
                
                let config = DownloadConfig {
                    quantization: quantization.clone(),
                };

                // Get global DownloadOrchestrator singleton (bounded concurrency)
                tracing::info!("🚀 Using global DownloadOrchestrator - all models will download in parallel with priority scheduling");

                // Submit all model jobs to orchestrator (no blocking, no per-model tasks)
                for model in models {
                    let raw_sender_clone = raw_sender.clone();
                    let event_sender_clone = event_sender_for_manifest.clone();
                    let quantization_clone = quantization.clone();

                    // Fetch manifest and submit all jobs to orchestrator immediately (no blocking)
                    let manifest = match crate::fetch_hf_manifest(&model).await {
                        Ok(manifest) => {
                            tracing::info!(
                                "📋 Manifest fetched for {}: {} files, {} total bytes",
                                model,
                                manifest.files.len(),
                                manifest.total_size
                            );

                            let quant = quantization_clone
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string());

                            // Send legacy ManifestEvent for compatibility
                            let manifest_event = ManifestEvent::new(
                                model.clone(),
                                manifest.files.len(),
                                manifest.total_size,
                                quant.clone(),
                            );

                            if let Err(e) = event_sender_clone
                                .send_async(ProgressEvent::Manifest(manifest_event))
                                .await
                            {
                                tracing::error!(
                                    "Failed to send manifest event for {}: {}",
                                    model,
                                    e
                                );
                                continue; // Continue with other models instead of returning error
                            }

                            // Convert RepoManifest to SimpleRepoManifest for accurate progress calculation
                            let simple_files: Vec<SimpleFileInfo> = manifest
                                .files
                                .iter()
                                .map(|file| SimpleFileInfo {
                                    path: file.path.clone(),
                                    size: file.size,
                                    hash: file.hash.clone(),
                                    remote_url: file.remote_url.clone(),
                                })
                                .collect();

                            let simple_repo_manifest = SimpleRepoManifest::new(
                                model.clone(),
                                simple_files,
                                manifest.total_size,
                                quant,
                            );

                            // Send detailed SimpleRepoManifest for accurate per-file progress calculation
                            if let Err(e) = event_sender_clone
                                .send_async(ProgressEvent::RepoManifest(simple_repo_manifest))
                                .await
                            {
                                tracing::error!(
                                    "Failed to send repo manifest event for {}: {}",
                                    model,
                                    e
                                );
                                continue; // Continue with other models instead of returning error
                            }

                            tracing::info!(
                                "✅ ManifestEvent and RepoManifest sent for {} - ready for orchestrator job submission",
                                model
                            );

                            // Brief delay for ManifestEvent processing
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                            manifest
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to fetch manifest for {}: {}", model, e);
                            continue; // Continue with other models instead of stopping all
                        }
                    };

                    // Get cache directory for downloads
                    let cache_dir = match get_model_cache_dir(&model) {
                        Ok(dir) => dir,
                        Err(e) => {
                            tracing::error!("❌ Failed to get cache directory for {}: {}", model, e);
                            continue; // Continue with other models
                        }
                    };

                    // Submit model jobs to global orchestrator (NO BLOCKING - immediate job submission)
                    let orchestrator = match crate::orchestration::download_orchestrator::get_global_orchestrator().await {
                        Ok(orchestrator) => orchestrator,
                        Err(e) => {
                            tracing::error!("❌ Failed to get global orchestrator for model {}: {}", model, e);
                            continue; // Continue with other models
                        }
                    };
                    
                    {
                        let orchestrator = orchestrator.lock().await;
                        if let Err(e) = orchestrator.submit_model_jobs(
                            &model,
                            &manifest,
                            &cache_dir,
                            &config,
                            raw_sender_clone,
                        ) {
                            tracing::error!("❌ Failed to submit jobs for model {}: {}", model, e);
                            continue; // Continue with other models
                        }
                    }

                    tracing::info!(
                        "✅ All {} files from model {} submitted to orchestrator with priority scheduling",
                        manifest.files.len(),
                        model
                    );
                }

                tracing::info!(
                    "🚀 All models submitted to orchestrator - downloads running in parallel with priority scheduling"
                );
                
                // All jobs submitted to orchestrator - downloads run in background with bounded concurrency
                // Progress events flow immediately to UI via RawDownloadEvent channel
                // No blocking, no waiting - orchestrator manages all parallelism and resource limits
            });
        }

        progress_receiver
    }
}

/// Create internal progress builder
pub fn internal_builder() -> InternalProgressBuilder {
    InternalProgressBuilder::new()
}
