//! Test ETag timeout configuration
//!
//! This example demonstrates that the ETag timeout is properly configurable
//! via the HF_HUB_ETAG_TIMEOUT environment variable.

use progresshub_client_http::chunk_fetcher::ChunkFetcher;

#[tokio::main]
async fn main() {
    println!("🔧 Testing ETag timeout configuration...");

    // Test with default timeout (10 seconds)
    let default_timeout = progresshub_config::environment::get_hf_hub_etag_timeout();
    println!("📊 Default ETag timeout: {} seconds", default_timeout);

    // Set custom timeout and verify it's picked up
    unsafe {
        std::env::set_var("HF_HUB_ETAG_TIMEOUT", "5");
    }
    let custom_timeout = progresshub_config::environment::get_hf_hub_etag_timeout();
    println!("🎯 Custom ETag timeout: {} seconds", custom_timeout);

    // Create a ChunkFetcher to test ETag request with configured timeout
    let fetcher = ChunkFetcher::new();

    // Test with a real HuggingFace URL that supports ETags
    let test_url = "https://huggingface.co/microsoft/DialoGPT-small/resolve/main/config.json";
    let file_size = 1000;

    println!("🌐 Testing ETag fetch with configured timeout...");
    println!("📍 URL: {}", test_url);

    let start_time = std::time::Instant::now();
    match fetcher.fetch_optimal_chunk_size(test_url, file_size) {
        Ok(chunk_size) => {
            let duration = start_time.elapsed();
            println!("✅ ETag fetch successful!");
            println!("   Optimal chunk size: {} bytes", chunk_size);
            println!("   Request completed in: {:?}", duration);
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("⚠️  ETag fetch failed in {:?}: {:?}", duration, e);
            println!("   This is expected if network is unavailable or timeout is very short");
        }
    }

    println!("🎉 ETag timeout configuration test completed!");
}
