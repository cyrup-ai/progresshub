//!
//! This module implements the core download execution logic that orchestrates
//! concurrent downloads using the new DownloadOrchestrator architecture.
//! 
//! Replaced the old join_all() blocking pattern with orchestrator delegation
//! for true parallel downloads with priority scheduling.

use anyhow::Result;

use crate::{types::DownloadConfig, manifest::RepoManifest};
use progresshub_common::RawDownloadEvent;

/// Download execution result for a single file.
///
/// This struct provides comprehensive metadata about each download operation,
/// including performance metrics and validation results.
#[derive(Debug, Clone)]
pub struct FileDownloadResult {
    /// Local path where the file was downloaded
    pub local_path: std::path::PathBuf,
    /// Remote URL that was downloaded from
    pub remote_url: String,
    /// Total bytes successfully downloaded
    pub bytes_downloaded: u64,
    /// Total file size in bytes (from manifest)
    pub total_bytes: u64,
    /// Time taken for the download operation
    pub download_duration: std::time::Duration,
    /// Whether the download was resumed from a previous attempt
    pub was_resumed: bool,
    /// Checksum validation result
    pub checksum_valid: bool,
}



/// Execute downloads using the new orchestrator architecture with real completion tracking.
///
/// This function delegates to the DownloadOrchestrator for true parallel downloads
/// with priority-based scheduling and bounded concurrency. Uses completion channels
/// to track actual download results instead of fake success values.
///
/// # Arguments
/// * `repo_id` - Repository identifier for manifest context
/// * `manifest` - Repository manifest containing all files to download
/// * `destination` - Base destination directory for downloads
/// * `config` - Download configuration and preferences
/// * `raw_event_sender` - Channel for sending raw download events
///
/// # Returns
/// Vector of real download results for each file processed
///
/// # Performance
/// - Uses DownloadOrchestrator with priority-based job scheduling
/// - Large files downloaded first for optimal throughput
/// - Bounded concurrency with intelligent resource management
/// - No blocking join_all() patterns - true parallel execution
/// - Real completion tracking via channels
#[inline]
pub async fn execute_downloads(
    repo_id: &str,
    manifest: &RepoManifest,
    destination: &std::path::Path,
    config: &DownloadConfig,
    raw_event_sender: Option<flume::Sender<RawDownloadEvent>>,
) -> Result<Vec<FileDownloadResult>> {
    tracing::info!(
        "🚀 Starting real downloads for {} files from repo {} using DownloadOrchestrator with completion tracking",
        manifest.files.len(),
        repo_id
    );

    // Get or create the global orchestrator
    let orchestrator = crate::orchestration::download_orchestrator::get_global_orchestrator().await?;
    let orchestrator = orchestrator.lock().await;
    
    // Submit jobs with completion tracking to get real results
    let raw_sender = raw_event_sender.ok_or_else(|| {
        anyhow::anyhow!("Raw event sender is required for download execution")
    })?;
    
    let completion_receiver = orchestrator.submit_model_jobs_with_completion(
        repo_id,
        manifest,
        destination,
        config,
        raw_sender
    )?;
    
    // Drop orchestrator lock to allow workers to process jobs
    drop(orchestrator);

    // Count expected files (after quantization filtering)
    let mut expected_files = 0;
    for file_info in &manifest.files {
        if let Some(ref quant) = config.quantization {
            let is_gguf_model = file_info.path.ends_with(".gguf") || file_info.path.contains(".gguf");
            if is_gguf_model && !file_info.path.contains(quant) {
                continue;
            }
        }
        expected_files += 1;
    }

    tracing::info!("📋 Waiting for {} download completions...", expected_files);

    // Collect real completion results from workers
    let mut results = Vec::with_capacity(expected_files);
    for _ in 0..expected_files {
        match completion_receiver.recv_async().await {
            Ok(download_result) => {
                let file_result = FileDownloadResult {
                    local_path: download_result.job.destination,
                    remote_url: download_result.job.file_info.remote_url,
                    bytes_downloaded: download_result.bytes_downloaded,
                    total_bytes: download_result.job.file_info.size,
                    download_duration: download_result.duration,
                    was_resumed: download_result.was_resumed,
                    checksum_valid: download_result.checksum_valid, // Actual checksum validation result
                };
                results.push(file_result);
                
                if download_result.success {
                    tracing::debug!("✅ Completed: {} ({} bytes)", download_result.job.file_info.path, download_result.bytes_downloaded);
                } else {
                    tracing::warn!("❌ Failed: {} - {}", download_result.job.file_info.path, 
                        download_result.error_message.unwrap_or_else(|| "Unknown error".to_string()));
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to receive completion result: {}", e);
                return Err(anyhow::anyhow!("Download completion tracking failed: {}", e));
            }
        }
    }

    tracing::info!(
        "✅ Real download execution complete: {} files processed with actual results",
        results.len()
    );

    Ok(results)
}

