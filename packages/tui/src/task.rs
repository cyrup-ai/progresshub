//! Task utilities for returning futures and streams from sync methods
//!
//! This module follows the pattern from cyrup-ai/async_task to provide
//! wrappers for futures and streams without using async fn or async_trait.

use futures::stream::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Wrapper for a concrete future type that can be returned from sync methods
pub struct AsyncTask<F> {
    // The wrapped future
    future: F,
}

impl<F> AsyncTask<F> {
    /// Create a new async task that wraps the given future
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

// Implement Future for AsyncTask by forwarding to the wrapped future
// This requires that F implements Unpin, since we're using Pin::new
impl<F: Future + Unpin> Future for AsyncTask<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx)
    }
}

/// Wrapper for a concrete stream type that can be returned from sync methods
pub struct AsyncStream<S> {
    // The wrapped stream
    stream: S,
}

impl<S> AsyncStream<S> {
    /// Create a new async stream that wraps the given stream
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

// Implement Stream for AsyncStream by forwarding to the wrapped stream
// This requires that S implements Unpin, since we're using Pin::new
impl<S: Stream + Unpin> Stream for AsyncStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}
