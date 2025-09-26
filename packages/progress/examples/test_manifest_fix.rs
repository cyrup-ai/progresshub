//! Test the manifest TLS fix by directly calling the manifest fetching function

use anyhow::Result;
use progresshub_progress::manifest::fetch_hf_manifest;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup basic logging to see the progress
    println!("Setting up logging...");

    println!("🧪 Testing manifest TLS fix...");

    // Test with a smaller model first
    let model = "microsoft/DialoGPT-small";

    println!("📥 Fetching manifest for model: {}", model);

    // Add a timeout to the fetch operation
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(8), fetch_hf_manifest(model)).await;

    match result {
        Ok(fetch_result) => match fetch_result {
            Ok(manifest) => {
                println!("✅ Manifest fetch successful!");
                println!("📊 Repository: {}", manifest.repo_id);
                println!("📁 Files found: {}", manifest.files.len());
                println!("📋 Total size: {} bytes", manifest.total_size);

                // Show first few files
                for (i, file) in manifest.files.iter().take(3).enumerate() {
                    println!("  {}. {} ({} bytes)", i + 1, file.path, file.size);
                }

                if manifest.files.len() > 3 {
                    println!("  ... and {} more files", manifest.files.len() - 3);
                }

                println!("🎉 SUCCESS: Manifest TLS fix is working!");
            }
            Err(e) => {
                println!("❌ Manifest fetch failed: {}", e);

                // Check if it's a TLS-related error
                let error_string = format!("{}", e);
                if error_string.contains("TLS")
                    || error_string.contains("SSL")
                    || error_string.contains("certificate")
                {
                    println!("🔴 TLS-related error detected - fix may not be working");
                    return Err(e);
                } else {
                    println!("🟡 Non-TLS error - TLS fix may be working but other issue present");
                    println!("🔍 Error details: {}", error_string);
                    return Err(e);
                }
            }
        },
        Err(_timeout) => {
            println!("⏰ Request timed out after 8 seconds");
            println!("🟡 This could mean the request is hanging - potential TLS or network issue");
        }
    }

    Ok(())
}
