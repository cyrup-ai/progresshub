//! Test basic HTTP connectivity to HuggingFace to isolate the issue

use anyhow::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing basic HTTP connectivity to HuggingFace...");

    // Create a simple reqwest client without our custom configuration
    let simple_client = reqwest::Client::new();

    let url = "https://huggingface.co/api/models/microsoft/DialoGPT-small/tree/main";
    println!("📡 Testing URL: {}", url);

    // Test with basic client
    println!("🔄 Testing with basic reqwest client...");
    let result = tokio::time::timeout(Duration::from_secs(5), simple_client.get(url).send()).await;

    match result {
        Ok(Ok(response)) => {
            println!("✅ Basic client SUCCESS! Status: {}", response.status());
            let headers = response.headers();
            println!("📊 Headers received: {} headers", headers.len());
        }
        Ok(Err(e)) => {
            println!("❌ Basic client failed: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            println!("⏰ Basic client timed out");
        }
    }

    // Test with our configured client (same as fixed manifest.rs)
    println!("🔄 Testing with our fixed configuration...");
    let our_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let result2 = tokio::time::timeout(Duration::from_secs(5), our_client.get(url).send()).await;

    match result2 {
        Ok(Ok(response)) => {
            println!("✅ Our client SUCCESS! Status: {}", response.status());
        }
        Ok(Err(e)) => {
            println!("❌ Our client failed: {}", e);
            let error_str = format!("{}", e);
            if error_str.contains("TLS") || error_str.contains("SSL") {
                println!("🔴 TLS error still present in our configuration");
            }
            return Err(e.into());
        }
        Err(_) => {
            println!("⏰ Our client timed out");
        }
    }

    Ok(())
}
