//! Comprehensive unit tests for progress calculator
//!
//! Extracted from progress/src/calculator/tests.rs to follow best practices
//! for clean test organization and better discoverability with nextest.

use progresshub_progress::calculator::{
    converter::*, core::*, formatter::*, snapshot::*, types::*,
};

#[test]
fn test_core_calculations() {
    // Basic percentage calculations with decimal precision
    assert_eq!(calculate_percentage(0, 100).value, Some(0.0));
    assert_eq!(calculate_percentage(333, 1000).value, Some(33.3));

    // Edge cases
    let result = calculate_percentage(100, 0);
    assert!(result.is_indeterminate);

    let result = calculate_percentage(1500, 1000);
    assert_eq!(result.value, Some(100.0));
    assert!(result.is_complete);
}

#[test]
fn test_immutable_types() {
    let mut file_metadata = im::OrdMap::new();
    file_metadata.insert("name".to_string(), "test.bin".to_string());

    let file_progress = ImmutableFileProgress {
        file_id: file_metadata,
        file_name: "test.bin".to_string(),
        bytes_downloaded: 500,
        total_bytes: 1000,
        is_cached: false,
        chunks_downloaded: im::Vector::new(),
    };

    assert!(!file_progress.is_complete());
    assert_eq!(file_progress.file_name, "test.bin");
}

#[test]
fn test_formatting() {
    let progress = ProgressPercentage {
        value: Some(45.2),
        is_complete: false,
        is_indeterminate: false,
    };

    assert!(format_percentage(&progress).contains("45.2"));
    assert_eq!(get_percentage_rounded_str(450, 1000), "45%");
    assert_eq!(format_bytes(1024), "1.00 KB");
}

#[test]
fn test_immutable_state_operations() {
    let manifest_data = std::collections::HashMap::new();
    let model_data = vec![(
        "model1".to_string(),
        vec![("file1.bin".to_string(), 500u64, 1000u64, false)],
    )];

    let initial_state = from_manifest_and_progress(&manifest_data, &model_data);
    assert_eq!(initial_state.overall_bytes_downloaded, 500);

    // Test immutable update
    let mut file_metadata = im::OrdMap::new();
    file_metadata.insert("name".to_string(), "file1.bin".to_string());

    let file_update = ImmutableFileProgress {
        file_id: file_metadata,
        file_name: "file1.bin".to_string(),
        bytes_downloaded: 750,
        total_bytes: 1000,
        is_cached: false,
        chunks_downloaded: im::Vector::new(),
    };

    let updated_state = with_updated_file(initial_state.clone(), "model1", file_update);

    // Verify immutability - original unchanged, new state updated
    assert_eq!(initial_state.overall_bytes_downloaded, 500);
    assert_eq!(updated_state.overall_bytes_downloaded, 750);
}

#[test]
fn test_converter_utilities() {
    let model_data = vec![(500u64, 1000u64), (300u64, 600u64)];
    let (total_down, total_exp, percentage) = calculate_overall_from_models(model_data.into_iter());

    assert_eq!(total_down, 800);
    assert_eq!(total_exp, 1600);
    assert_eq!(percentage, 50.0);
}

#[test]
fn test_progress_completion() {
    assert!(is_progress_complete(1000, 1000));
    assert!(!is_progress_complete(500, 1000));
    assert!(!is_progress_complete(1000, 0)); // Invalid total

    assert_eq!(get_progress_ratio(500, 1000), 0.5);
    assert_eq!(get_progress_ratio(1000, 0), 0.0); // Zero division case
}
