# Async API Design Specification

## Overview

This document clarifies the apparent contradiction between "synchronous API" and returning a `Future`.

## The Design: Sync Function Returning Future

The API is **synchronous** in that the function itself is not `async`:

```rust
// This is what we have - a sync function returning a Future
pub fn download<P>(
    models: impl Into<OneOrMany<String>>,
    progress: P
) -> impl Future<Output = Result<DownloadResult>>
where
    P: ProgressHandler + Send + Sync + 'static
{
    // Function body runs synchronously
    let models = models.into();
    let orchestrator = MultiDownloadOrchestrator::new();
    
    // Returns a Future without being async
    orchestrator.download_async(models, progress)
}
```

## Why Not `async fn`?

We deliberately avoid `async fn` for the public API:

```rust
// We DON'T do this
pub async fn download<P>(...) -> Result<DownloadResult> { ... }
```

Because:
1. **Trait compatibility** - Can't use `async fn` in traits without async-trait
2. **Hidden boxing** - async fn secretly boxes the Future
3. **Cleaner API** - User sees exactly what they get: a Future

## How It Works Internally

The pattern is:

```rust
impl MultiDownloadOrchestrator {
    // Public API - sync function returning Future
    pub fn download(&self, models: Vec<String>) -> impl Future<Output = Result<...>> {
        // Create the async task
        let task = self.download_internal(models);
        
        // Return it without awaiting
        task
    }
    
    // Internal implementation - actual async function
    async fn download_internal(&self, models: Vec<String>) -> Result<...> {
        // Async work happens here
        for model in models {
            self.download_one(model).await?;
        }
        Ok(...)
    }
}
```

## What About CONVENTIONS.md?

CONVENTIONS.md says "Provide synchronous interfaces with `.await()` called internally" - this applies to **callback-based APIs**, not Future-returning APIs.

Example of what CONVENTIONS means:

```rust
// BAD - exposing async complexity in callbacks
trait ProgressHandler {
    async fn handle(&self, progress: Progress);  // NO!
}

// GOOD - hiding async inside
trait ProgressHandler {
    fn handle(&self, progress: Progress);  // Sync interface
}

impl ProgressHandler for MyHandler {
    fn handle(&self, progress: Progress) {
        // If we need async, we spawn it internally
        tokio::spawn(async move {
            do_async_thing(progress).await;
        });
    }
}
```

## Usage Pattern

The user experience is clean:

```rust
// In an async context
let result = download(vec!["model1", "model2"], handler).await?;

// Or spawn it
let handle = tokio::spawn(download(models, handler));
let result = handle.await??;

// Or block on it (in non-async context)
let result = tokio::runtime::Handle::current()
    .block_on(download(models, handler))?;
```

## Key Points

1. **Sync function** - The download() function itself is not async
2. **Returns Future** - It returns a Future that must be awaited
3. **No hidden boxing** - Return type is explicit
4. **Clean API** - User knows exactly what they're getting
5. **Follows CONVENTIONS** - We don't expose async in traits/callbacks

## Why This Pattern?

This pattern gives us:
- **Flexibility** - User chooses how to execute the Future
- **Clarity** - No hidden allocations or boxing
- **Compatibility** - Works with any async runtime
- **Simplicity** - One function, one return type