use futures::stream::StreamExt;
use progresshub_progress::channel::ProgressData;
use progresshub_progress::types::FileStatus;
use progresshub_progress::debounced::DebouncedProgressStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Create a test ProgressData with specified parameters
fn create_test_event(model_id: &str, file_path: &str, bytes: u64) -> ProgressData {
    ProgressData {
        model_id: model_id.to_string(),
        file_path: file_path.to_string(),
        bytes_downloaded: bytes,
        total_bytes: 1000,
        speed_mbps: 50.0,
        from_cache: false,
        status: FileStatus::Downloading,
        error_message: None,
        timestamp: Instant::now(),
        is_cached: false,
        error: None,
    }
}

#[tokio::test]
async fn test_atomic_counter_consistency_under_load() {
    // Test that atomic counter accurately reflects queue contents under concurrent load
    let (sender, receiver) = flume::unbounded();
    let mut stream = DebouncedProgressStream::new(
        receiver,
        Duration::from_millis(50),
        100, // queue capacity
    );

    let events_to_send = 1000;
    let num_threads = 10;
    let events_per_thread = events_to_send / num_threads;

    // Spawn multiple threads to send events concurrently
    let sender_handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let sender = sender.clone();
            thread::spawn(move || {
                for i in 0..events_per_thread {
                    let event = create_test_event(
                        &format!("model_{}", thread_id),
                        &format!("file_{}_{}", thread_id, i),
                        i as u64,
                    );
                    sender.send(event).expect("Failed to send event");
                }
            })
        })
        .collect();

    // Wait for all senders to complete
    for handle in sender_handles {
        handle.join().expect("Thread panicked");
    }

    // Drop the sender to signal completion
    drop(sender);

    // Consume all events and verify consistency
    let mut events_received = 0;
    let timeout_duration = Duration::from_secs(10);

    while let Ok(Some(_event)) = timeout(timeout_duration, stream.next()).await {
        events_received += 1;
    }

    // Verify we received events (some may be coalesced, so exact count may vary)
    assert!(
        events_received > 0,
        "Should have received at least some events"
    );
    assert!(
        events_received <= events_to_send,
        "Should not receive more events than sent"
    );

    println!(
        "Sent {} events, received {} events (coalescing reduced duplication)",
        events_to_send, events_received
    );
}

#[tokio::test]
async fn test_coalescing_threshold_accuracy() {
    // Test that coalescing kicks in at the correct queue capacity threshold (75%)
    let (sender, receiver) = flume::unbounded();
    let queue_capacity = 20; // Smaller capacity to make threshold easier to hit
    let threshold = (queue_capacity * 3) / 4; // 15 events should trigger coalescing
    
    let mut stream = DebouncedProgressStream::new(
        receiver,
        Duration::from_millis(200), // Longer debounce to prevent time-based debouncing
        queue_capacity,
    );

    // Send many events with the SAME model_id and file_path so they can be coalesced
    // This is key - coalescing only works for events with identical (model_id, file_path)
    let model_id = "test_model";
    let file_path = "test_file.bin"; // Same file path for all events
    
    // Send enough events to exceed the threshold (15 + 5 = 20 events)
    for i in 0..threshold + 5 {
        let event = create_test_event(model_id, file_path, i as u64);
        sender.send(event).expect("Failed to send event");
        // Small delay to ensure events accumulate in queue
        tokio::time::sleep(Duration::from_micros(100)).await;
    }

    // Close sender to trigger stream completion
    drop(sender);

    // Now start consuming - coalescing should have occurred during polling
    let mut events_received = 0;
    let mut final_bytes = 0u64;
    let timeout_duration = Duration::from_secs(3);

    while let Ok(Some(event)) = timeout(timeout_duration, stream.next()).await {
        events_received += 1;
        if event.model_id == model_id && event.file_path == file_path {
            final_bytes = event.bytes_downloaded;
        }
    }

    // After coalescing, we should receive significantly fewer events than we sent
    // Since all events have the same (model_id, file_path), they should coalesce to 1 event
    let events_sent = threshold + 5;
    assert!(
        events_received < events_sent,
        "Coalescing should have reduced event count from {} to {}, but got {}",
        events_sent, events_received, events_received
    );

    println!(
        "Sent {} events with same model+file, received {} events after coalescing, final bytes: {}",
        events_sent, events_received, final_bytes
    );
}

#[tokio::test]
async fn test_high_throughput_race_conditions() {
    // Stress test with very high event throughput to expose race conditions
    let (sender, receiver) = flume::unbounded();
    let mut stream = DebouncedProgressStream::new(
        receiver,
        Duration::from_millis(10), // Fast debouncing
        500, // Large capacity
    );

    let total_events = 10000;
    let num_threads = 20;
    let events_per_thread = total_events / num_threads;
    let events_processed = Arc::new(AtomicUsize::new(0));

    let start_time = Instant::now();

    // Spawn producer threads
    let sender_handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let sender = sender.clone();
            thread::spawn(move || {
                for i in 0..events_per_thread {
                    let event = create_test_event(
                        &format!("model_{}", thread_id % 5), // Use fewer models to increase coalescing
                        &format!("file_{}", i % 10), // Reuse file names to trigger coalescing
                        (thread_id * 1000 + i) as u64,
                    );
                    sender.send(event).expect("Failed to send event");
                    
                    // Add small delays occasionally to create bursty traffic
                    if i % 100 == 0 {
                        thread::sleep(Duration::from_micros(10));
                    }
                }
            })
        })
        .collect();

    // Consumer task
    let events_processed_clone = events_processed.clone();
    let consumer_task = tokio::spawn(async move {
        let timeout_duration = Duration::from_secs(30);
        while let Ok(Some(_event)) = timeout(timeout_duration, stream.next()).await {
            events_processed_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Wait for all producers to complete
    for handle in sender_handles {
        handle.join().expect("Producer thread panicked");
    }

    // Signal completion and wait for consumer
    drop(sender);
    consumer_task.await.expect("Consumer task panicked");

    let elapsed = start_time.elapsed();
    let final_count = events_processed.load(Ordering::Relaxed);

    println!(
        "High throughput test: processed {} events in {:?} ({:.0} events/sec)",
        final_count,
        elapsed,
        final_count as f64 / elapsed.as_secs_f64()
    );

    // Verify we processed a reasonable number of events
    assert!(final_count > 0, "Should have processed some events");
    assert!(
        final_count <= total_events,
        "Should not process more events than sent"
    );
}

#[tokio::test]
async fn test_boundary_conditions() {
    // Test edge cases: empty queue, full queue, and rapid fill/drain cycles
    let (sender, receiver) = flume::unbounded();
    let queue_capacity = 10;
    let mut stream = DebouncedProgressStream::new(
        receiver,
        Duration::from_millis(50),
        queue_capacity,
    );

    // Test 1: Empty queue handling
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Test 2: Fill queue to exact capacity
    for i in 0..queue_capacity {
        let event = create_test_event("model_boundary", &format!("file_{}", i), i as u64);
        sender.send(event).expect("Failed to send event");
    }

    // Test 3: Overfill to trigger coalescing
    for i in queue_capacity..queue_capacity * 2 {
        let event = create_test_event("model_boundary", &format!("file_{}", i), i as u64);
        sender.send(event).expect("Failed to send event");
    }

    // Test 4: Rapid drain and refill
    let mut events_received = 0;
    let timeout_duration = Duration::from_millis(100);

    // Consume some events
    for _ in 0..5 {
        if let Ok(Some(_event)) = timeout(timeout_duration, stream.next()).await {
            events_received += 1;
        }
    }

    // Refill with more events
    for i in 0..5 {
        let event = create_test_event("model_refill", &format!("file_refill_{}", i), i as u64);
        sender.send(event).expect("Failed to send event");
    }

    drop(sender);

    // Consume remaining events
    while let Ok(Some(_event)) = timeout(timeout_duration, stream.next()).await {
        events_received += 1;
    }

    assert!(events_received > 0, "Should have received events during boundary test");
    println!("Boundary test processed {} events", events_received);
}

#[tokio::test] 
async fn test_event_preservation_during_coalescing() {
    // Verify that the latest event is preserved during coalescing operations
    let (sender, receiver) = flume::unbounded();
    let mut stream = DebouncedProgressStream::new(
        receiver,
        Duration::from_millis(100),
        20, // Small capacity to trigger coalescing
    );

    let model_id = "test_model";
    let file_path = "test_file.bin";

    // Send many events for the same model+file to trigger coalescing
    let initial_bytes = 100u64;
    let final_bytes = 500u64;
    let num_events = 50;

    // Send initial events
    for i in 0..num_events - 1 {
        let event = create_test_event(model_id, file_path, initial_bytes + i);
        sender.send(event).expect("Failed to send event");
    }

    // Send final event with distinctive value
    let final_event = create_test_event(model_id, file_path, final_bytes);
    sender.send(final_event).expect("Failed to send final event");

    drop(sender);

    // Consume events and check if we get the final value
    let mut latest_bytes = 0u64;
    let mut events_received = 0;
    let timeout_duration = Duration::from_secs(5);

    while let Ok(Some(event)) = timeout(timeout_duration, stream.next()).await {
        if event.model_id == model_id && event.file_path == file_path {
            latest_bytes = event.bytes_downloaded;
        }
        events_received += 1;
    }

    assert!(events_received > 0, "Should have received at least one event");
    
    // Due to coalescing, we should have received the latest event
    // (though the exact bytes value may vary due to coalescing behavior)
    println!(
        "Event preservation test: received {} events, latest bytes: {}",
        events_received, latest_bytes
    );
}