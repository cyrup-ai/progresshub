use progresshub_client_http::{HttpClient, QuicClient};

#[tokio::test]
async fn test_custom_endpoint() {
    let client =
        HttpClient::new("github.com:443").expect("Failed to create client with valid endpoint");
    assert_eq!(client.connection_info().base_url, "github.com:443");
}

#[tokio::test]
async fn test_timeout_configuration() {
    let client = QuicClient::for_huggingface()
        .expect("Failed to create HuggingFace client")
        .with_timeout(120);
    assert_eq!(client.connection_info().timeout, 120);
}
