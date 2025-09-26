# ProgressHub Specifications

This directory contains detailed specifications that clarify potentially confusing aspects of the ProgressHub architecture.

## Documents

1. **[01-channel-architecture.md](01-channel-architecture.md)**
   - Clarifies "event bus" vs "direct channels" confusion
   - Shows the actual channel implementation
   - Explains why we use direct channels, not a global bus

2. **[02-crate-structure.md](02-crate-structure.md)**
   - Lists the actual crates in the workspace
   - Explains why `progresshub-core` and `progresshub-api` don't exist
   - Shows where types actually live

3. **[03-async-api-design.md](03-async-api-design.md)**
   - Resolves the "sync vs async" API contradiction
   - Explains why we return `impl Future` from a sync function
   - Shows how this follows CONVENTIONS.md

4. **[04-event-bootstrap.md](04-event-bootstrap.md)**
   - Explains how the TUI starts without tickers
   - Shows what triggers the first render
   - Details the event-driven render loop

5. **[05-channel-topology.md](05-channel-topology.md)**
   - Maps out all channels in the system
   - Shows there are only TWO channels total (per application)
   - Explains channel lifecycle and ownership

6. **[06-api-design.md](06-api-design.md)**
   - Explains the new channel-based API design
   - Shows how ChannelProgressHandler works
   - Details error handling semantics

7. **[07-design-decisions.md](07-design-decisions.md)**
   - Captures key design decisions and rationale
   - Explains why we use flume, not mpsc
   - Documents channel lifecycle choices

## When to Read These

- **Before implementing** - Read relevant specs first
- **When confused** - If ARCHITECTURE.md and TODO.md conflict, check here
- **During review** - Ensure implementation matches specifications

## Key Principles

1. **No global state** - Everything is passed explicitly
2. **No tickers** - Pure event-driven architecture
3. **Direct channels** - No event bus abstraction
4. **Clear ownership** - Each component owns its part
5. **Simple topology** - Just two channels for the entire system