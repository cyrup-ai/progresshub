//! Example showing how to use ProgressHub with a beautiful custom progress display.

#![recursion_limit = "256"]

use anyhow::Result;
use colored::Colorize;
use display::FancyProgressDisplay;
use progresshub::{OneOrMany, ProgressHub};
use std::env;
use std::fmt::Write;

mod display;

/// Static constants for zero-allocation string handling
const DEFAULT_MODEL: &str = "bert-base-uncased";
const HEADER_LINE: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
const SEPARATOR_LINE: &str = "─────────────────────────────────────────────────";

#[tokio::main]
async fn main() -> Result<()> {
    // Get model from args or use a default - zero allocation with proper lifetime handling
    let model_arg = env::args().nth(1);
    let model = model_arg.as_deref().unwrap_or(DEFAULT_MODEL);

    // Print beautiful header with zero allocations
    println!(
        "\n{}",
        "🚀 ProgressHub - Rust Model Downloader".bold().cyan()
    );
    println!("{}", HEADER_LINE.bright_black());
    println!("📦 Model: {}", model.bold().blue());
    println!("{}\n", HEADER_LINE.bright_black());

    // Use new on_progress_event API - clean callback-based interface! 🎯
    let download_result = {
        // Create our fancy progress display with estimated file count
        let total_files = 16; // Conservative estimate for bert-base-uncased
        let mut display = FancyProgressDisplay::new(model, total_files)
            .map_err(|e| anyhow::anyhow!("Failed to create progress display: {}", e))?;

        let result = ProgressHub::builder()
            .model(model)
            .on_progress_event(move |progress, termcolor| {
                // Update display with each progress event using provided Termcolor
                if let Err(e) = display.update_progress(&progress, termcolor) {
                    let _ = termcolor.error(&format!("Display update failed: {}", e));
                }

                // Show completion when download is done
                if progress.is_complete() {
                    if let Err(e) = display.show_completion(termcolor) {
                        let _ = termcolor.error(&format!("Failed to show completion: {}", e));
                    }
                }
            })
            .download()
            .await
            .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?;

        result
    };

    // Extract result for summary - now we have OneOrMany from the new API
    let results = vec![match download_result {
        OneOrMany::One(result) => result,
        OneOrMany::Many(results) => {
            // For multiple results, take the first one for this simple example
            match results.into_iter().next() {
                Some(result) => result,
                None => return Err(anyhow::anyhow!("No results received from download")),
            }
        }
    }];

    // Print beautiful summary with pre-allocated buffer for zero allocations
    println!("\n{}", "📊 Download Summary".bold().cyan());
    println!("{}", SEPARATOR_LINE.bright_black());

    let mut summary_buffer = String::with_capacity(256);
    let mut size_buffer = String::with_capacity(32);

    // Convert to the expected format
    let results_wrapper = if results.len() == 1 {
        match results.into_iter().next() {
            Some(result) => OneOrMany::One(result),
            None => return Err(anyhow::anyhow!("Expected one result but found none")),
        }
    } else {
        OneOrMany::Many(results)
    };

    for (i, result) in results_wrapper.into_iter().enumerate() {
        // Zero-allocation summary formatting
        summary_buffer.clear();
        if write!(summary_buffer, "#{}", i + 1).is_err() {
            continue; // Skip this entry on write error
        }

        // Show detailed model information using ZeroOneOrMany
        match &result.models {
            progresshub::ZeroOneOrMany::Zero => {
                println!("{} No models downloaded", summary_buffer.bold().red());
            }
            progresshub::ZeroOneOrMany::One(model) => {
                // Zero-allocation size formatting
                size_buffer.clear();
                if write_bytes_human(&mut size_buffer, result.total_downloaded_bytes as f64)
                    .is_err()
                {
                    size_buffer.push_str("Unknown");
                }

                println!(
                    "{} {} ({})",
                    summary_buffer.bold().cyan(),
                    model.model_id.bold(),
                    size_buffer.dimmed()
                );

                println!(
                    "  ├─ {}: {} -> {}",
                    "Cache Path".dimmed(),
                    model.model_cache_path.display(),
                    if model.size_reconciled {
                        "✅ reconciled".green()
                    } else {
                        "⚠️  size mismatch".yellow()
                    }
                );

                println!(
                    "  ├─ {}: {}/{} files ({}/{} bytes)",
                    "Files".dimmed(),
                    model.files_downloaded,
                    model.files_expected,
                    model.total_downloaded_bytes,
                    model.total_expected_bytes
                );

                // Show first few files as examples
                let files_to_show = model.files.iter().take(3);
                for (idx, file) in files_to_show.enumerate() {
                    let prefix = if idx == model.files.len().min(3) - 1 && model.files.len() <= 3 {
                        "  └─ 📄"
                    } else {
                        "  ├─ 📄"
                    };
                    println!(
                        "{} {}: {} bytes {}",
                        prefix,
                        file.filename.dimmed(),
                        file.downloaded_size,
                        if file.from_cache {
                            "(cached)".dimmed()
                        } else {
                            "(downloaded)".dimmed()
                        }
                    );
                }

                if model.files.len() > 3 {
                    println!("  └─ ... and {} more files", model.files.len() - 3);
                }
            }
            progresshub::ZeroOneOrMany::Many(models) => {
                // Zero-allocation size formatting
                size_buffer.clear();
                if write_bytes_human(&mut size_buffer, result.total_downloaded_bytes as f64)
                    .is_err()
                {
                    size_buffer.push_str("Unknown");
                }

                println!(
                    "{} {} models ({})",
                    summary_buffer.bold().cyan(),
                    models.len(),
                    size_buffer.dimmed()
                );

                for (model_idx, model) in models.iter().enumerate() {
                    let prefix = if model_idx == models.len() - 1 {
                        "  └─"
                    } else {
                        "  ├─"
                    };
                    println!(
                        "{} 📦 {}: {}/{} files {}",
                        prefix,
                        model.model_id.bold(),
                        model.files_downloaded,
                        model.files_expected,
                        if model.size_reconciled {
                            "✅".green()
                        } else {
                            "⚠️".yellow()
                        }
                    );
                }
            }
        }

        println!(
            "  ├─ {}: {:.2} seconds",
            "Duration".dimmed(),
            result.total_duration.as_secs_f64()
        );
        println!(
            "  ├─ {}: {:.2} MB/s",
            "Avg Speed".dimmed(),
            result.average_speed_mbps
        );
        println!(
            "  └─ {}: {}\n",
            "Reconciled".dimmed(),
            if result.fully_reconciled {
                "✅ All sizes match".green()
            } else {
                "⚠️  Size mismatches".yellow()
            }
        );
    }

    println!("{}", HEADER_LINE.bright_black());

    Ok(())
}

/// Zero-allocation byte formatting using pre-allocated buffer
#[inline]
fn write_bytes_human(buffer: &mut String, bytes: f64) -> Result<(), std::fmt::Error> {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    const THRESHOLD: f64 = 1024.0;

    let mut size = bytes;
    let mut unit_index = 0;

    // Find appropriate unit without allocations
    while size >= THRESHOLD && unit_index < UNITS.len().saturating_sub(1) {
        size /= THRESHOLD;
        unit_index += 1;
    }

    // Write directly to buffer for zero allocations
    write!(buffer, "{:.2} {}", size, UNITS[unit_index])
}
