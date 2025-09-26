# Event Bootstrap Specification

## Overview

This document explains how the event-driven TUI starts up without any tickers or timers.

## The Bootstrap Sequence

### 1. Initial State Creation

```rust
// In main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Create channels
    let orchestrator = MultiDownloadOrchestrator::new();
    let progress_rx = orchestrator.take_receiver();
    
    // Create app with initial state
    let mut app = App::new(progress_rx);
    
    // Start downloads (this sends first events)
    let download_handle = tokio::spawn(
        orchestrator.download(models, progress_handler)
    );
    
    // Initialize terminal
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    
    // FIRST RENDER - triggered manually
    terminal.draw(|f| app.render(f))?;
    
    // Now enter event loop
    app.run_event_loop(terminal).await?;
}
```

### 2. Event Loop Structure

```rust
impl App {
    async fn run_event_loop(&mut self, mut terminal: Terminal<B>) -> Result<()> {
        let mut event_stream = EventStream::new();
        
        loop {
            tokio::select! {
                // User input events
                Some(Ok(event)) = event_stream.next() => {
                    match event {
                        Event::Key(key) => {
                            self.handle_key(key);
                            // Key press triggers render
                            terminal.draw(|f| self.render(f))?;
                        }
                        Event::Resize(_, _) => {
                            // Resize triggers render
                            terminal.draw(|f| self.render(f))?;
                        }
                        _ => {}
                    }
                }
                
                // Progress events from downloads
                Some(progress) = self.progress_rx.recv() => {
                    self.handle_progress(progress);
                    // Progress triggers render
                    terminal.draw(|f| self.render(f))?;
                }
                
                // Bandwidth events
                Some(bandwidth) = self.bandwidth_rx.recv() => {
                    self.handle_bandwidth(bandwidth);
                    // Bandwidth update triggers render
                    terminal.draw(|f| self.render(f))?;
                }
            }
            
            if self.should_quit {
                break;
            }
        }
        
        Ok(())
    }
}
```

## What Triggers Renders?

**ONLY actual events trigger renders:**

1. **User Input** - Key press, mouse click, terminal resize
2. **Progress Update** - Download progress from workers
3. **Bandwidth Update** - Network stats change
4. **Initial Render** - One manual draw before loop starts

## No Idle Rendering

When nothing is happening:
- No downloads running → no progress events
- No user input → no key events  
- No network activity → no bandwidth events
- **Result: ZERO renders, ZERO CPU usage**

## Common Patterns

### Starting a Download

```rust
// User presses 'd' to start download
Key(KeyCode::Char('d')) => {
    self.start_download();           // This will cause progress events
    terminal.draw(|f| self.render(f))?;  // Immediate feedback
}
```

### Progress Arrives

```rust
// Download worker sends progress
progress_tx.send(ProgressData {
    model: "llama-2-7b",
    bytes: 1_000_000,
    total: 10_000_000,
}).await?;

// TUI receives it and renders
Some(progress) = self.progress_rx.recv() => {
    self.update_progress(progress);  // Update state
    terminal.draw(|f| self.render(f))?;  // Render new state
}
```

## Why No Render Event?

We don't need a separate `Event::Render` because:
1. **Every event triggers its own render** - Direct cause and effect
2. **No batching needed** - Events are already throttled at source
3. **Simpler code** - Render is always in response to something

## What About Animations?

If you need animations (e.g., spinning progress):
1. **Don't use a ticker!**
2. Use progress events as animation frames
3. Or spawn a dedicated animation task that sends events only when needed

```rust
// Only spawn animator when download is active
if self.downloads_active() {
    tokio::spawn(async move {
        while downloading {
            animation_tx.send(AnimationFrame::Next).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}
```

## Key Principles

1. **Event → State Change → Render** - Always in that order
2. **No renders without events** - Save CPU and battery
3. **Manual first render** - Bootstrap the display
4. **Events drive everything** - The UI reacts, never polls