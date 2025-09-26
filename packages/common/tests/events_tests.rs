use progresshub_common::events::{ManifestEvent, RawDownloadEvent};

#[test]
fn test_raw_download_event_creation() {
    let event = RawDownloadEvent::new(
        "microsoft/DialoGPT-small".to_string(),
        "Q4_K_M".to_string(),
        "https://huggingface.co/microsoft/DialoGPT-small/resolve/main/pytorch_model.bin"
            .to_string(),
        "pytorch_model.bin".to_string(),
        1024,
        2048,
        None,
        None,
    );

    assert_eq!(event.model_id, "microsoft/DialoGPT-small");
    assert_eq!(event.quant, "Q4_K_M");
    assert_eq!(event.local_filepath, "pytorch_model.bin");
    assert_eq!(event.bytes_downloaded, 1024);
    assert_eq!(event.total_bytes, 2048);
    assert_eq!(
        event.file_key(),
        "microsoft/DialoGPT-small:pytorch_model.bin"
    );
}

#[test]
fn test_manifest_event_creation() {
    let event = ManifestEvent::new(
        "microsoft/DialoGPT-medium".to_string(),
        12,
        1000000000,
        "Q4_K_M".to_string(),
    );

    assert_eq!(event.model_id, "microsoft/DialoGPT-medium");
    assert_eq!(event.total_files, 12);
    assert_eq!(event.total_bytes, 1000000000);
    assert_eq!(event.quant, "Q4_K_M");
}
