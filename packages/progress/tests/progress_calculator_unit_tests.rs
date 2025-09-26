//! Unit tests for ProgressCalculator formatting and calculation methods
//!
//! Tests extracted from production src/ file to maintain proper separation
//! between production code and test code, following architectural guidelines.

use std::time::SystemTime;

use progresshub_progress::calculator::{
    ImmutableProgressState, ModelData, ProgressCalculator, TimingData,
};

fn create_test_progress_calculator() -> ProgressCalculator {
    let progress_state = ImmutableProgressState {
        models: im::Vector::new(),
        overall_bytes_downloaded: 1024,
        overall_total_bytes: 2048,
        manifest_data: im::OrdMap::new(),
        timestamp: SystemTime::now(),
    };

    let timing_data = TimingData::new_download();
    let model_data = ModelData::new(
        "microsoft/DialoGPT-small".to_string(),
        "pytorch_model.bin".to_string(),
        1,
        0,
    );

    ProgressCalculator::new(
        progress_state,
        "Q4_K_M".to_string(),
        timing_data,
        model_data,
    )
}

#[test]
fn test_percentage_formatted() {
    let calc = create_test_progress_calculator();
    let percentage = calc.percentage_formatted();
    // The formatter adds a leading space to align the percentage
    assert_eq!(percentage, " 50.0%");
}

#[test]
fn test_bytes_formatted() {
    let calc = create_test_progress_calculator();
    let bytes = calc.bytes_formatted();
    // This will depend on the format_bytes implementation
    assert!(bytes.contains("/"));
    assert!(bytes.contains("1024") || bytes.contains("1.0"));
}

#[test]
fn test_quantization_info_formatted() {
    let calc = create_test_progress_calculator();
    let quant_info = calc.quantization_info_formatted();
    assert_eq!(quant_info, "Quantization: Q4_K_M");
}

#[test]
fn test_progress_key() {
    let calc = create_test_progress_calculator();
    let key = calc.progress_key();
    assert_eq!(key, "microsoft/DialoGPT-small:pytorch_model.bin");
}

#[test]
fn test_is_complete() {
    let mut calc = create_test_progress_calculator();
    assert!(!calc.is_complete());

    // Set to complete
    calc.progress_state.overall_bytes_downloaded = 2048;
    assert!(calc.is_complete());
}
