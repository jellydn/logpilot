# Coding Conventions

**Analysis Date:** 2025-04-10

## Naming Patterns

**Files:**
- Module files: `snake_case.rs` (`log_entry.rs`, `session.rs`)
- Test files: Co-located, no separate naming convention

**Functions:**
- snake_case: `add_entry()`, `get_cluster()`, `run()`
- Async functions prefixed with `async`: `async fn handle()`
- Constructor pattern: `new()` for primary constructor

**Variables:**
- snake_case: `session_name`, `buffer_size`
- Type parameters: PascalCase `T`, `K`, `V`

**Types:**
- Structs: PascalCase `LogEntry`, `McpServer`
- Enums: PascalCase `LogPilotError`, `Severity`
- Traits: PascalCase (often ending in -able/-ible if behavior-based)
- Type aliases: PascalCase `Result<T>`

## Code Style

**Formatting:**
- Tool: `rustfmt` (standard Rust formatter)
- Config: Default settings (no custom rustfmt.toml observed)
- CI check: `cargo fmt -- --check`

**Linting:**
- Tool: `clippy` (standard Rust linter)
- Strictness: `-D warnings` (deny all warnings in CI)
- CI check: `cargo clippy --all-features -- -D warnings`

## Import Organization

**Order:**
1. Standard library (`std::`, `core::`)
2. External crates (alphabetically: `chrono::`, `serde::`, `tokio::`)
3. Internal modules (`crate::`)

**Path Aliases:**
- None observed - full paths used

**Example from `src/lib.rs`:**
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
```

## Error Handling

**Patterns:**
- Custom error enum: `LogPilotError` with `#[derive(Error)]`
- `thiserror` for boilerplate reduction
- `anyhow` for error propagation
- `Result<T>` type alias for consistency
- `?` operator with `#[from]` conversions
- Constructor methods: `LogPilotError::tmux()`, `.parse()`, `.config()`

## Logging

**Framework:** `tracing` with `tracing-subscriber`

**Patterns:**
- Events: `tracing::info!()`, `tracing::warn!()`, `tracing::error!()`, `tracing::debug!()`
- Initialize in main: `tracing_subscriber::fmt::init()`
- Contextual logging with spans (where appropriate)

**Example:**
```rust
use tracing::{info, warn};
info!("Starting MCP server");
warn!("Buffer full for pane: {}", pane_id);
```

## Comments

**When to Comment:**
- Module-level documentation: `//!` doc comments
- Public API documentation: `///` doc comments
- Complex logic explanations
- TODO markers (observed in codebase)

**Documentation:**
- Standard Rust doc comments (`///` and `//!`)
- Public items should have doc comments
- Examples in doc comments where helpful

## Function Design

**Size:** No strict limit, but functions tend to be focused (20-50 lines typical)

**Parameters:**
- Prefer struct-based options for complex parameters
- Use references for large types (`&str`, `&[T]`)
- Async functions for I/O operations

**Return Values:**
- `Result<T>` for fallible operations
- `Option<T>` for nullable returns
- Unit `()` for side-effect-only functions

## Module Design

**Exports:**
- `pub mod` for public submodules
- `pub use` for re-exports (e.g., `pub use error::{LogPilotError, Result}`)
- Prelude pattern: `src/models/prelude.rs` for common imports

**Visibility:**
- `pub` for public API
- Default (private) for internal implementation
- `pub(crate)` for crate-internal visibility where needed

---

*Convention analysis: 2025-04-10*
