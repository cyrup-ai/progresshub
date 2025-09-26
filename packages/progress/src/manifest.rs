//! HuggingFace manifest fetching and repository analysis.
//!
//! This module handles fetching repository manifests from the HuggingFace Hub API
//! and processing them for download orchestration.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Manifest describing a repository's contents.
#[derive(Debug, Clone)]
pub struct RepoManifest {
    /// Repository ID.
    pub repo_id: String,
    /// List of files in the repository.
    pub files: Vec<FileInfo>,
    /// Total size of all files.
    pub total_size: u64,
}

/// Information about a single file in the repository.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Path relative to repository root.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Optional hash/checksum (git oid).
    pub hash: Option<String>,
    /// Remote URL from manifest (used exactly as provided)
    pub remote_url: String,
    /// Version tracking information for invalidation detection
    pub version_info: ManifestVersionInfo,
}

/// Manifest version information for download state invalidation
#[derive(Debug, Clone)]
pub struct ManifestVersionInfo {
    /// File hash from HuggingFace API (blob_id) for version tracking
    pub file_hash: Option<String>,
    /// Exact file size from manifest for validation
    pub file_size: u64,
    /// ETag or last-modified timestamp if available
    pub last_modified: Option<String>,
    /// Recommended chunk size based on file size analysis
    pub chunk_size_hint: u64,
    /// Repository revision or commit hash if available
    pub repo_revision: Option<String>,
}

impl ManifestVersionInfo {
    /// Create new version info with file size and hash
    pub fn new(file_size: u64, file_hash: Option<String>) -> Self {
        let chunk_size_hint = Self::calculate_optimal_chunk_size(file_size);

        Self {
            file_hash,
            file_size,
            last_modified: None,
            chunk_size_hint,
            repo_revision: None,
        }
    }

    /// Calculate optimal chunk size based on file size
    pub fn calculate_optimal_chunk_size(file_size: u64) -> u64 {
        const SMALL_FILE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
        const MEDIUM_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB

        const SMALL_CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
        const MEDIUM_CHUNK_SIZE: u64 = 4 * 1024 * 1024; // 4MB  
        const LARGE_CHUNK_SIZE: u64 = 8 * 1024 * 1024; // 8MB

        if file_size <= SMALL_FILE_THRESHOLD {
            SMALL_CHUNK_SIZE
        } else if file_size <= MEDIUM_FILE_THRESHOLD {
            MEDIUM_CHUNK_SIZE
        } else {
            LARGE_CHUNK_SIZE
        }
    }

    /// Generate a version string for state file validation
    pub fn version_string(&self) -> String {
        let mut components = Vec::new();

        components.push(format!("size:{}", self.file_size));

        if let Some(ref hash) = self.file_hash {
            components.push(format!("hash:{hash}"));
        }

        if let Some(ref rev) = self.repo_revision {
            components.push(format!("rev:{rev}"));
        }

        if let Some(ref modified) = self.last_modified {
            components.push(format!("modified:{modified}"));
        }

        components.join("|")
    }
}

/// HuggingFace API response structures for tree endpoint.
type HfTreeResponse = Vec<HfFileInfo>;

#[derive(Debug, Deserialize)]
struct HfFileInfo {
    #[serde(rename = "type")]
    file_type: String,
    oid: String,
    size: u64,
    path: String,
    lfs: Option<LfsInfo>,
}

#[derive(Debug, Deserialize)]
struct LfsInfo {
    oid: String,
}

/// Create HTTP client with HF_TOKEN authentication if available.
pub fn create_hf_client() -> Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(token) = progresshub_config::environment::get_hf_token() {
        tracing::info!("Using HF_TOKEN for authentication");
        let auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .context("Invalid HF_TOKEN format")?;
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
    } else {
        tracing::warn!("No HF_TOKEN found, making unauthenticated request");
    }

    let timeout = progresshub_config::environment::get_hf_hub_download_timeout();
    tracing::info!("Creating reqwest client with timeout: {}s", timeout);

    // Use EXACT same configuration as working client_http
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(10)) // Fix TLS panic - matches client_http
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    Ok(client)
}

/// Fetch the manifest for a repository from HuggingFace with enhanced version tracking.
pub async fn fetch_hf_manifest(repo_id: &str) -> Result<RepoManifest> {
    let client = create_hf_client()?;
    let url = format!("https://huggingface.co/api/models/{repo_id}/tree/main");

    tracing::info!("Requesting URL: {}", url);

    println!("📥 Fetching model manifest from HuggingFace...");

    let response = match client.get(&url).send().await {
        Ok(resp) => {
            tracing::info!("Request successful, status: {}", resp.status());
            println!("✅ Manifest received, parsing model files...");
            resp
        }
        Err(e) => {
            tracing::error!("Request failed with error: {:?}", e);
            return Err(anyhow::anyhow!(
                "Failed to send request to HuggingFace API at URL: {} - Error: {}",
                url,
                e
            ));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error response body".to_string());
        return Err(anyhow::anyhow!(
            "Failed to fetch repository info: {} - Response body: {}",
            status,
            error_body
        ));
    }

    // Extract version metadata from response headers
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"').to_string());

    let last_modified_header = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let response_text = response
        .text()
        .await
        .context("Failed to get response text from HuggingFace API")?;

    tracing::debug!(
        "HF API raw response (first 1000 chars): {}",
        &response_text[..std::cmp::min(1000, response_text.len())]
    );

    let tree_response: HfTreeResponse = serde_json::from_str(&response_text)
        .context("Failed to parse tree response from HuggingFace API")?;

    // Filter only files (not directories)
    let file_entries: Vec<_> = tree_response
        .into_iter()
        .filter(|entry| entry.file_type == "file")
        .collect();

    let mut files = Vec::with_capacity(file_entries.len());
    let mut total_size = 0u64;

    let repo_revision = etag.clone();
    let repo_last_modified = last_modified_header;

    for hf_file in file_entries {
        let size = hf_file.size;
        tracing::debug!(
            "HF API file: {} - size from API: {} -> used size: {}",
            hf_file.path,
            hf_file.size,
            size
        );
        total_size = total_size.saturating_add(size);

        let mut version_info = ManifestVersionInfo::new(size, Some(hf_file.oid.clone()));
        version_info.last_modified = repo_last_modified.clone().or_else(|| etag.clone());
        version_info.repo_revision = repo_revision.clone();
        version_info.chunk_size_hint = calculate_optimized_chunk_size(size, &hf_file.path);

        // Always use standard resolve URLs - client_http handles all downloads now
        let remote_url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, hf_file.path
        );

        // Only use hash for LFS files (which have proper content hashes)
        // Skip validation for small files (Git blob OIDs aren't content hashes)
        let file_hash = if let Some(ref lfs_info) = hf_file.lfs {
            tracing::info!(
                "🔍 LFS file detected: {} - hash: {}",
                hf_file.path,
                lfs_info.oid
            );
            Some(lfs_info.oid.clone())
        } else {
            tracing::info!(
                "📄 Non-LFS file: {} - skipping hash validation",
                hf_file.path
            );
            None
        };

        files.push(FileInfo {
            path: hf_file.path.clone(),
            size,
            hash: file_hash,
            remote_url,
            version_info,
        });
    }

    tracing::info!(
        "Repository analysis for {}: {} files total",
        repo_id,
        files.len()
    );

    Ok(RepoManifest {
        repo_id: repo_id.to_string(),
        files,
        total_size,
    })
}

/// Calculate optimized chunk size based on file size and type.
fn calculate_optimized_chunk_size(file_size: u64, filename: &str) -> u64 {
    const SMALL_FILE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
    const MEDIUM_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB
    const LARGE_FILE_THRESHOLD: u64 = 1024 * 1024 * 1024; // 1GB

    const SMALL_CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
    const MEDIUM_CHUNK_SIZE: u64 = 4 * 1024 * 1024; // 4MB  
    const LARGE_CHUNK_SIZE: u64 = 8 * 1024 * 1024; // 8MB
    const XLARGE_CHUNK_SIZE: u64 = 16 * 1024 * 1024; // 16MB

    let is_model_file = filename.ends_with(".bin")
        || filename.ends_with(".safetensors")
        || filename.ends_with(".gguf");

    let is_config_file =
        filename.ends_with(".json") || filename.ends_with(".txt") || filename.ends_with(".md");

    match (file_size, is_model_file, is_config_file) {
        (_, _, true) => SMALL_CHUNK_SIZE,
        (size, true, _) if size > LARGE_FILE_THRESHOLD => XLARGE_CHUNK_SIZE,
        (size, true, _) if size > MEDIUM_FILE_THRESHOLD => LARGE_CHUNK_SIZE,
        (size, true, _) if size > SMALL_FILE_THRESHOLD => MEDIUM_CHUNK_SIZE,
        (size, _, _) if size > LARGE_FILE_THRESHOLD => LARGE_CHUNK_SIZE,
        (size, _, _) if size > MEDIUM_FILE_THRESHOLD => MEDIUM_CHUNK_SIZE,
        (size, _, _) if size > SMALL_FILE_THRESHOLD => SMALL_CHUNK_SIZE,
        _ => SMALL_CHUNK_SIZE,
    }
}
