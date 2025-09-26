//! Zero-allocation chunk assembly and validation
//!
//! Provides blazing-fast chunk validation using SHA256 hashing and lock-free
//! atomic file writing for parallel chunk assembly with elegant ergonomic APIs.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};
use tokio::sync::{Mutex, Semaphore};

use sha1::Sha1;
use sha2::{Digest, Sha256};
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
};


use crate::chunk_state::{ChunkError, ChunkResult};

/// Global semaphore to limit concurrent file I/O operations
/// This prevents resource exhaustion and "background task failed" errors
static FILE_IO_SEMAPHORE: std::sync::OnceLock<Arc<Semaphore>> = std::sync::OnceLock::new();

/// Get the global file I/O semaphore, creating it if it doesn't exist
fn get_file_io_semaphore() -> Arc<Semaphore> {
    FILE_IO_SEMAPHORE.get_or_init(|| {
        // Limit to 128 concurrent file operations to prevent resource exhaustion
        Arc::new(Semaphore::new(128))
    }).clone()
}

/// Validates chunk data integrity using SHA1 hashing (`HuggingFace` standard)
pub struct ChunkValidator {
    hasher: Sha1,
}

impl ChunkValidator {
    /// Create a new chunk validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: Sha1::new(),
        }
    }

    /// Add data to the hash calculation
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Finalize hash calculation and return hex string
    #[must_use]
    pub fn finalize(self) -> String {
        let result = self.hasher.finalize();
        hex::encode(result)
    }

    /// Validate a chunk against expected hash (if available)
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::ValidationFailed` if the calculated hash doesn't match the expected hash.
    pub fn validate_chunk(data: &[u8], expected_hash: Option<&str>) -> ChunkResult<String> {
        if let Some(expected) = expected_hash {
            let actual_hash = match expected.len() {
                40 => {
                    // SHA1 hash (40 hex chars) - Git blob OID format
                    // Git calculates blob OID as: sha1("blob <size>\0<content>")
                    let mut hasher = Sha1::new();
                    hasher.update(format!("blob {}\0", data.len()).as_bytes());
                    hasher.update(data);
                    hex::encode(hasher.finalize())
                }
                64 => {
                    // SHA256 hash (64 hex chars) - LFS content hash
                    let mut hasher = Sha256::new();
                    hasher.update(data);
                    hex::encode(hasher.finalize())
                }
                _ => {
                    return Err(ChunkError::ValidationFailed {
                        expected: expected.to_string(),
                        actual: format!("Unsupported hash length: {}", expected.len()),
                    });
                }
            };

            if actual_hash != expected {
                return Err(ChunkError::ValidationFailed {
                    expected: expected.to_string(),
                    actual: actual_hash,
                });
            }

            Ok(actual_hash)
        } else {
            // No expected hash, just compute SHA1 for compatibility
            let mut validator = Self::new();
            validator.update(data);
            Ok(validator.finalize())
        }
    }
}

impl Default for ChunkValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple offset-based file writer for testing and direct file manipulation
/// 
/// `ChunkWriter` provides a straightforward API for writing data to specific
/// offsets within a file, with pre-allocation and sync capabilities.
#[derive(Debug)]
pub struct ChunkWriter {
    file_path: PathBuf,
    file: Arc<Mutex<File>>,
    file_size: u64,
}

impl ChunkWriter {
    /// Create a new `ChunkWriter` with pre-allocated file size
    ///
    /// Pre-allocates the file to the specified size and enables random-access writes.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to create/write
    /// * `file_size` - Total size to pre-allocate for the file
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if file creation or pre-allocation fails.
    pub async fn with_size(file_path: &Path, file_size: u64) -> ChunkResult<Self> {
        // Acquire semaphore permit to limit concurrent file operations
        let semaphore = get_file_io_semaphore();
        let _permit = semaphore.acquire().await.map_err(|_| {
            ChunkError::WriteError("File I/O semaphore closed - system shutting down".to_string())
        })?;

        tracing::debug!(
            "ChunkWriter creating file {:?} with size {} bytes",
            file_path,
            file_size
        );

        // Ensure parent directory exists with retry logic
        if let Some(parent) = file_path.parent() {
            let mut retries: u32 = 0;
            loop {
                match tokio::fs::create_dir_all(parent).await {
                    Ok(()) => break,
                    Err(e) => {
                        retries += 1;
                        if retries >= 3 {
                            return Err(ChunkError::WriteError(format!(
                                "Failed to create parent directory for chunk file after {retries} retries: {e}"
                            )));
                        }
                        tracing::warn!(
                            "Directory creation attempt {} failed, retrying: {e}",
                            retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(10 * u64::from(retries))).await;
                    }
                }
            }
        }

        // Create file with retry logic
        let mut retries: u32 = 0;
        let file = loop {
            match File::create(&file_path).await {
                Ok(file) => break file,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to create chunk file {} after {} retries: {e}",
                            file_path.display(),
                            retries
                        )));
                    }
                    tracing::warn!(
                        "File creation attempt {} failed for {}, retrying: {e}",
                        retries,
                        file_path.display()
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(20 * u64::from(retries))).await;
                }
            }
        };

        // Pre-allocate file to specified size with retry logic
        if file_size > 0 {
            let mut retries: u32 = 0;
            loop {
                match file.set_len(file_size).await {
                    Ok(()) => break,
                    Err(e) => {
                        retries += 1;
                        if retries >= 3 {
                            return Err(ChunkError::WriteError(format!(
                                "Failed to pre-allocate file {} to {} bytes after {} retries: {e}",
                                file_path.display(),
                                file_size,
                                retries
                            )));
                        }
                        tracing::warn!(
                            "Pre-allocation attempt {} failed for {} bytes, retrying: {e}",
                            retries,
                            file_size
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(50 * u64::from(retries))).await;
                    }
                }
            }

            // Sync metadata to ensure pre-allocation is committed
            if let Err(e) = file.sync_all().await {
                return Err(ChunkError::WriteError(format!(
                    "Failed to sync file metadata after pre-allocation: {e}"
                )));
            }
        }

        tracing::info!(
            "Created ChunkWriter for {:?} with {} bytes pre-allocated",
            file_path,
            file_size
        );

        Ok(Self {
            file_path: file_path.to_path_buf(),
            file: Arc::new(Mutex::new(file)),
            file_size,
        })
    }

    /// Write chunk data at specific offset in the file
    ///
    /// Seeks to the specified offset and writes the data. Supports random-access writes
    /// to any valid offset within the pre-allocated file size.
    ///
    /// # Arguments
    /// * `offset` - Byte offset within the file to write at
    /// * `data` - Data to write at the specified offset
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if seeking or writing fails.
    pub async fn write_chunk(&self, offset: u64, data: &[u8]) -> ChunkResult<()> {
        // Acquire semaphore permit to limit concurrent write operations
        let semaphore = get_file_io_semaphore();
        let _permit = semaphore.acquire().await.map_err(|_| {
            ChunkError::WriteError("File I/O semaphore closed - system shutting down".to_string())
        })?;

        // Validate offset is within file bounds
        if offset + data.len() as u64 > self.file_size {
            return Err(ChunkError::WriteError(format!(
                "Write at offset {} with {} bytes would exceed file size {} bytes",
                offset,
                data.len(),
                self.file_size
            )));
        }

        let mut file = self.file.lock().await;

        // Seek to offset with retry logic
        let mut retries: u32 = 0;
        loop {
            match file.seek(std::io::SeekFrom::Start(offset)).await {
                Ok(_) => break,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to seek to offset {} in file {} after {} retries: {e}",
                            offset,
                            self.file_path.display(),
                            retries
                        )));
                    }
                    tracing::warn!(
                        "Seek attempt {} failed for offset {}, retrying: {e}",
                        retries,
                        offset
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * u64::from(retries))).await;
                }
            }
        }

        // Write data with retry logic
        let mut retries: u32 = 0;
        loop {
            match file.write_all(data).await {
                Ok(()) => break,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to write {} bytes at offset {} to file {} after {} retries: {e}",
                            data.len(),
                            offset,
                            self.file_path.display(),
                            retries
                        )));
                    }
                    tracing::warn!(
                        "Write attempt {} failed for {} bytes at offset {}, retrying: {e}",
                        retries,
                        data.len(),
                        offset
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(50 * u64::from(retries))).await;
                }
            }
        }

        tracing::debug!(
            "ChunkWriter wrote {} bytes at offset {} to {:?}",
            data.len(),
            offset,
            self.file_path
        );

        Ok(())
    }

    /// Sync all pending writes to disk
    ///
    /// Ensures all data and metadata changes are committed to storage.
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if syncing fails.
    pub async fn sync(&self) -> ChunkResult<()> {
        let file = self.file.lock().await;
        file.sync_all().await.map_err(|e| {
            ChunkError::WriteError(format!(
                "Failed to sync chunk file {}: {e}",
                self.file_path.display()
            ))
        })?;

        tracing::debug!("ChunkWriter synced file {:?}", self.file_path);
        Ok(())
    }

    /// Get the file path
    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Get the total file size
    #[must_use]
    pub fn file_size(&self) -> u64 {
        self.file_size
    }
}

/// Range-based file writer for blazing-fast parallel chunk writing without pre-allocation
#[derive(Debug)]
pub struct RangeFileWriter {
    range_file_path: PathBuf,
    base_file_path: PathBuf,
    model_id: String,
    model_hash: String,
    range_index: u32,
    range_start: u64,
    range_end: u64,
    bytes_written: Arc<AtomicU64>,
    file: Arc<Mutex<File>>,
}

impl RangeFileWriter {
    /// Create a new range file writer for a specific byte range
    ///
    /// Creates a range file named `{base_filename}.part.{model_hash}.{range_index}` that starts at 0 bytes
    /// and grows naturally as data is written. No pre-allocation is performed.
    ///
    /// # Arguments
    /// * `base_file_path` - The final file path (e.g., `/path/to/model.safetensors`)
    /// * `model_id` - Model identifier for hash generation (e.g., "microsoft/DialoGPT-small")
    /// * `range_index` - Zero-based index of this range (e.g., 0, 1, 2...)
    /// * `range_start` - Start byte offset for this range in the final file
    /// * `range_end` - End byte offset for this range in the final file
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if directory creation or file creation fails.
    pub async fn create_range_file(
        base_file_path: &Path,
        model_id: &str,
        range_index: u32,
        range_start: u64,
        range_end: u64,
    ) -> ChunkResult<Self> {
        // Acquire semaphore permit to limit concurrent file operations
        let semaphore = get_file_io_semaphore();
        let _permit = semaphore.acquire().await.map_err(|_| {
            ChunkError::WriteError("File I/O semaphore closed - system shutting down".to_string())
        })?;

        // Generate model hash for collision prevention
        let model_hash = Self::generate_model_hash(model_id);
        
        // Generate range file path: {base_filename}.part.{model_hash}.{range_index}
        let range_file_path = Self::generate_range_file_path_with_hash(base_file_path, &model_hash, range_index);
        
        tracing::debug!(
            "RangeFileWriter creating range file {} for model {} (hash: {}) bytes {}-{} at {:?}",
            range_index,
            model_id,
            model_hash,
            range_start,
            range_end,
            range_file_path
        );

        // Ensure parent directory exists with retry logic
        if let Some(parent) = range_file_path.parent() {
            // Retry directory creation up to 3 times to handle race conditions
            let mut retries: u32 = 0;
            loop {
                match tokio::fs::create_dir_all(parent).await {
                    Ok(()) => break,
                    Err(e) => {
                        retries += 1;
                        if retries >= 3 {
                            return Err(ChunkError::WriteError(format!(
                                "Failed to create parent directory for range file after {retries} retries: {e}"
                            )));
                        }
                        tracing::warn!(
                            "Directory creation attempt {} failed, retrying: {e}",
                            retries
                        );
                        // Small delay to reduce race condition probability
                        tokio::time::sleep(tokio::time::Duration::from_millis(10 * u64::from(retries))).await;
                    }
                }
            }
        }

        // Create range file starting at 0 bytes (no pre-allocation) with retry logic
        let mut retries: u32 = 0;
        let file = loop {
            match File::create(&range_file_path).await {
                Ok(file) => break file,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to create range file {} after {} retries: {e}",
                            range_file_path.display(),
                            retries
                        )));
                    }
                    tracing::warn!(
                        "File creation attempt {} failed for {}, retrying: {e}",
                        retries,
                        range_file_path.display()
                    );
                    // Small delay to reduce resource contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(20 * u64::from(retries))).await;
                }
            }
        };

        tracing::info!(
            "Created range file {} for model {} (hash: {}): {:?} (bytes {}-{}, size: 0)",
            range_index,
            model_id,
            model_hash,
            range_file_path,
            range_start,
            range_end
        );

        Ok(Self {
            range_file_path,
            base_file_path: base_file_path.to_path_buf(),
            model_id: model_id.to_string(),
            model_hash,
            range_index,
            range_start,
            range_end,
            bytes_written: Arc::new(AtomicU64::new(0)),
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Generate a short hash from model ID for collision prevention
    ///
    /// Uses the shared hash generation function from progresshub-common
    /// to ensure consistency between file creation and recognition.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier (e.g., "microsoft/DialoGPT-small")
    ///
    /// # Returns
    /// 8-character hex hash (e.g., "a1b2c3d4")
    #[must_use]
    pub fn generate_model_hash(model_id: &str) -> String {
        progresshub_common::generate_model_hash(model_id)
    }

    /// Generate the range file path with model hash for collision prevention
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (e.g., `/path/to/model.safetensors`)
    /// * `model_hash` - 8-character model hash for collision prevention
    /// * `range_index` - Range index (e.g., 0, 1, 2...)
    ///
    /// # Returns
    /// Range file path (e.g., `/path/to/model.safetensors.part.a1b2c3d4.0`)
    #[must_use]
    pub fn generate_range_file_path_with_hash(
        base_file_path: &Path,
        model_hash: &str,
        range_index: u32,
    ) -> PathBuf {
        let mut range_path = base_file_path.to_path_buf();
        let range_extension = format!("part.{model_hash}.{range_index}");
        
        if let Some(current_extension) = range_path.extension() {
            // File has extension: model.safetensors -> model.safetensors.part.a1b2c3d4.0
            range_path.set_extension(format!(
                "{}.{}",
                current_extension.to_string_lossy(),
                range_extension
            ));
        } else {
            // No extension: model -> model.part.a1b2c3d4.0
            range_path.set_extension(range_extension);
        }
        
        range_path
    }

    /// Generate the range file path for a given base file, model ID, and range index
    ///
    /// Convenience method that generates hash and path in one call.
    ///
    /// # Arguments
    /// * `base_file_path` - Base file path (e.g., `/path/to/model.safetensors`)
    /// * `model_id` - Model identifier for hash generation
    /// * `range_index` - Range index (e.g., 0, 1, 2...)
    ///
    /// # Returns
    /// Range file path (e.g., `/path/to/model.safetensors.part.a1b2c3d4.0`)
    #[must_use]
    pub fn generate_range_file_path(
        base_file_path: &Path,
        model_id: &str,
        range_index: u32,
    ) -> PathBuf {
        let model_hash = Self::generate_model_hash(model_id);
        Self::generate_range_file_path_with_hash(base_file_path, &model_hash, range_index)
    }

    /// Write chunk data to the range file
    ///
    /// Data is appended sequentially to the range file. The offset parameter is ignored
    /// since each range file contains only data for its specific byte range.
    ///
    /// # Arguments
    /// * `_offset` - Ignored (range files are written sequentially)
    /// * `data` - Data to write to the range file
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if file I/O operations fail.
    pub async fn write_chunk(&self, _offset: u64, data: &[u8]) -> ChunkResult<()> {
        // Acquire semaphore permit to limit concurrent write operations
        let semaphore = get_file_io_semaphore();
        let _permit = semaphore.acquire().await.map_err(|_| {
            ChunkError::WriteError("File I/O semaphore closed - system shutting down".to_string())
        })?;

        let mut file = self.file.lock().await;
        
        // Write data sequentially (append to end of range file) with retry logic
        let mut retries: u32 = 0;
        loop {
            match file.write_all(data).await {
                Ok(()) => break,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to write {} bytes to range file {} after {} retries: {e}",
                            data.len(),
                            self.range_file_path.display(),
                            retries
                        )));
                    }
                    tracing::warn!(
                        "Write attempt {} failed for {} bytes to {}, retrying: {e}",
                        retries,
                        data.len(),
                        self.range_file_path.display()
                    );
                    // Small delay to reduce resource contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(50 * u64::from(retries))).await;
                }
            }
        }

        // Sync data to ensure it's written to disk with retry logic
        let mut retries: u32 = 0;
        loop {
            match file.sync_data().await {
                Ok(()) => break,
                Err(e) => {
                    retries += 1;
                    if retries >= 3 {
                        return Err(ChunkError::WriteError(format!(
                            "Failed to sync range file data to disk after {retries} retries: {e}"
                        )));
                    }
                    tracing::warn!(
                        "Sync attempt {} failed for {}, retrying: {e}",
                        retries,
                        self.range_file_path.display()
                    );
                    // Small delay to reduce resource contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * u64::from(retries))).await;
                }
            }
        }

        // Update bytes written atomically
        self.bytes_written.fetch_add(data.len() as u64, Ordering::Relaxed);

        tracing::debug!(
            "Range {} (model: {}, hash: {}) wrote {} bytes (total: {} bytes)",
            self.range_index,
            self.model_id,
            self.model_hash,
            data.len(),
            self.bytes_written()
        );

        Ok(())
    }

    /// Get total bytes written to this range file
    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    /// Get the range file path
    #[must_use]
    pub fn range_file_path(&self) -> &Path {
        &self.range_file_path
    }

    /// Get the base file path
    #[must_use]
    pub fn base_file_path(&self) -> &Path {
        &self.base_file_path
    }

    /// Get the model ID
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Get the model hash
    #[must_use]
    pub fn model_hash(&self) -> &str {
        &self.model_hash
    }

    /// Get the range index
    #[must_use]
    pub fn range_index(&self) -> u32 {
        self.range_index
    }

    /// Get the range start byte offset
    #[must_use]
    pub fn range_start(&self) -> u64 {
        self.range_start
    }

    /// Get the range end byte offset
    #[must_use]
    pub fn range_end(&self) -> u64 {
        self.range_end
    }

    /// Get expected range size in bytes
    #[must_use]
    pub fn expected_range_size(&self) -> u64 {
        self.range_end - self.range_start
    }

    /// Check if this range is complete (all expected bytes written)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.bytes_written() >= self.expected_range_size()
    }

    /// Get completion percentage for this range (0.0 to 1.0)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn completion_percentage(&self) -> f64 {
        let written = self.bytes_written();
        let expected = self.expected_range_size();
        
        // For very large values (>2^53), use integer division to maintain precision
        if expected > (1_u64 << 53) || written > (1_u64 << 53) {
            if expected > 0 {
                ((written * 1000) / expected) as f64 / 1000.0
            } else {
                1.0
            }
        } else {
            let written_f64 = written as f64;
            let expected_f64 = expected as f64;
            if expected_f64 > 0.0 {
                (written_f64 / expected_f64).min(1.0)
            } else {
                1.0
            }
        }
    }

    /// Sync all pending writes to disk
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if syncing fails.
    pub async fn sync(&self) -> ChunkResult<()> {
        self.file.lock().await.sync_all().await.map_err(|e| {
            ChunkError::WriteError(format!(
                "Failed to sync range file {}: {e}",
                self.range_file_path.display()
            ))
        })?;
        Ok(())
    }

    /// Finalize the range file and prepare for combination
    ///
    /// This method ensures all data is synced and the range file is ready
    /// for combination with other ranges.
    ///
    /// # Errors
    ///
    /// Returns `ChunkError::WriteError` if finalization fails.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub async fn finalize(&self) -> ChunkResult<()> {
        self.sync().await?;
        
        tracing::info!(
            "Finalized range file {} with {} bytes ({}% complete)",
            self.range_index,
            self.bytes_written(),
            (self.completion_percentage() * 100.0).clamp(0.0, 100.0) as u32
        );
        
        Ok(())
    }
}
