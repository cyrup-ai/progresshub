# Crate Structure Specification

## Overview

This document clarifies which crates actually exist in the ProgressHub workspace and their purposes.

## Actual Crate Structure

The following crates exist and are used:

```
progresshub/
├── bandwidth/          # Network bandwidth monitoring
├── client_gix/         # Git backend implementation
├── client_progresshub/ # Backend selector/consolidator
├── client_selector/    # Multi-download orchestrator
├── client_xet/         # CAS/XET backend implementation
├── config/             # Configuration management
├── progress/           # Progress types and channels
└── tui/                # Terminal UI and main binary
```

## About progresshub-core and progresshub-api

**These crates do NOT exist and should NOT be created.**

The TODO.md references these crates, but this is outdated. Here's where their functionality actually lives:

### Types from "progresshub-core"

The types that TODO.md puts in progresshub-core are actually distributed:

1. **OneOrMany<T>** → Lives in `client_selector/src/types.rs`
2. **ProgressHandler trait** → Lives in `progress/src/handler.rs`
3. **Result types** → Lives in `client_progresshub/src/types.rs`
4. **Error types** → Each crate has its own error types

### API from "progresshub-api"

The public API doesn't need its own crate. It's exposed directly from `client_selector`:

```rust
// In client_selector/src/lib.rs
pub use multi_download::download;
pub use types::{OneOrMany, DownloadResult, ModelResult};
```

## Why No Core Crate?

We avoid a "core" or "common" crate because:

1. **Circular dependency risk** - Core crates often create dependency cycles
2. **Unclear ownership** - "Common" code has no clear owner
3. **Kitchen sink problem** - They accumulate unrelated utilities
4. **Compilation time** - Changes trigger full workspace rebuilds

## Where Types Actually Live

| Type | Location | Why There |
|------|----------|-----------|
| `OneOrMany<T>` | `client_selector` | Used for download API input |
| `ProgressData` | `progress` | Core progress type |
| `DownloadResult` | `client_progresshub` | Output of download operations |
| `Backend` enum | `client_progresshub` | Backend selection logic |
| `CasError` | `client_xet` | CAS-specific errors |
| `GitError` | `client_gix` | Git-specific errors |

## Dependency Flow

```
tui
 └─> client_selector
      ├─> client_progresshub
      │    ├─> client_xet
      │    └─> client_gix
      └─> progress
```

Each crate depends only on what it needs. No central "core" crate.

## For Developers

**Follow the existing structure.** Don't create new crates unless there's a clear architectural boundary. Put types where they're used, not in a grab-bag "core" crate.