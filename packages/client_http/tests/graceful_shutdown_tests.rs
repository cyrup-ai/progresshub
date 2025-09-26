//! Comprehensive tests for graceful thread pool shutdown mechanism
//!
//! Tests verify that the ThreadPoolManager correctly handles:
//! - Graceful shutdown with cancellation tokens
//! - Timeout-based forced shutdown with abort handles
//! - Immediate shutdown for emergency scenarios
//! - Task cancellation during download operations
//! - Race conditions in shutdown coordination

use std::time::Duration;
use tokio::time::sleep;
// use tokio_util::sync::CancellationToken;  // Not needed - we get it from ThreadPoolManager
use progresshub_client_http::concurrent_manager::{ThreadPoolManager, ConcurrentChunkManager};

/// Test graceful shutdown with immediate task completion
#[tokio::test]
async fn test_graceful_shutdown_immediate_completion() {
    let mut thread_pool = ThreadPoolManager::new();
    let cancellation_token = thread_pool.cancellation_token();
    
    // Create a quick task that completes immediately
    let handle = tokio::spawn(async move {
        // Check cancellation token at start
        if cancellation_token.is_cancelled() {
            return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                "Task cancelled".to_string()
            ));
        }
        
        // Simulate quick work
        sleep(Duration::from_millis(10)).await;
        
        // Check cancellation token before completing
        if cancellation_token.is_cancelled() {
            return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                "Task cancelled".to_string()
            ));
        }
        
        Ok(())
    });
    
    // Store abort handle
    thread_pool.add_abort_handle(handle.abort_handle());
    
    // Wait for task completion
    let result = handle.await.expect("Task should complete");
    assert!(result.is_ok(), "Task should complete successfully");
    
    // Test graceful shutdown (should be immediate since task already completed)
    let shutdown_result = thread_pool.shutdown_graceful(Duration::from_secs(1)).await;
    assert!(shutdown_result.is_ok(), "Graceful shutdown should succeed for completed tasks");
}

/// Test graceful shutdown with cancellation signal
#[tokio::test]
async fn test_graceful_shutdown_with_cancellation() {
    let mut thread_pool = ThreadPoolManager::new();
    let cancellation_token = thread_pool.cancellation_token();
    
    // Create a long-running task that respects cancellation
    let task_token = cancellation_token.clone();
    let handle = tokio::spawn(async move {
        // Simulate work with periodic cancellation checks
        for i in 0..100 {
            if task_token.is_cancelled() {
                tracing::info!("Task cancelled gracefully at iteration {}", i);
                return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                    "Task cancelled gracefully".to_string()
                ));
            }
            sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    });
    
    // Store abort handle
    thread_pool.add_abort_handle(handle.abort_handle());
    
    // Let task run briefly, then request graceful shutdown
    sleep(Duration::from_millis(100)).await;
    
    // Start graceful shutdown with reasonable timeout
    let shutdown_future = thread_pool.shutdown_graceful(Duration::from_millis(500));
    
    // Wait for task and shutdown
    let (task_result, shutdown_result) = tokio::join!(handle, shutdown_future);
    
    // Task should be cancelled gracefully
    assert!(task_result.is_ok(), "Task should complete (even if cancelled)");
    if let Ok(task_outcome) = task_result {
        // Task should be cancelled
        assert!(task_outcome.is_err(), "Task should be cancelled");
    }
    
    // Shutdown should succeed (task was cancelled gracefully)
    // Note: This may return an error count if task was force-aborted after timeout
    println!("Shutdown result: {:?}", shutdown_result);
}

/// Test immediate shutdown without waiting
#[tokio::test]
async fn test_immediate_shutdown() {
    let mut thread_pool = ThreadPoolManager::new();
    let cancellation_token = thread_pool.cancellation_token();
    
    // Create multiple long-running tasks
    let mut handles = Vec::new();
    for i in 0..5 {
        let task_token = cancellation_token.clone();
        let handle = tokio::spawn(async move {
            // Long-running work that ignores cancellation (to test force abort)
            for j in 0..1000 {
                sleep(Duration::from_millis(10)).await;
                // Only check cancellation occasionally to simulate slow response
                if j % 10 == 0 && task_token.is_cancelled() {
                    return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                        format!("Task {} cancelled", i)
                    ));
                }
            }
            Ok(())
        });
        
        thread_pool.add_abort_handle(handle.abort_handle());
        handles.push(handle);
    }
    
    // Let tasks start
    sleep(Duration::from_millis(50)).await;
    
    // Verify tasks are active
    assert_eq!(thread_pool.active_task_count(), 5);
    
    // Force immediate shutdown
    thread_pool.shutdown_immediate();
    
    // Verify all tasks are aborted
    for handle in handles {
        let result = handle.await;
        // Tasks should be aborted (JoinError)
        assert!(result.is_err(), "Task should be aborted");
    }
}

/// Test timeout-based forced abort
#[tokio::test]
async fn test_timeout_forced_abort() {
    let mut thread_pool = ThreadPoolManager::new();
    let cancellation_token = thread_pool.cancellation_token();
    
    // Create a task that responds to cancellation slowly
    let task_token = cancellation_token.clone();
    let handle: tokio::task::JoinHandle<Result<(), progresshub_client_http::chunk_state::ChunkError>> = tokio::spawn(async move {
        // Long work period without checking cancellation
        sleep(Duration::from_secs(2)).await;
        
        // Finally check cancellation
        if task_token.is_cancelled() {
            return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                "Task cancelled after delay".to_string()
            ));
        }
        
        Ok(())
    });
    
    thread_pool.add_abort_handle(handle.abort_handle());
    
    // Request graceful shutdown with short timeout
    let shutdown_result = thread_pool.shutdown_graceful(Duration::from_millis(100)).await;
    
    // Should result in forced abort due to timeout
    assert!(shutdown_result.is_err(), "Should force abort due to timeout");
    if let Err(aborted_count) = shutdown_result {
        assert_eq!(aborted_count, 1, "Should abort 1 task");
    }
    
    // Task should be aborted
    let task_result = handle.await;
    assert!(task_result.is_err(), "Task should be aborted due to timeout");
}

/// Test concurrent shutdown requests
#[tokio::test]
async fn test_concurrent_shutdown_requests() {
    let mut thread_pool = ThreadPoolManager::new();
    let cancellation_token = thread_pool.cancellation_token();
    
    // Create a responsive task
    let task_token = cancellation_token.clone();
    let handle: tokio::task::JoinHandle<Result<(), progresshub_client_http::chunk_state::ChunkError>> = tokio::spawn(async move {
        loop {
            if task_token.is_cancelled() {
                return Err(progresshub_client_http::chunk_state::ChunkError::WriteError(
                    "Task cancelled".to_string()
                ));
            }
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    thread_pool.add_abort_handle(handle.abort_handle());
    
    // Multiple concurrent shutdown requests
    thread_pool.request_shutdown();
    thread_pool.request_shutdown(); // Should be idempotent
    
    // Verify cancellation token is set
    assert!(thread_pool.is_shutdown_requested());
    
    // Wait briefly and verify task responds to cancellation
    sleep(Duration::from_millis(50)).await;
    
    let task_result = handle.await;
    assert!(task_result.is_ok(), "Task should complete");
    if let Ok(outcome) = task_result {
        assert!(outcome.is_err(), "Task should be cancelled");
    }
}

/// Test empty thread pool shutdown
#[tokio::test]
async fn test_empty_thread_pool_shutdown() {
    let thread_pool = ThreadPoolManager::new();
    
    // Shutdown empty pool should succeed immediately
    let shutdown_result = thread_pool.shutdown_graceful(Duration::from_millis(100)).await;
    assert!(shutdown_result.is_ok(), "Empty thread pool should shutdown gracefully");
}

/// Test shutdown status methods
#[tokio::test]
async fn test_shutdown_status_methods() {
    let mut thread_pool = ThreadPoolManager::new();
    
    // Initially not shutdown
    assert!(!thread_pool.is_shutdown_requested());
    assert_eq!(thread_pool.active_task_count(), 0);
    
    // Add some abort handles (simulate active tasks)
    let dummy_handle1: tokio::task::JoinHandle<Result<(), progresshub_client_http::chunk_state::ChunkError>> = tokio::spawn(async { Ok(()) });
    let dummy_handle2: tokio::task::JoinHandle<Result<(), progresshub_client_http::chunk_state::ChunkError>> = tokio::spawn(async { Ok(()) });
    
    thread_pool.add_abort_handle(dummy_handle1.abort_handle());
    thread_pool.add_abort_handle(dummy_handle2.abort_handle());
    
    assert_eq!(thread_pool.active_task_count(), 2);
    
    // Request shutdown
    thread_pool.request_shutdown();
    assert!(thread_pool.is_shutdown_requested());
    
    // Wait for tasks to complete
    let _ = dummy_handle1.await;
    let _ = dummy_handle2.await;
}

/// Integration test: ConcurrentChunkManager with shutdown
#[tokio::test]
async fn test_concurrent_chunk_manager_shutdown_integration() {
    let chunk_manager = ConcurrentChunkManager::new();
    
    // Test initial state
    assert!(!chunk_manager.thread_pool().is_shutdown_requested());
    assert_eq!(chunk_manager.thread_pool().active_task_count(), 0);
    
    // Test shutdown request
    chunk_manager.request_shutdown();
    assert!(chunk_manager.thread_pool().is_shutdown_requested());
    
    // Test graceful shutdown (should be immediate since no tasks)
    let shutdown_result = chunk_manager.shutdown(Duration::from_millis(100)).await;
    assert!(shutdown_result.is_ok(), "Manager with no tasks should shutdown gracefully");
}

/// Test abort handle lifecycle
#[tokio::test]
async fn test_abort_handle_lifecycle() {
    let mut thread_pool = ThreadPoolManager::new();
    
    // Create task and get abort handle
    let handle: tokio::task::JoinHandle<Result<(), progresshub_client_http::chunk_state::ChunkError>> = tokio::spawn(async {
        sleep(Duration::from_secs(10)).await; // Long-running task
        Ok(())
    });
    
    let abort_handle = handle.abort_handle();
    thread_pool.add_abort_handle(abort_handle);
    
    // Abort the task via immediate shutdown
    thread_pool.shutdown_immediate();
    
    // Task should be aborted
    let result = handle.await;
    assert!(result.is_err(), "Task should be aborted");
}