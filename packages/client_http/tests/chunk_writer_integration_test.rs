//! Integration test for ChunkWriter to diagnose data loss bug

use progresshub_client_http::chunk_assembler::ChunkWriter;
use std::path::PathBuf;

#[tokio::test]
async fn test_chunk_writer_basic_functionality() {
    // Create a test file
    let test_file = PathBuf::from("/tmp/test_chunk_writer_integration.bin");

    // Remove file if it exists
    let _ = tokio::fs::remove_file(&test_file).await;

    println!("🧪 Testing ChunkWriter with 1MB file...");

    // Create ChunkWriter with 1MB size
    let file_size = 1024 * 1024; // 1MB
    let writer = ChunkWriter::with_size(&test_file, file_size).await.unwrap();

    println!("📝 Writing test chunks...");

    // Write some test data at different offsets
    let chunk1 = b"CHUNK1_DATA_START";
    let chunk2 = b"CHUNK2_DATA_MIDDLE";
    let chunk3 = b"CHUNK3_DATA_END";

    writer.write_chunk(0, chunk1).await.unwrap(); // Start of file
    writer.write_chunk(512 * 1024, chunk2).await.unwrap(); // Middle 
    writer
        .write_chunk(file_size - chunk3.len() as u64, chunk3)
        .await
        .unwrap(); // End

    println!("🔄 Syncing file...");
    writer.sync().await.unwrap();

    println!("📖 Reading file back...");
    let file_data = tokio::fs::read(&test_file).await.unwrap();

    println!(
        "📊 File size: {} bytes (expected: {})",
        file_data.len(),
        file_size
    );

    // Check if chunks were written correctly
    let chunk1_read = &file_data[0..chunk1.len()];
    let chunk2_start = 512 * 1024;
    let chunk2_read = &file_data[chunk2_start..chunk2_start + chunk2.len()];
    let chunk3_start = (file_size - chunk3.len() as u64) as usize;
    let chunk3_read = &file_data[chunk3_start..chunk3_start + chunk3.len()];

    println!(
        "✅ Chunk 1: {:?} (expected: {:?})",
        String::from_utf8_lossy(chunk1_read),
        String::from_utf8_lossy(chunk1)
    );

    println!(
        "✅ Chunk 2: {:?} (expected: {:?})",
        String::from_utf8_lossy(chunk2_read),
        String::from_utf8_lossy(chunk2)
    );

    println!(
        "✅ Chunk 3: {:?} (expected: {:?})",
        String::from_utf8_lossy(chunk3_read),
        String::from_utf8_lossy(chunk3)
    );

    // Check if chunks match
    if chunk1_read == chunk1 && chunk2_read == chunk2 && chunk3_read == chunk3 {
        println!("🎉 SUCCESS: All chunks written correctly!");
    } else {
        println!("❌ FAILURE: Chunks don't match!");

        if chunk1_read != chunk1 {
            println!("  Chunk 1 mismatch");
        }
        if chunk2_read != chunk2 {
            println!("  Chunk 2 mismatch");
        }
        if chunk3_read != chunk3 {
            println!("  Chunk 3 mismatch");
        }

        panic!("ChunkWriter test failed!");
    }

    // Check for zeros in between chunks
    let zeros_between_1_and_2 = &file_data[chunk1.len()..chunk2_start];
    let zero_count = zeros_between_1_and_2.iter().filter(|&&b| b == 0).count();
    println!(
        "🔍 Zeros between chunk 1 and 2: {}/{} bytes",
        zero_count,
        zeros_between_1_and_2.len()
    );

    // Clean up
    let _ = tokio::fs::remove_file(&test_file).await;

    assert!(chunk1_read == chunk1 && chunk2_read == chunk2 && chunk3_read == chunk3);
}
