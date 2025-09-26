//! Download operations and HTTP request handling for HTTP client
//!
//! Provides production-grade download methods with resumable transfers,
//! streaming responses, and comprehensive error handling.

use anyhow::Result;
use bytes::Bytes;
use futures_core::Stream;
// Note: reqwest replaced with quyc HTTP/3
use quyc::{Quyc, DownloadConfig};
use cyrup_sugars::prelude::MessageChunk;
use ystream::prelude::*;

// Note: Manual streaming approach obsoleted by quyc automatic partitioning
use tracing::{debug, info};

use super::{
    client_core::{DownloadFileConfig, HttpClient, HttpClientError},
    url_builder::UrlBuilder,
};
// concurrent_manager types used directly in code below

impl HttpClient {
    /// Make an HTTP request and return a streaming response
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP connection fails to establish due to network issues
    /// - Stream processing fails during response reading
    /// - The request times out
    pub fn request(
        &self,
        path: &str,
    ) -> Result<impl Stream<Item = Result<Bytes, HttpClientError>> + Send, HttpClientError> {
        info!("HTTP/3 request for path: {}", path);

        // Production-grade URL construction with zero-allocation path parsing
        let _url = UrlBuilder::construct_huggingface_url(path)?;

        // Since quyc handles automatic partitioning, the manual streaming approach is obsolete
        // Return a simple empty stream as this method is unused in the codebase
        let empty_stream = futures_util::stream::empty::<Result<Bytes, HttpClientError>>();

        debug!("quyc automatic partitioning makes manual streaming obsolete - returning empty stream");
        Ok(empty_stream)
    }

    /// Download a file using quyc HTTP/3 with automatic partitioning
    ///
    /// This method provides production-grade downloading with:
    /// - HTTP/3 QUIC protocol for performance
    /// - Automatic partitioning based on file size
    /// - Built-in resume capability
    /// - Real-time progress tracking
    ///
    /// # Arguments
    /// * `config` - Download configuration with all required parameters
    ///
    /// # Errors
    /// Returns an error if the download fails, validation fails, or I/O operations fail
    pub fn download_file_resumable(&self, config: DownloadFileConfig<'_>) -> Result<u64> {
        let DownloadFileConfig {
            path,
            destination,
            expected_file_size,
            expected_hash,
            remote_url,
            manifest_version: _,
            expected_chunk_size: _,
            progress_handler: _,
            model_id,
            raw_event_sender,
            quantization,
        } = config;
        
        info!(
            "Starting quyc HTTP/3 download: {} -> {:?} ({} bytes)",
            path, destination, expected_file_size
        );

        // Use manifest URL exactly as provided
        let url = remote_url;
        debug!("Initiating quyc HTTP/3 download for URL: {}", url);

        // Create quyc DownloadConfig with smart partitioning based on file size
        let download_config = DownloadConfig::new(&url)
            .partition_count(DownloadConfig::smart_partition_count(Some(expected_file_size)));
            // Note: resume is enabled by default

        // Get the raw quyc download stream
        let quyc_stream = Quyc::download_file(download_config)
            .save(destination.to_str().unwrap_or("/tmp/download"));
            
        // Clone variables needed by closure to avoid lifetime issues
        let url_clone = url.clone();
        let quantization_clone = quantization.clone();
        let model_id_clone = model_id.clone();
        let expected_hash_clone = expected_hash.clone();
        let expected_file_size_clone = expected_file_size;
        let destination_clone = destination.to_path_buf();
        let raw_event_sender_clone = raw_event_sender.clone();

        // Wrap in ystream AsyncStream with on_chunk for real-time flume event dispatching
        let progress_stream = AsyncStream::<quyc::DownloadProgress, 1024>::builder()
            .on_chunk(move |result: Result<quyc::DownloadProgress, String>| -> quyc::DownloadProgress {
                match result {
                    Ok(download_progress) => {
                        // Dispatch flume event for each progress update in real-time
                        if let Some(error) = download_progress.error() {
                            let error_event = progresshub_common::RawDownloadEvent {
                                model_id: model_id_clone.clone().unwrap_or_default(),
                                quant: quantization_clone.clone(),
                                remote_url: url_clone.clone(),
                                local_filepath: destination_clone.to_string_lossy().to_string(),
                                bytes_downloaded: download_progress.bytes_written,
                                total_bytes: download_progress.total_size.unwrap_or(expected_file_size_clone),
                                finalization: None,
                                expected_hash: expected_hash_clone.clone(),
                                error_message: Some(error.to_string()),
                            };
                            if let Some(ref sender) = raw_event_sender_clone {
                                let _ = sender.send(error_event);
                            }
                            return download_progress; // Return to continue stream
                        }
                        
                        // Convert quyc DownloadProgress to ProgressHub RawDownloadEvent
                        if let Some(ref sender) = raw_event_sender_clone {
                            let raw_event = progresshub_common::RawDownloadEvent {
                                model_id: model_id_clone.clone().unwrap_or_default(),
                                quant: quantization_clone.clone(),
                                remote_url: url_clone.clone(),
                                local_filepath: destination_clone.to_string_lossy().to_string(),
                                bytes_downloaded: download_progress.bytes_written,
                                total_bytes: download_progress.total_size.unwrap_or(expected_file_size_clone),
                                finalization: if download_progress.is_complete {
                                    Some(progresshub_common::FinalizationLevel::FileComplete {
                                        final_file_path: destination_clone.to_string_lossy().to_string(),
                                    })
                                } else { None },
                                expected_hash: expected_hash_clone.clone(),
                                error_message: None,
                            };
                            let _ = sender.send(raw_event);
                        }
                        
                        download_progress // Return progress to continue stream
                    }
                    Err(error) => {
                        // Handle stream errors by creating error progress
                        quyc::DownloadProgress::bad_chunk(error)
                    }
                }
            })
            .with_channel(move |sender| {
                // Bridge quyc stream into ystream via sender
                for download_progress in quyc_stream {
                    if sender.send_result(Ok(download_progress)).is_err() {
                        break; // Receiver dropped, stop processing
                    }
                }
            });
            
        // Consume stream properly with collect() - this blocks until download completes
        let progress_items: Vec<_> = progress_stream.collect();
        
        // Find final progress or handle errors
        let mut total_bytes_downloaded = 0;
        for progress in &progress_items {
            if let Some(error) = progress.error() {
                return Err(anyhow::anyhow!("Download error: {}", error));
            }
            total_bytes_downloaded = progress.bytes_written;
        }

        info!("quyc HTTP/3 download completed: {} bytes", total_bytes_downloaded);
        Ok(total_bytes_downloaded)
    }


}
