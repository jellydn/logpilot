# Coding Conventions

**Analysis Date:** 2026-08-26

## Naming Patterns

**Files:**
- Rust modules use `snake_case.rs` matching the module name (`src/analyzer/alerts.rs`, `src/models/log_entry.rs`, `src/pipeline/formats.rs`).
- Domain code lives in a directory with a `mod.rs` barrel (`src/analyzer/`, `src/buffer/`, `src/capture/`, `src/cli/`, `src/mcp/`, `src/models/`, `src/pipeline/`).
- Crate roots are `src/lib.rs` (library) and `src/main.rs` (binary named `logpilot` in `Cargo.toml`).
- Integration tests are `tests/test_*.rs` (`tests/test_alerts.rs`, `tests/test_mcp_protocol.rs`). Intended extra tests sit under `tests/integration/` (`tests/integration/test_analyzer.rs`, `tests/integration/test_capture.rs`); fixtures are `tests/fixtures/mock_tmux.sh`.
- No `rustfmt.toml` or `clippy.toml` in the repo; toolchain pins tools in `rust-toolchain.toml`.

**Functions:**
- `snake_case` throughout (`validate_target` in `src/capture/tmux.rs`, `process_entry` in `src/analyzer/mod.rs`, `store_entry` in `src/buffer/persistence.rs`).
- Constructors are `new` / `new_with_*` (`LogEntry::new`, `LogEntry::new_with_severity` in `src/models/log_entry.rs`).
- Fluent builders are `with_*` and return `Self` (`with_severity`, `with_service`, `with_parsed_field` in `src/models/log_entry.rs`).
- Predicates are `is_*` (`is_severe`, `is_empty`, `is_active`).
- CLI command entry points are async `handle` in `src/cli/filter.rs`, `src/cli/ask.rs`, `src/cli/summarize.rs`, `src/cli/status.rs`, `src/cli/mcp.rs`; watch is `run` in `src/cli/watch.rs`.
- Tests are `test_<behavior>` (`test_severity_from_str` in `src/models/severity.rs`).

**Variables:**
- `snake_case` locals and fields (`buffer_minutes`, `raw_content`, `window_count`).
- Compiled regexes are `SCREAMING_SNAKE` `static` `Lazy` values (`TIMESTAMP_ISO8601_RE`, `SEVERITY_RE` in `src/pipeline/parser.rs`; `VALID_TARGET_RE` in `src/capture/tmux.rs`).
- CLI option structs use clap field names that match flags (`level`, `follow`, `pattern` in `src/cli/filter.rs`).

**Types:**
- `PascalCase` structs and enums (`LogEntry`, `Severity`, `AlertEvaluator`, `BufferManager`, `TmuxCommand`).
- Clap argument structs are `*Args` (`FilterArgs`, `AskArgs`, `McpArgs`) or `*Options` (`WatchOptions` in `src/cli/watch.rs`).
- Domain error is `LogPilotError` with crate alias `Result<T>` in `src/error.rs`.
- Enums use `PascalCase` variants (`Severity::Error`, `AlertType::RecurringError`); serde `rename_all` is `"UPPERCASE"` for `Severity` (`src/models/severity.rs`) and `"PascalCase"` for alert enums (`src/models/alert.rs`).

## Code Style

**Formatting:**
- Tool: `rustfmt` (component in `rust-toolchain.toml`; `just fmt` / CI job `fmt` run `cargo fmt -- --check`; `just fix-fmt` runs `cargo fmt`).
- No `rustfmt.toml` / `.rustfmt.toml` — rustfmt defaults apply (typically max width 100, edition inherited from `Cargo.toml` `edition = "2021"`).
- Pre-commit hook in `.pre-commit-config.yaml` runs `cargo fmt -- --check`.
- `CONTRIBUTING.md` “Code Style”: follow Rust naming, use `cargo fmt`, document public APIs with `///`.

**Linting:**
- Tool: `clippy` (component in `rust-toolchain.toml`).
- No `clippy.toml`. CI `clippy` job and `just lint` run `cargo clippy --all-features -- -D warnings` (`.github/workflows/ci.yml`).
- `just test` runs `cargo clippy` without `-D warnings`; pre-commit `clippy` hook is also `cargo clippy` without deny-warnings.
- Module-level `#![allow(dead_code)]` is used for APIs not yet wired to CLI (`src/analyzer/mod.rs`, `src/buffer/mod.rs`, `src/capture/mod.rs`, `src/mcp/mod.rs`, `src/pipeline/mod.rs`).
- Spot allows: `#[allow(dead_code)]` on unused helpers (`src/models/log_entry.rs`, `src/models/alert.rs`); `#[allow(clippy::wrong_self_convention)]` in `src/buffer/persistence.rs`.
- MSRV: `rust-version = "1.86"` in `Cargo.toml`; `rust-toolchain.toml` pins channel `1.98`. CI uses `dtolnay/rust-toolchain@stable`.

## Import Organization

**Order:**
1. Crate / parent modules: `crate::...` or `super::...` first (`src/buffer/manager.rs`, `src/models/log_entry.rs`, `src/cli/filter.rs`).
2. Third-party crates: `chrono`, `serde`, `tokio`, `uuid`, `tracing`, `clap`, `sqlx`, `regex`, `once_cell`, etc.
3. `std` (`std::collections::HashMap`, `std::path::PathBuf`) mixed after crate imports; rustfmt `group_imports` is default Mix (no reorder config).

Typical file (`src/capture/pane.rs`): `crate::{capture, error, models}` → `std` → `tokio` → `tracing` → `uuid`.

**Path Aliases:**
- None (no `[paths]`, no `extern crate` aliases). Use `crate::` from library modules and `super::` within a module tree (`src/models/log_entry.rs` uses `super::severity::Severity`).
- Binary `src/main.rs` redeclares `mod analyzer;` … `mod pipeline;` instead of depending on the `logpilot` library; integration tests import `logpilot::...`.

## Error Handling

**Patterns:**
- Library/domain: `thiserror` enum `LogPilotError` with `#[from]` for `std::io::Error` and `sqlx::Error`, plus contextual variants `Tmux`, `DatabaseOp`, `Config`, `SessionNotFound` (`src/error.rs`). Helpers: `LogPilotError::tmux`, `::config`, `::db_op`.
- Crate `Result<T> = std::result::Result<T, LogPilotError>` re-exported from `src/lib.rs`.
- CLI-facing commands often return `anyhow::Result<()>` (`src/cli/ask.rs`, `src/cli/summarize.rs`, `src/cli/status.rs`, `src/cli/mcp.rs`) while filter/watch use `crate::error::Result`.
- Convert with `.map_err(|e| LogPilotError::db_op(...))` (`src/buffer/persistence.rs`) or `.map_err(LogPilotError::Io)` (`src/capture/tmux.rs`, `src/cli/filter.rs`).
- `?` is the default propagation style; recoverable CLI paths print `eprintln!("Error: {}", e)` in `src/main.rs` instead of aborting the process for filter/summarize/ask.
- `CONTRIBUTING.md` says handle errors explicitly — no `unwrap` in production. Production still uses `unwrap`/`expect` for static regex compilation (`src/pipeline/parser.rs`, `src/capture/tmux.rs`) and `unwrap_or` / `unwrap_or_else` for fallbacks (`src/lib.rs` config paths, `src/cli/watch.rs` persistence fallback).
- Tests use `.unwrap()` / `.expect("should ...")` freely (`src/mcp/protocol.rs`, `src/buffer/persistence.rs`).

## Logging

**Framework:** `tracing` + `tracing-subscriber` (`Cargo.toml`; `tracing_subscriber::fmt::init()` in `src/main.rs`).

**Patterns:**
- Levels: `error`, `warn`, `info`, `debug` imported per module (`src/capture/pane.rs`, `src/mcp/server.rs`, `src/cli/watch.rs`). `RUST_LOG` documented in `CONTRIBUTING.md` (`error` / `warn` / `info` default / `debug` / `trace`).
- Structured fields in `src/observability.rs`: `info!(entries_captured = ..., "Captured 1000 entries")`, `debug!(event = "capture", session = ..., pane = ..., bytes = ...)`.
- Operational CLI/MCP messages also use `eprintln!("[LogPilot] ...")` (`src/cli/mcp.rs`) and `info!`/`warn!` for watch lifecycle (`src/cli/watch.rs`).
- Persistence init failure is `warn!` then in-memory fallback (`src/cli/watch.rs`).

## Comments

**When to Comment:**
- Every domain module starts with `//!` crate/module docs describing purpose (`src/buffer/ring.rs`, `src/observability.rs`, `src/mcp/resources.rs`).
- `///` on public constructors, methods, and security-sensitive behavior (`src/capture/tmux.rs` pipe-pane SECURITY note; `src/models/log_entry.rs` `signature`).
- Inline `//` for pipeline steps (`src/analyzer/mod.rs` “Step 1: Parse…”) and non-obvious constraints (SQLite shared in-memory URI comment in `tests/test_pipeline_integration.rs`).
- Models often omit `///` on fields (`src/models/log_entry.rs`, `src/models/alert.rs`) — docs live on methods and modules, not every field.

**JSDoc/TSDoc:**
- N/A (Rust). Public API docs use rustdoc `///` / `//!` as specified in `CONTRIBUTING.md`. No rustdoc `Cargo.toml` `[package.metadata.docs.rs]` extras; `just docs` runs `cargo doc --open`.

## Function Design

**Size:** `CONTRIBUTING.md` asks for small, focused functions. Typical impl methods are short (buffer ops in `src/buffer/ring.rs`). Larger functions are command handlers (`src/cli/filter.rs` `handle`) and MCP `handle_request` match dispatch (`src/mcp/server.rs`).

**Parameters:**
- CLI: clap `*Args` / `WatchOptions` structs rather than long positional lists.
- Domain constructors take `impl Into<String>` for owned strings (`LogEntry::new`, `Alert::new`).
- Optional data is `Option<T>` (`pane: Option<String>`, `service: Option<String>`).
- IDs are `uuid::Uuid`; time is `chrono::DateTime<Utc>`.
- Mutation via `&mut self` on parsers (`LogParser::parse(&mut entry)` in `src/pipeline/parser.rs`); shared state is `Arc<RwLock<_>>` (`src/analyzer/mod.rs`).

**Return Values:**
- Fallible: `Result<T>` / `anyhow::Result<T>`.
- Absence: `Option<T>` (`newest()`, `check_recurring_error`).
- Async I/O and locks: `async fn` with tokio (`BufferManager`, `PersistenceStore`, CLI handlers).
- `Default` is implemented alongside `new()` (`Analyzer`, `RingBuffer`, `Pipeline`, `Config`).

## Module Design

**Exports:**
- `src/lib.rs` `pub mod` for each domain; re-exports `Analyzer`, `LogPilotError`, `Result`, `Pipeline`.
- Domain `mod.rs` files `pub mod` children and `pub use` the public types (`src/models/mod.rs` re-exports `LogEntry`, `Severity`, `Alert`, …; `src/analyzer/mod.rs` re-exports `AlertEvaluator`, `ErrorRateCalculator`; `src/mcp/mod.rs` re-exports `McpServer`).
- Binary `src/main.rs` has a parallel private `mod` tree (does not `use logpilot as _`); integration tests consume the library crate.
- `src/mcp/rmcp_server.rs` exists but is commented out of `src/mcp/mod.rs` (Rust 1.86 / `rmcp` crate disabled in `Cargo.toml`).

**Barrel Files:**
- Used: each domain `mod.rs` is the barrel. `src/cli/mod.rs` only `pub mod`s children (no `pub use`). Models barrel is the fullest `pub use` surface (`src/models/mod.rs`).

---

*Convention analysis: 2026-08-26*
