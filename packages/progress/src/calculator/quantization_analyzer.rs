//! Quantization pattern analysis and file intelligence.
//!
//! This module contains ALL quantization detection and analysis logic,
//! following the Pure Flume Channel Event-Driven Architecture where
//! the Progress crate is the ONLY location for intelligence.

use regex::Regex;
use std::collections::HashSet;

/// File information for quantization analysis.
#[derive(Debug, Clone)]
pub struct FileAnalysisInfo {
    /// File path or URL
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Optional XET hash (indicates XET availability)
    pub xet_hash: Option<String>,
}

/// Results of quantization analysis on a set of files.
#[derive(Debug, Clone)]
pub struct QuantizationAnalysis {
    /// All available quantizations found, sorted alphabetically
    pub available_quantizations: Vec<String>,
    /// Total number of quantized model files (excludes config files)
    pub total_quantized_files: usize,
    /// Count of files per quantization
    pub quantization_file_counts: Vec<(String, usize)>,
    /// Files with XET hashes available
    pub xet_available_files: usize,
    /// Files that require QUIC protocol
    pub quic_only_files: usize,
}

/// Quantization pattern analyzer - the centralized intelligence for file analysis.
pub struct QuantizationAnalyzer;

impl QuantizationAnalyzer {
    /// Create a new quantization analyzer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze a collection of files for available quantizations and patterns.
    ///
    /// This is the primary intelligence method that examines file paths/URLs
    /// to determine all available quantizations, XET availability, and file patterns.
    ///
    /// # Arguments
    /// * `files` - Collection of files to analyze
    ///
    /// # Returns
    /// Complete quantization analysis with all available information
    pub fn analyze_files(&self, files: &[FileAnalysisInfo]) -> QuantizationAnalysis {
        let mut quantizations = HashSet::new();
        let mut quantization_counts = std::collections::HashMap::new();
        let mut xet_available_files = 0;
        let mut quic_only_files = 0;
        let mut total_quantized_files = 0;

        for file in files {
            // Skip non-model files
            if self.is_non_model_file(&file.path) {
                continue;
            }

            // Count XET vs QUIC availability
            if file.xet_hash.is_some() {
                xet_available_files += 1;
            } else {
                quic_only_files += 1;
            }

            // Extract quantization if present
            if let Some(quantization) = self.extract_quantization(&file.path) {
                quantizations.insert(quantization.clone());
                *quantization_counts.entry(quantization).or_insert(0) += 1;
                total_quantized_files += 1;
            }
        }

        // Sort quantizations alphabetically
        let mut available_quantizations: Vec<String> = quantizations.into_iter().collect();
        available_quantizations.sort();

        // Create sorted file counts
        let mut quantization_file_counts: Vec<(String, usize)> =
            quantization_counts.into_iter().collect();
        quantization_file_counts.sort_by(|a, b| a.0.cmp(&b.0));

        QuantizationAnalysis {
            available_quantizations,
            total_quantized_files,
            quantization_file_counts,
            xet_available_files,
            quic_only_files,
        }
    }

    /// Validate that a requested quantization exists in the analyzed files.
    ///
    /// Returns detailed error messages when validation fails, including
    /// available alternatives and file counts for user guidance.
    ///
    /// # Arguments
    /// * `analysis` - Results from analyze_files()
    /// * `requested_quantization` - User's requested quantization
    ///
    /// # Returns
    /// * `Ok(())` if quantization exists
    /// * `Err(anyhow::Error)` with detailed error message if not found
    pub fn validate_quantization_request(
        &self,
        analysis: &QuantizationAnalysis,
        requested_quantization: &str,
    ) -> anyhow::Result<()> {
        // If no quantizations found, model doesn't support quantization filtering
        if analysis.available_quantizations.is_empty() {
            return Err(anyhow::anyhow!(
                "Model does not contain quantized files. Quantization filtering is not applicable for this model."
            ));
        }

        // Normalize requested quantization for comparison
        let normalized_requested = self.normalize_quantization(requested_quantization);

        // Check if requested quantization exists (case-insensitive)
        let quantization_exists = analysis
            .available_quantizations
            .iter()
            .any(|q| self.normalize_quantization(q) == normalized_requested);

        if quantization_exists {
            tracing::debug!(
                "Quantization '{}' validated successfully. Found in available quantizations: {:?}",
                requested_quantization,
                analysis.available_quantizations
            );
            Ok(())
        } else {
            // Generate detailed error message with file counts
            let quantization_info: Vec<String> = analysis
                .quantization_file_counts
                .iter()
                .map(|(quant, count)| {
                    format!(
                        "{} ({} file{})",
                        quant,
                        count,
                        if *count == 1 { "" } else { "s" }
                    )
                })
                .collect();

            let available_list = quantization_info.join(", ");

            Err(anyhow::anyhow!(
                "Quantization '{}' not found in {} quantized model file{}. Available quantizations: {}",
                requested_quantization,
                analysis.total_quantized_files,
                if analysis.total_quantized_files == 1 {
                    ""
                } else {
                    "s"
                },
                available_list
            ))
        }
    }

    /// Count how many files match a specific quantization.
    ///
    /// # Arguments
    /// * `files` - Files to analyze
    /// * `quantization` - Quantization to count matches for
    ///
    /// # Returns
    /// Number of model files matching the quantization
    pub fn count_quantized_files(&self, files: &[FileAnalysisInfo], quantization: &str) -> usize {
        files
            .iter()
            .filter(|file| {
                // Skip non-model files
                if self.is_non_model_file(&file.path) {
                    return false;
                }

                // Check if file matches the quantization
                if let Some(file_quant) = self.extract_quantization(&file.path) {
                    self.normalize_quantization(&file_quant)
                        == self.normalize_quantization(quantization)
                } else {
                    false
                }
            })
            .count()
    }

    /// Extract quantization pattern from a file path or URL.
    ///
    /// Analyzes file paths/URLs to identify quantization patterns in:
    /// - GGUF files (.gguf)
    /// - PyTorch model files (.bin)
    /// - SafeTensors files (.safetensors)
    ///
    /// # Arguments
    /// * `path` - File path or URL to analyze
    ///
    /// # Returns
    /// * `Some(quantization)` if quantization pattern found
    /// * `None` if no quantization detected
    pub fn extract_quantization(&self, path: &str) -> Option<String> {
        if path.ends_with(".gguf") {
            self.extract_quantization_from_gguf(path)
        } else if path.ends_with(".bin") || path.ends_with(".safetensors") {
            self.extract_quantization_from_model_file(path)
        } else {
            None
        }
    }

    /// Extract quantization from GGUF filename using regex patterns.
    fn extract_quantization_from_gguf(&self, filename: &str) -> Option<String> {
        let upper_filename = filename.to_uppercase();

        // Common GGUF quantization patterns
        let patterns = [
            // Standard patterns: model-Q4_K_M.gguf, model.Q4_K_M.gguf, model_Q4_K_M.gguf
            r"-([QF][0-9_A-Z]+)\.GGUF$",
            r"\.([QF][0-9_A-Z]+)\.GGUF$",
            r"_([QF][0-9_A-Z]+)\.GGUF$",
            // Embedded patterns: model-Q4_K_M-instruct.gguf
            r"-([QF][0-9_A-Z]+)-",
            r"\.([QF][0-9_A-Z]+)\.",
            r"_([QF][0-9_A-Z]+)_",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern)
                && let Some(captures) = regex.captures(&upper_filename)
                && let Some(quant_match) = captures.get(1)
            {
                let quantization = quant_match.as_str();
                return Some(self.normalize_quantization(quantization));
            }
        }
        None
    }

    /// Extract quantization from model files (.bin, .safetensors).
    fn extract_quantization_from_model_file(&self, filename: &str) -> Option<String> {
        let upper_filename = filename.to_uppercase();

        // Look for quantization indicators in filename
        let quantization_indicators = [
            "Q2_K", "Q3_K_S", "Q3_K_M", "Q3_K_L", "Q4_0", "Q4_1", "Q4_K_S", "Q4_K_M", "Q5_0",
            "Q5_1", "Q5_K_S", "Q5_K_M", "Q6_K", "Q8_0", "F16", "FP16", "F32", "FP32",
        ];

        for &indicator in &quantization_indicators {
            if upper_filename.contains(indicator) {
                return Some(self.normalize_quantization(indicator));
            }
        }
        None
    }

    /// Normalize quantization names for consistent comparison.
    ///
    /// Handles common variations like FP16 -> F16, case normalization, etc.
    fn normalize_quantization(&self, quantization: &str) -> String {
        match quantization.to_uppercase().as_str() {
            "FP16" => "F16".to_string(),
            "FP32" => "F32".to_string(),
            other => other.to_string(),
        }
    }

    /// Check if a file is a non-model file that should be excluded from quantization analysis.
    ///
    /// Non-model files (config.json, tokenizer files, README, etc.) are always downloaded
    /// regardless of quantization filtering and don't participate in quantization analysis.
    pub fn is_non_model_file(&self, filename: &str) -> bool {
        let non_model_extensions = [
            ".json", ".txt", ".md", ".py", ".yaml", ".yml", ".cfg", ".ini",
        ];
        let non_model_names = [
            "config.json",
            "tokenizer.json",
            "tokenizer_config.json",
            "vocab.json",
            "merges.txt",
            "special_tokens_map.json",
            "added_tokens.json",
            "generation_config.json",
            "README.md",
            "model_index.json",
            "scheduler_config.json",
            "tokenizer.model",
        ];

        // Check by extension
        if non_model_extensions
            .iter()
            .any(|ext| filename.ends_with(ext))
        {
            return true;
        }

        // Check by exact filename
        if non_model_names.contains(&filename) {
            return true;
        }

        false
    }
}

impl Default for QuantizationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
