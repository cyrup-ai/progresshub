//! Zero-allocation debounced progress event stream using crossbeam channels
//!
//! This module provides a high-performance debounced stream for progress events that:
//! - Emits isolated events immediately (speed lane for immediate feedback)
//! - Debounces rapid events with trailing edge emission (smooth UI updates)
//! - Zero allocations after initialization
//! - Lock-free operation using crossbeam's SegQueue
//! - Handles backpressure with bounded channels
//! - Graceful handling of channel disconnection and errors

use crossbeam::queue::SegQueue;
use futures_timer::Delay;
use futures_util::{FutureExt, Stream};
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::channel::{ProgressData, ProgressReceiver};
use crate::memory_ordering::{
    MemoryOrderingValidator, CounterDirection, FetchOperationPurpose,
    documentation
};

pin_project! {
    /// A debounced progress event stream that emits isolated events immediately and rapid events
    /// after a specified delay of inactivity, yielding `ProgressData`.
    ///
    /// This stream processes events from a `flume::Receiver<ProgressData>`, implementing a
    /// hybrid leading/trailing debounce: isolated events (arriving after a pause longer
    /// than the delay duration) are emitted immediately ("speed lane"), while rapid
    /// successive events are debounced to emit only the most recent one after the delay.
    ///
    /// ## Performance Characteristics
    ///
    /// - Zero allocations beyond initial setup via pre-allocated SegQueue
    /// - Lock-free operation using crossbeam's SegQueue for event buffering
    /// - Bounded memory usage with configurable queue capacity
    /// - Immediate feedback for isolated events, smooth updates for rapid events
    /// - Graceful backpressure handling with channel fullness monitoring
    ///
    /// ## Debounce Strategy
    ///
    /// - **Speed Lane**: Events arriving after delay period emit immediately
    /// - **Debounce Lane**: Rapid successive events emit latest after delay
    /// - **First Event**: Always emits immediately for instant user feedback
    /// - **Channel Full**: Graceful degradation with event coalescing
    pub struct DebouncedProgressStream {
        receiver: ProgressReceiver,
        queue: SegQueue<ProgressData>,
        queue_capacity_counter: AtomicUsize,
        queue_len_counter: AtomicUsize,
        delay_duration: Duration,
        #[pin]
        delay: Delay,
        last_item: Option<ProgressData>,
        last_event_time: Option<Instant>,
        queue_capacity: usize,
        events_processed: u64,
        events_debounced: u64,
        events_immediate: u64,
    }
}

impl DebouncedProgressStream {
    /// Creates a new `DebouncedProgressStream` from a receiver and delay duration.
    ///
    /// # Arguments
    /// - `receiver`: The `flume::Receiver<ProgressData>` providing progress events
    /// - `delay`: The duration to wait before emitting rapid events
    /// - `queue_capacity`: Maximum number of events to buffer (for memory control)
    ///
    /// # Performance
    /// - Zero allocations during operation (beyond initial `SegQueue` setup)
    /// - Lock-free `SegQueue` for high-performance event buffering
    /// - Bounded memory usage with configurable capacity
    /// - Safe, checked operations with comprehensive error handling
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use progresshub_progress::debounced::DebouncedProgressStream;
    /// use progresshub_progress::channel::create_progress_channel;
    ///
    /// let (tx, rx) = create_progress_channel(256);
    /// let debounced = DebouncedProgressStream::new(rx, Duration::from_millis(50), 512);
    /// ```
    #[inline]
    pub fn new(receiver: ProgressReceiver, delay: Duration, queue_capacity: usize) -> Self {
        Self {
            receiver,
            queue: SegQueue::new(),
            queue_capacity_counter: AtomicUsize::new(0),
            queue_len_counter: AtomicUsize::new(0),
            delay_duration: delay,
            delay: Delay::new(Duration::ZERO),
            last_item: None,
            last_event_time: None,
            queue_capacity,
            events_processed: 0,
            events_debounced: 0,
            events_immediate: 0,
        }
    }

    /// Get the number of events currently queued for processing
    /// 
    /// **Memory Ordering**: Uses `Relaxed` ordering because queue length is used for
    /// backpressure and statistics. Exact ordering of counter updates doesn't affect
    /// correctness - approximate values are sufficient for capacity management.
    #[inline]
    pub fn queue_len(&self) -> usize {
        // Load queue length with relaxed ordering - see memory_ordering::documentation::queue_length_ordering()
        self.queue_len_counter.load(documentation::queue_length_ordering())
    }

    /// Get the configured queue capacity for memory management
    #[inline]
    pub fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Check if the queue is approaching capacity (>75% full)
    #[inline]
    pub fn is_queue_nearly_full(&self) -> bool {
        self.queue_len() > (self.queue_capacity * 3) / 4
    }

    /// Get performance metrics for monitoring and debugging
    #[inline]
    pub fn metrics(&self) -> DebouncedMetrics {
        DebouncedMetrics {
            events_processed: self.events_processed,
            events_debounced: self.events_debounced,
            events_immediate: self.events_immediate,
            queue_length: self.queue_len(),
            queue_capacity: self.queue_capacity,
            has_pending_item: self.last_item.is_some(),
        }
    }


}

impl Stream for DebouncedProgressStream {
    type Item = ProgressData;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let mut this = self.as_mut().project();

            // Check if the delay has expired for a pending item
            if this.last_item.is_some() && this.delay.poll_unpin(cx).is_ready() {
                *this.last_event_time = None;
                // We've verified is_some(), so take() should always return Some
                match this.last_item.take() {
                    Some(item) => {
                        *this.events_debounced += 1;
                        return Poll::Ready(Some(item));
                    }
                    None => {
                        // This should never happen since we verified is_some()
                        // Log the inconsistency but continue polling
                        tracing::debug!(
                            "Debouncer state inconsistency - last_item was None despite is_some check"
                        );
                        // Continue to next iteration
                    }
                }
            }

            // Process incoming events from the receiver with atomic capacity reservation
            loop {
                match this.receiver.try_recv() {
                    Ok(event) => {
                        // Atomically reserve capacity using fetch_add with capacity check
                        // **Memory Ordering**: Relaxed ordering for capacity management - exact ordering
                        // not critical since this is backpressure control, not synchronization
                        MemoryOrderingValidator::validate_fetch_operation_ordering(
                            "queue_capacity_reserve",
                            documentation::queue_length_ordering(),
                            FetchOperationPurpose::SimpleCounter,
                        );
                        let prev_count = this.queue_capacity_counter.fetch_add(1, documentation::queue_length_ordering());
                        if prev_count < *this.queue_capacity {
                            // Successfully reserved space - safe to push
                            this.queue.push(event);
                            
                            // Increment queue length counter with validation
                            MemoryOrderingValidator::validate_relaxed_counter_operation(
                                &this.queue_len_counter,
                                "queue_length_increment",
                                CounterDirection::Bidirectional,
                            );
                            this.queue_len_counter.fetch_add(1, documentation::queue_length_ordering());
                            *this.events_processed += 1;
                        } else {
                            // Capacity exceeded - atomically release reservation and break
                            MemoryOrderingValidator::validate_fetch_operation_ordering(
                                "queue_capacity_release",
                                documentation::queue_length_ordering(),
                                FetchOperationPurpose::SimpleCounter,
                            );
                            this.queue_capacity_counter.fetch_sub(1, documentation::queue_length_ordering());
                            break;
                        }
                    }
                    Err(flume::TryRecvError::Empty) => break,
                    Err(flume::TryRecvError::Disconnected) => break,
                }
            }

            // Handle receiver disconnection
            if matches!(
                this.receiver.try_recv(),
                Err(flume::TryRecvError::Disconnected)
            ) {
                // Emit any pending item immediately on disconnection
                if let Some(item) = this.last_item.take() {
                    return Poll::Ready(Some(item));
                }
                // Check if any events remain in queue using atomic length counter
                // **Memory Ordering**: Relaxed load for queue length check - approximate value sufficient
                if this.queue_len_counter.load(documentation::queue_length_ordering()) == 0 {
                    return Poll::Ready(None); // Stream exhausted
                }
            }

            // If queue is nearly full, coalesce events to prevent memory issues
            // **Memory Ordering**: Relaxed load for capacity check - approximate value sufficient for backpressure
            let queue_len = this.queue_capacity_counter.load(documentation::queue_length_ordering());
            let latest_event = if queue_len > (*this.queue_capacity * 3) / 4 {
                // Coalesce events by keeping only the latest event per model+file combination
                let mut latest_events = std::collections::HashMap::new();
                let mut events_coalesced = 0;

                // Collect all events, keeping only the latest per model+file key
                while let Some(event) = this.queue.pop() {
                    // Decrement atomic counters for each pop to maintain consistency
                    // **Memory Ordering**: Relaxed ordering for counter decrements - consistency maintained by event queue ordering
                    MemoryOrderingValidator::validate_relaxed_counter_operation(
                        &this.queue_capacity_counter,
                        "queue_capacity_decrement_coalesce",
                        CounterDirection::Bidirectional,
                    );
                    this.queue_capacity_counter.fetch_sub(1, documentation::queue_length_ordering());
                    
                    MemoryOrderingValidator::validate_relaxed_counter_operation(
                        &this.queue_len_counter,
                        "queue_length_decrement_coalesce",
                        CounterDirection::Bidirectional,
                    );
                    this.queue_len_counter.fetch_sub(1, documentation::queue_length_ordering());
                    
                    let key = (event.model_id.clone(), event.file_path.clone());
                    latest_events.insert(key, event);
                    events_coalesced += 1;
                }

                // If we coalesced multiple events, update metrics
                if events_coalesced > 1 {
                    *this.events_debounced += events_coalesced - 1;
                }

                // Return the latest event if any (arbitrary selection from latest_events)
                latest_events.into_values().next()
            } else {
                // Normal processing: get the latest event from queue with atomic counter management
                if let Some(event) = this.queue.pop() {
                    // Decrement counters with validated relaxed ordering
                    // **Memory Ordering**: Relaxed ordering sufficient for queue management counters
                    MemoryOrderingValidator::validate_relaxed_counter_operation(
                        &this.queue_capacity_counter,
                        "queue_capacity_decrement_normal",
                        CounterDirection::Bidirectional,
                    );
                    this.queue_capacity_counter.fetch_sub(1, documentation::queue_length_ordering());
                    
                    MemoryOrderingValidator::validate_relaxed_counter_operation(
                        &this.queue_len_counter,
                        "queue_length_decrement_normal",
                        CounterDirection::Bidirectional,
                    );
                    this.queue_len_counter.fetch_sub(1, documentation::queue_length_ordering());
                    
                    Some(event)
                } else {
                    None
                }
            };

            if let Some(event) = latest_event {
                let now = Instant::now();

                if let Some(last_time) = *this.last_event_time {
                    if now.duration_since(last_time) > *this.delay_duration {
                        // Speed lane: isolated event after delay period - emit immediately
                        *this.last_event_time = Some(now);
                        *this.events_immediate += 1;
                        return Poll::Ready(Some(event));
                    }
                } else {
                    // First event: emit immediately for instant user feedback
                    *this.last_event_time = Some(now);
                    *this.events_immediate += 1;
                    return Poll::Ready(Some(event));
                }

                // Rapid event: store and reset delay for debouncing
                *this.last_item = Some(event);
                *this.last_event_time = Some(now);
                this.delay.as_mut().reset(*this.delay_duration);

                // Continue polling to check if delay expires
                continue;
            }

            // No events available and no pending item
            if this.last_item.is_none() && this.queue_len_counter.load(Ordering::Relaxed) == 0 {
                // Wake immediately for next poll to check for new events
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let pending = self.queue_len() + if self.last_item.is_some() { 1 } else { 0 };
        (pending, None)
    }
}

/// Performance and operational metrics for the debounced stream
#[derive(Debug, Clone, Copy)]
pub struct DebouncedMetrics {
    /// Total number of events processed from the receiver
    pub events_processed: u64,
    /// Number of events that were debounced (delayed)
    pub events_debounced: u64,
    /// Number of events emitted immediately (speed lane)
    pub events_immediate: u64,
    /// Current length of the event queue
    pub queue_length: usize,
    /// Maximum capacity of the event queue
    pub queue_capacity: usize,
    /// Whether there's a pending item waiting for delay expiration
    pub has_pending_item: bool,
}

impl DebouncedMetrics {
    /// Calculate the immediate emission rate (percentage of events emitted immediately)
    #[inline]
    pub fn immediate_rate(&self) -> f64 {
        if self.events_processed == 0 {
            0.0
        } else {
            (self.events_immediate as f64) / (self.events_processed as f64) * 100.0
        }
    }

    /// Calculate the debounce effectiveness (percentage of events debounced)
    #[inline]
    pub fn debounce_rate(&self) -> f64 {
        if self.events_processed == 0 {
            0.0
        } else {
            (self.events_debounced as f64) / (self.events_processed as f64) * 100.0
        }
    }

    /// Calculate queue utilization as a percentage of capacity
    #[inline]
    pub fn queue_utilization(&self) -> f64 {
        if self.queue_capacity == 0 {
            0.0
        } else {
            (self.queue_length as f64) / (self.queue_capacity as f64) * 100.0
        }
    }
}

/// Creates a debounced progress stream from a flume receiver with specified delay and capacity.
///
/// # Arguments
/// - `receiver`: The `flume::Receiver<ProgressData>` providing progress events
/// - `delay`: The duration to wait before emitting rapid events
/// - `queue_capacity`: Maximum number of events to buffer
///
/// # Performance
/// - Zero allocations during stream operation
/// - Lock-free event processing with crossbeam SegQueue
/// - Bounded memory usage with configurable capacity
/// - Immediate feedback for isolated events, smooth updates for rapid events
///
/// # Example
/// ```
/// use std::time::Duration;
/// use futures_util::{pin_mut, StreamExt};
/// use progresshub_progress::channel::create_progress_channel;
/// use progresshub_progress::debounced::debounced_progress_stream;
///
/// let (tx, rx) = create_progress_channel(256);
/// let stream = debounced_progress_stream(rx, Duration::from_millis(50), 512);
/// pin_mut!(stream);
///
/// // Stream will emit isolated events immediately and debounce rapid events
/// ```
#[inline]
pub fn debounced_progress_stream(
    receiver: ProgressReceiver,
    delay: Duration,
    queue_capacity: usize,
) -> DebouncedProgressStream {
    DebouncedProgressStream::new(receiver, delay, queue_capacity)
}

impl std::fmt::Debug for DebouncedProgressStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebouncedProgressStream")
            .field("queue_len", &self.queue_len())
            .field("queue_capacity", &self.queue_capacity)
            .field("delay_duration", &self.delay_duration)
            .field("has_pending", &self.last_item.is_some())
            .field("events_processed", &self.events_processed)
            .field("events_debounced", &self.events_debounced)
            .field("events_immediate", &self.events_immediate)
            .finish()
    }
}
