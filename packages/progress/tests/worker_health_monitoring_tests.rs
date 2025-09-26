//! Comprehensive tests for Worker Thread Health Monitoring system
//!
//! Tests verify:
//! - Worker heartbeat tracking and timing
//! - Health metrics collection and accuracy
//! - Unhealthy worker detection and thresholds
//! - Automatic worker restart functionality
//! - Health report aggregation and status determination
//! - Thread safety under concurrent operations

use std::sync::Arc;
use std::time::Duration;
use std::thread;

use progresshub_progress::orchestration::download_orchestrator::{
    WorkerHealthStatus, WorkerHealthMonitor
};

#[test]
fn test_worker_health_status_creation() {
    let worker_id = 42;
    let health_status = WorkerHealthStatus::new(worker_id);
    
    // Verify initial state
    assert_eq!(health_status.worker_id, worker_id);
    assert!(health_status.is_healthy.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(health_status.jobs_processed.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(health_status.jobs_failed.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(health_status.restart_count.load(std::sync::atomic::Ordering::Relaxed), 0);
    
    // Verify heartbeat is recent (within last second)
    let metrics = health_status.get_metrics();
    assert!(metrics.time_since_heartbeat_ms < 1000, "Initial heartbeat should be recent");
}

#[test]
fn test_worker_heartbeat_tracking() {
    let health_status = WorkerHealthStatus::new(0);
    
    // Get initial heartbeat time
    let initial_metrics = health_status.get_metrics();
    assert!(initial_metrics.time_since_heartbeat_ms < 1000, "Initial heartbeat should be recent");
    
    // Wait a measurable amount and send new heartbeat
    thread::sleep(Duration::from_millis(50));
    health_status.heartbeat();
    
    // Verify heartbeat was updated (should be more recent)
    let updated_metrics = health_status.get_metrics();
    assert!(updated_metrics.time_since_heartbeat_ms < 200, 
        "Recent heartbeat should be within 200ms of heartbeat() call");
}

#[test]
fn test_job_completion_tracking() {
    let health_status = WorkerHealthStatus::new(0);
    
    // Track successful jobs
    health_status.job_completed();
    health_status.job_completed();
    health_status.job_completed();
    
    // Track failed jobs
    health_status.job_failed();
    
    let metrics = health_status.get_metrics();
    assert_eq!(metrics.jobs_processed, 3);
    assert_eq!(metrics.jobs_failed, 1);
    
    // Verify success rate calculation
    let success_rate = metrics.success_rate();
    assert!((success_rate - 75.0).abs() < 0.1, 
        "Success rate should be 75% (3 successful out of 4 total)");
}

#[test]
fn test_worker_responsiveness_checking() {
    let health_status = WorkerHealthStatus::new(0);
    
    // Worker should be responsive initially
    let metrics = health_status.get_metrics();
    assert!(metrics.is_responsive(30_000), "Worker should be responsive within 30 seconds");
    assert!(metrics.is_responsive(1_000), "Worker should be responsive within 1 second initially");
    
    // Simulate old heartbeat by manipulating time
    // Note: This test verifies the responsiveness calculation logic
    let old_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64 - 35_000; // 35 seconds ago
    
    health_status.last_heartbeat.store(old_timestamp, std::sync::atomic::Ordering::Relaxed);
    
    let metrics = health_status.get_metrics();
    assert!(!metrics.is_responsive(30_000), "Worker should not be responsive after 35 seconds");
    assert!(metrics.is_responsive(40_000), "Worker should still be responsive within 40 seconds");
}

#[test]
fn test_worker_health_monitor_creation() {
    let worker_count = 8;
    let health_monitor = WorkerHealthMonitor::new(worker_count);
    
    // Verify all workers are created
    for worker_id in 0..worker_count {
        let worker_health = health_monitor.get_worker_health(worker_id);
        assert!(worker_health.is_some(), "Worker {} should exist", worker_id);
        
        let status = worker_health.unwrap();
        assert_eq!(status.worker_id, worker_id);
        assert!(status.is_healthy.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    // Verify non-existent worker returns None
    assert!(health_monitor.get_worker_health(worker_count).is_none());
}

#[test]
fn test_health_report_generation() {
    let worker_count = 4;
    let health_monitor = WorkerHealthMonitor::new(worker_count);
    
    // Simulate some job activity
    if let Some(worker0) = health_monitor.get_worker_health(0) {
        worker0.job_completed();
        worker0.job_completed();
    }
    
    if let Some(worker1) = health_monitor.get_worker_health(1) {
        worker1.job_completed();
        worker1.job_failed();
    }
    
    // Mark one worker as unhealthy
    if let Some(worker2) = health_monitor.get_worker_health(2) {
        worker2.mark_unhealthy();
    }
    
    let health_report = health_monitor.get_health_report();
    
    // Verify report structure
    assert_eq!(health_report.total_workers, worker_count);
    assert_eq!(health_report.worker_metrics.len(), worker_count);
    
    // Verify job counting
    assert_eq!(health_report.total_jobs_processed, 3); // 2 from worker0 + 1 from worker1
    assert_eq!(health_report.total_jobs_failed, 1); // 1 from worker1
    
    // Verify health calculation
    assert_eq!(health_report.healthy_workers, 3); // workers 0, 1, 3 should be healthy
    assert_eq!(health_report.workers_needing_restart, 1); // worker 2 is unhealthy
    
    // Verify overall success rate
    let success_rate = health_report.overall_success_rate();
    assert!((success_rate - 75.0).abs() < 0.1, 
        "Overall success rate should be 75% (3 successful out of 4 total)");
    
    // Verify overall health percentage
    let health_percentage = health_report.overall_health_percentage();
    assert_eq!(health_percentage, 75.0); // 3 out of 4 workers healthy
}

#[test]
fn test_health_status_determination() {
    let health_monitor = WorkerHealthMonitor::new(4);
    
    // All healthy - should be HEALTHY status
    let healthy_report = health_monitor.get_health_report();
    assert_eq!(healthy_report.status_summary(), "HEALTHY");
    assert!(!healthy_report.is_critical());
    
    // Mark one worker unhealthy - should be WARNING status
    if let Some(worker) = health_monitor.get_worker_health(0) {
        worker.mark_unhealthy();
    }
    
    let warning_report = health_monitor.get_health_report();
    assert_eq!(warning_report.status_summary(), "WARNING");
    assert!(!warning_report.is_critical());
    
    // Mark majority unhealthy - should be CRITICAL status
    if let Some(worker) = health_monitor.get_worker_health(1) {
        worker.mark_unhealthy();
    }
    if let Some(worker) = health_monitor.get_worker_health(2) {
        worker.mark_unhealthy();
    }
    
    let critical_report = health_monitor.get_health_report();
    assert_eq!(critical_report.status_summary(), "CRITICAL");
    assert!(critical_report.is_critical());
}

#[test]
fn test_worker_restart_marking() {
    let health_status = Arc::new(WorkerHealthStatus::new(0));
    
    // Initially healthy with no restarts
    assert!(health_status.is_healthy.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(health_status.restart_count.load(std::sync::atomic::Ordering::Relaxed), 0);
    
    // Mark unhealthy
    health_status.mark_unhealthy();
    assert!(!health_status.is_healthy.load(std::sync::atomic::Ordering::Relaxed));
    
    // Verify thread handle was taken
    {
        let handle_guard = health_status.thread_handle.lock();
        assert!(handle_guard.is_none(), "Thread handle should be taken when marked unhealthy");
    }
    
    // Simulate restart with new handle
    let new_handle = thread::Builder::new()
        .name("test-worker".to_string())
        .spawn(|| {
            // Empty test thread
            thread::sleep(Duration::from_millis(10));
        })
        .expect("Failed to spawn test thread");
    
    health_status.mark_restarted(new_handle);
    
    // Verify restart state
    assert!(health_status.is_healthy.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(health_status.restart_count.load(std::sync::atomic::Ordering::Relaxed), 1);
    
    // Verify thread handle was restored
    {
        let handle_guard = health_status.thread_handle.lock();
        assert!(handle_guard.is_some(), "Thread handle should be restored after restart");
    }
}

#[test]
fn test_concurrent_health_operations() {
    let health_status = Arc::new(WorkerHealthStatus::new(0));
    let num_threads = 8;
    let operations_per_thread = 100;
    
    let mut handles = Vec::new();
    
    // Spawn threads to perform concurrent operations
    for thread_id in 0..num_threads {
        let health_status_clone = Arc::clone(&health_status);
        
        let handle = thread::spawn(move || {
            for i in 0..operations_per_thread {
                match thread_id % 4 {
                    0 => health_status_clone.heartbeat(),
                    1 => health_status_clone.job_completed(),
                    2 => health_status_clone.job_failed(),
                    3 => {
                        let _metrics = health_status_clone.get_metrics();
                        health_status_clone.update_uptime();
                    }
                    _ => unreachable!(),
                }
                
                // Small delay to increase concurrency chance
                if i % 10 == 0 {
                    thread::sleep(Duration::from_micros(1));
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    // Verify final state is consistent
    let final_metrics = health_status.get_metrics();
    
    // Calculate expected job operations: thread_id % 4 determines operation type
    // threads 1, 5 (% 4 == 1) do job_completed, threads 2, 6 (% 4 == 2) do job_failed
    let completing_threads = 2; // threads 1, 5 
    let failing_threads = 2; // threads 2, 6
    let expected_completions = operations_per_thread * completing_threads;
    let expected_failures = operations_per_thread * failing_threads;
    let expected_total_operations = expected_completions + expected_failures;
    
    assert_eq!(final_metrics.jobs_processed + final_metrics.jobs_failed, expected_total_operations,
        "Expected {} total job operations, got {} (processed: {}, failed: {})", 
        expected_total_operations, 
        final_metrics.jobs_processed + final_metrics.jobs_failed,
        final_metrics.jobs_processed,
        final_metrics.jobs_failed);
    
    // Success rate should be exactly 50% (threads 1,5 complete, threads 2,6 fail)
    let success_rate = final_metrics.success_rate();
    assert_eq!(final_metrics.jobs_processed, expected_completions, "Should have exactly {} completions", expected_completions);
    assert_eq!(final_metrics.jobs_failed, expected_failures, "Should have exactly {} failures", expected_failures);
    assert_eq!(success_rate, 50.0, "Success rate should be exactly 50%");
    
    // Worker should still be healthy and responsive
    assert!(final_metrics.is_healthy);
    assert!(final_metrics.is_responsive(1000));
}

#[test]
fn test_health_monitoring_thread_safety() {
    let worker_count = 16;
    let health_monitor = Arc::new(WorkerHealthMonitor::new(worker_count));
    let num_reader_threads = 4;
    let num_writer_threads = 4;
    let operations_per_thread = 50;
    
    let mut handles = Vec::new();
    
    // Spawn reader threads that get health reports
    for _ in 0..num_reader_threads {
        let health_monitor_clone = Arc::clone(&health_monitor);
        
        let handle = thread::spawn(move || {
            for _ in 0..operations_per_thread {
                let _report = health_monitor_clone.get_health_report();
                thread::sleep(Duration::from_micros(100));
            }
        });
        
        handles.push(handle);
    }
    
    // Spawn writer threads that modify worker health
    for thread_id in 0..num_writer_threads {
        let health_monitor_clone = Arc::clone(&health_monitor);
        
        let handle = thread::spawn(move || {
            for i in 0..operations_per_thread {
                let worker_id = (thread_id + i) % worker_count;
                
                if let Some(worker_health) = health_monitor_clone.get_worker_health(worker_id) {
                    match i % 3 {
                        0 => worker_health.heartbeat(),
                        1 => worker_health.job_completed(),
                        2 => worker_health.job_failed(),
                        _ => unreachable!(),
                    }
                }
                
                thread::sleep(Duration::from_micros(100));
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    // Verify final state is consistent
    let final_report = health_monitor.get_health_report();
    assert_eq!(final_report.total_workers, worker_count);
    assert_eq!(final_report.worker_metrics.len(), worker_count);
    
    // All workers should still be healthy (no failures injected)
    assert_eq!(final_report.healthy_workers, worker_count);
    assert_eq!(final_report.workers_needing_restart, 0);
    assert_eq!(final_report.status_summary(), "HEALTHY");
}