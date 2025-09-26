//! Example showing how to use ProgressHub with default CLI progress display.

use anyhow::Result;
use progresshub::ProgressHub;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Get model from args or use a default
    let model = env::args()
        .nth(1)
        .unwrap_or_else(|| "bert-base-uncased".to_string());

    println!("Starting download of model: {model}");

    // Use ProgressHub with default CLI display
    let results = ProgressHub::builder()
        .model(&model)
        .with_cli_progress() // This enables the beautiful CLI progress display
        .download()
        .await?;

    // Print results

    // Iterate through all results (OneOrMany can contain one or multiple results)
    for (i, result) in results.into_iter().enumerate() {
        println!("\nDownload {}:", i + 1);

        // Show model results using ZeroOneOrMany
        match &result.models {
            progresshub::ZeroOneOrMany::Zero => {
                println!("   No models downloaded");
            }
            progresshub::ZeroOneOrMany::One(model) => {
                println!(
                    "   Model: {} -> {}",
                    model.model_id,
                    model.model_cache_path.display()
                );
                println!(
                    "   Files: {}/{} downloaded",
                    model.files_downloaded, model.files_expected
                );
                println!("   Size reconciled: {}", model.size_reconciled);

                // Show individual files
                for file in &model.files {
                    println!("     📄 {} -> {}", file.filename, file.path.display());
                    println!(
                        "        Size: {}/{} bytes ({})",
                        file.downloaded_size,
                        file.expected_size,
                        if file.from_cache {
                            "cached"
                        } else {
                            "downloaded"
                        }
                    );
                }
            }
            progresshub::ZeroOneOrMany::Many(models) => {
                println!("   {} models downloaded:", models.len());
                for model in models {
                    println!(
                        "     📦 {} -> {}",
                        model.model_id,
                        model.model_cache_path.display()
                    );
                    println!(
                        "        Files: {}/{}, Size reconciled: {}",
                        model.files_downloaded, model.files_expected, model.size_reconciled
                    );
                }
            }
        }

        println!(
            "   Total downloaded: {} bytes",
            result.total_downloaded_bytes
        );
        println!("   Total expected: {} bytes", result.total_expected_bytes);
        println!("   Fully reconciled: {}", result.fully_reconciled);
        println!(
            "   Duration: {:.2} seconds",
            result.total_duration.as_secs_f64()
        );
        println!("   Average speed: {:.2} MB/s", result.average_speed_mbps);
    }

    Ok(())
}
