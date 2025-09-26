use progresshub_progress::internal_builder;

#[tokio::test]
async fn test_internal_builder_creation() {
    let builder = internal_builder();
    assert!(builder.get_models().is_empty());
    assert!(builder.get_quantization().is_none());
    assert!(builder.get_destination().is_none());
    assert!(!builder.get_force());
    assert!(!builder.get_verbose());
}

#[tokio::test]
async fn test_builder_fluent_api() {
    let builder = internal_builder()
        .model("test-model")
        .quantization("Q4_K_M")
        .force(true);

    assert_eq!(builder.get_models(), &vec!["test-model"]);
    assert_eq!(builder.get_quantization(), &Some("Q4_K_M".to_string()));
    assert!(builder.get_force());
}
