//! Tests for Visual Completion Flow functionality
//!
//! This module contains comprehensive tests for the completion state machine
//! including state transitions, timing, and visual feedback verification.

use progresshub_tui::ui::state::{AppState, CompletionState};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

#[test]
fn test_completion_state_transitions() {
    let mut app_state = AppState::default();

    // Initial state should be Active
    assert_eq!(app_state.completion_state, CompletionState::Active);
    assert!(!app_state.should_quit());

    // Simulate downloads completing - should trigger visual completion
    app_state.handle_downloads_complete();
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );
    assert!(!app_state.should_quit()); // Should not quit during visual phase

    // Simulate completion timer finishing - should transition to Complete
    app_state.completion_state = CompletionState::Complete;
    assert!(app_state.should_quit()); // Should quit after visual completion
}

#[test]
fn test_manual_quit_during_completion() {
    let mut app_state = AppState::default();

    // Start visual completion
    app_state.handle_downloads_complete();
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );

    // Manual quit during visual completion should work
    app_state.should_quit = true;
    assert!(app_state.should_quit());
}

#[test]
fn test_completion_timer_initialization() {
    let mut app_state = AppState::default();

    // Timer should not be initialized initially
    assert!(app_state.completion_timer.is_none());

    // After downloads complete, should be in visual completion (no timer yet)
    app_state.handle_downloads_complete();
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );
    assert!(app_state.completion_timer.is_none()); // Timer not set yet

    // Transition to Complete state should set the timer
    app_state.transition_to_complete();
    assert!(app_state.completion_timer.is_some());

    // Timer should be recent (within last second)
    let timer = app_state
        .completion_timer
        .expect("Timer should be initialized");
    let elapsed = timer.elapsed();
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_completion_state_machine_edge_cases() {
    let mut app_state = AppState {
        should_quit: true,
        ..Default::default()
    };
    assert!(app_state.should_quit());

    // Test multiple completion calls don't break state
    app_state.should_quit = false;
    app_state.handle_downloads_complete();
    let first_timer = app_state.completion_timer;

    app_state.handle_downloads_complete(); // Second call
    let second_timer = app_state.completion_timer;

    // Timer should not be reset on multiple calls
    assert_eq!(first_timer, second_timer);
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );
}

#[tokio::test]
async fn test_completion_flow_integration() {
    let mut app_state = AppState::default();
    let (_done_tx, _done_rx) = oneshot::channel::<()>();

    // Simulate the main event loop logic
    assert_eq!(app_state.completion_state, CompletionState::Active);

    // Simulate downloads completing - enters visual completion
    app_state.handle_downloads_complete();
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );
    assert!(app_state.completion_timer.is_none()); // No timer yet

    // Transition to Complete state (sets timer)
    app_state.transition_to_complete();
    assert_eq!(app_state.completion_state, CompletionState::Complete);
    assert!(app_state.completion_timer.is_some());

    // Simulate timer logic (without actually waiting 2 seconds)
    app_state.completion_timer = Some(Instant::now() - Duration::from_secs(3));

    // Check if completion timer has elapsed (simulating main loop logic)
    if let Some(timer) = app_state.completion_timer
        && timer.elapsed() >= Duration::from_secs(2)
    {
        app_state.completion_state = CompletionState::Complete;
    }

    assert_eq!(app_state.completion_state, CompletionState::Complete);
    assert!(app_state.should_quit());
}

#[test]
fn test_visual_completion_requirements() {
    let mut app_state = AppState::default();

    // Test that visual completion state allows for UI updates
    app_state.handle_downloads_complete();
    assert_eq!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    );

    // During visual completion, we should be able to check completion state
    assert!(matches!(
        app_state.completion_state,
        CompletionState::VisualCompletion
    ));

    // This state allows the UI to show:
    // - 100% progress on all models
    // - Green completion state
    // - Checkmarks on models
    // - "COMPLETE ✅" on overall progress

    // The state should persist for the timer duration
    assert!(!app_state.should_quit()); // Don't quit yet, show visual feedback
}
