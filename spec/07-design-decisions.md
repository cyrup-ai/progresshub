# Design Decisions

## Overview

This document captures the key design decisions and their rationale.

## 1. Use flume, not std::sync::mpsc

**Decision**: Use `flume` channels throughout the codebase.

**Rationale**:
- Better async performance - designed for async from the ground up
- Works in both sync and async contexts without separate types
- More ergonomic API (e.g., `recv()` returns `Result` not `Option`)
- Better error messages
- Already in our dependencies

## 2. Channel per Download Session

**Decision**: Each `download()` call creates its own progress channel.

**Rationale**:
- Clear lifecycle - channel lives as long as download
- No global state to manage
- Multiple concurrent downloads don't interfere
- Easy to test in isolation

**Alternative considered**: One global channel for app lifetime
- Rejected because it complicates cleanup and testing

## 3. Model ID Passed Down, Not Extracted

**Decision**: Model ID is passed from orchestrator to handler, not parsed from paths.

**Rationale**:
- Path parsing is fragile (what if path format changes?)
- Orchestrator knows the model being downloaded
- Clear data flow from source of truth

**Implementation**:
```rust
let handler = ChannelProgressHandler::new(tx).for_model(model_id);
```

## 4. Ratatui v0.30 as Separate Task

**Decision**: Don't mix ratatui v0.30 upgrade with channel refactoring.

**Rationale**:
- Keep changes focused and reviewable
- Ratatui upgrade might have its own issues
- Can test channel changes independently
- Already documented that we want v0.30 in ARCHITECTURE.md

## 5. Error Only on Final Failure

**Decision**: Only send `ProgressData` with `error: Some(...)` on final, unrecoverable failures.

**Rationale**:
- Transient errors (network blips) are retried automatically
- UI doesn't need to show every retry
- Clear semantics: error = this won't complete
- Reduces noise in progress stream

**Examples**:
- Network timeout that retries: No error sent
- 404 Not Found: Error sent (can't retry)
- Disk full: Error sent (can't recover)

## 6. Clean API Design

**Decision**: API returns `(Future, ProgressReceiver)` tuple.

**Rationale**:
- Clean design from the start
- No trait object overhead
- Direct access to channels
- Simple and explicit

## 7. ChannelProgressHandler as Adapter

**Decision**: Keep `ProgressHandler` trait, implement `ChannelProgressHandler` as adapter.

**Rationale**:
- Download backends already use `ProgressHandler` trait
- No need to change all backends
- Clean adapter pattern
- Could add other handlers later (e.g., LoggingProgressHandler)

## 8. Time-based Throttling in Handler

**Decision**: Throttle progress updates to 100ms per file in the handler.

**Rationale**:
- Prevents UI flooding
- Reduces channel traffic
- Still responsive (10 updates/second max)
- First and last updates always sent

## 9. Two Channels Total

**Decision**: Exactly two channels - progress and bandwidth.

**Rationale**:
- Simple to understand
- Different domains, different channels
- No complex multiplexing needed
- Easy to add third channel if needed later

## 10. Focus on Implementation Only

**Decision**: This implementation work excludes tests and documentation.

**Rationale**:
- Another specialized agent handles all testing
- Another specialized agent handles all documentation  
- Mixing concerns slows down implementation
- Specialized agents can rip through tests/docs in minutes
- Clean separation of implementation vs validation/documentation work

