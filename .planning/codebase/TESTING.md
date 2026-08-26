# Testing Patterns

**Analysis Date:** 2026-08-26

## Test Framework

**Runner:**
- Rust `libtest` via `cargo test` (edition 2021, MSRV `1.86` in `Cargo.toml`; toolchain channel `1.98` in `rust-toolchain.toml`).
- Async tests: `#[tokio::test]` (tokio `full` in `Cargo.toml`).
- Config: `Cargo.toml` `[dev-dependencies]` (`tokio-test = "0.4"`, `tempfile = "3.19"`). No `tests/Cargo.toml`, no rustc test harness override. `tokio-test` is declared but unused in source.
- CI (`.github/workflows/ci.yml`): `cargo build --release` then `cargo test --all-features` (release binary required for MCP protocol tests). Separate jobs: `cargo fmt -- --check`, `cargo clippy --all-features -- -D warnings`.
- Pre-commit (`.pre-commit-config.yaml`): `cargo test` on every commit.

**Assertion Library:**
- Standard library macros: `assert!`, `assert_eq!`, optional message args (`assert!(rate >= 10.0, "Error rate should exceed threshold")` in `tests/test_alerts.rs`). No `pretty_assertions`, `claim`, or `assert_matches`.

**Run Commands:**
```bash
just test              # cargo test && cargo clippy (local verification; Justfile)
cargo test             # Run all lib + bin + tests/*.rs tests
cargo test -- --nocapture   # just verbose-test
cargo test test_error_rate_threshold_alert   # Single test (CONTRIBUTING.md)
cargo test --test test_alerts                # One integration crate
just watch-test        # cargo watch -x test (requires cargo-watch)
just ci                # fmt check then test (Justfile: `ci: fmt test`)
just lint              # cargo clippy --all-features -- -D warnings
just fmt               # cargo fmt -- --check
```

Watch mode is `just watch-test` (`cargo watch -x test`), not `cargo test --watch`. Coverage is not wired (no tarpaulin/llvm-cov recipe).

**MCP protocol quirk** (`tests/test_mcp_protocol.rs`, `AGENTS.md`, CI comment):

```bash
cargo build --release
cargo test --test test_mcp_protocol
```

Tests spawn `./target/release/logpilot` (`Command::new("./target/release/logpilot")` in `tests/test_mcp_protocol.rs`). Debug `cargo test` without a prior release build fails to start the server. Manual JSON-RPC checks: `docs/MCP_TESTING.md`.

## Test File Organization

**Location:**
- Co-located unit tests: `#[cfg(test)] mod tests` at the bottom of the source file (43 `#[cfg(test)]` sites under `src/`, e.g. `src/models/severity.rs`, `src/buffer/ring.rs`, `src/pipeline/parser.rs`, `src/mcp/protocol.rs`).
- Integration crates: `tests/test_alerts.rs`, `tests/test_filter.rs`, `tests/test_mcp_protocol.rs`, `tests/test_pipeline_integration.rs` (each is its own Cargo test crate; they `use logpilot::...`).
- `tests/integration/test_analyzer.rs` and `tests/integration/test_capture.rs` are **not** auto-discovered: Cargo only builds `tests/*.rs` as integration tests; a `tests/integration/` directory is compiled only if referenced from `tests/integration.rs` or `tests/integration/main.rs` (neither exists). Treat them as intended/manual fixtures unless wired.
- Helpers: `tests/fixtures/mock_tmux.sh`. `CONTRIBUTING.md` also mentions `tests/contract/` and `benches/`; those directories are not present.

**Naming:**
- Files: `test_<area>.rs` for integration crates; `test_<area>.rs` under `tests/integration/`.
- Functions: `test_<behavior>` (`test_mcp_initialize`, `test_ring_buffer_capacity_eviction`).
- Helpers: `create_test_entry`, `make_entry`, `test_entry`, `TestFixture`.

**Structure:**
```
src/**/*.rs              # #[cfg(test)] mod tests { ... } unit/async unit
tests/test_*.rs          # cargo integration tests (library crate)
tests/integration/       # not auto-run; analyzer + capture scenarios
tests/fixtures/          # mock_tmux.sh
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(content: &str, severity: Severity) -> LogEntry {
        LogEntry {
            id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
            sequence: 1,
            timestamp: Utc::now(),
            severity,
            service: None,
            raw_content: content.to_string(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    #[test]
    fn test_ring_buffer_push_and_get() {
        let mut buffer = RingBuffer::new(10, 30);
        let entry = create_test_entry("test", Severity::Info);
        buffer.push(entry.clone());
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.newest().unwrap().raw_content, "test");
    }
}
```

Pattern from `src/buffer/ring.rs`. Integration example from `tests/test_alerts.rs`:

```rust
/// Test: Error rate > threshold triggers alert (T078)
#[tokio::test]
async fn test_error_rate_threshold_alert() {
    let (evaluator, _alert_rx) = AlertEvaluator::new();
    let calc = ErrorRateCalculator::new();
    for _ in 0..15 {
        calc.record_error(Some("test-service"));
    }
    let rate = calc.calculate_rate(Some("test-service"));
    assert!(rate >= 10.0, "Error rate should exceed threshold");
    let alert = evaluator.check_error_rate(rate, Some("test-service"));
    assert!(alert.is_some(), "Alert should be triggered when rate exceeds threshold");
}
```

**Patterns:**
- Setup: local helper constructors (`create_test_entry` in `src/buffer/manager.rs`, `src/mcp/data_store.rs`; `TestFixture` in `tests/integration/test_analyzer.rs`); `PersistenceStore::new_in_memory()` / `BufferManager::new_in_memory` / shared-memory SQLite URI in `tests/test_pipeline_integration.rs`.
- Teardown: MCP child `let _ = child.kill()` (`tests/test_mcp_protocol.rs`); `session.cleanup().await` in `tests/integration/test_capture.rs`. No `Drop` guards in unit tests. No `#[should_panic]` / `#[ignore]`.
- Assertion: `assert_eq!` for values; `assert!(x.is_ok())` / `is_err()` for validation (`src/capture/tmux.rs`); table-driven `for (content, expected) in cases` (`src/pipeline/parser.rs`).

## Mocking

**Framework:** None (`mockall` / `mockito` not in `Cargo.toml`). Isolation is in-memory fakes, test-only constructors, and a shell tmux stub.

**Patterns:**
```rust
// In-memory persistence instead of a real DB file
let store = PersistenceStore::new_in_memory().await.unwrap();

// Shared in-memory SQLite so pooled connections see the same DB
let manager = BufferManager::with_persistence(
    "file:test_buffer_only_persists_error_and_fatal?mode=memory&cache=shared",
    1000,
    60,
    Severity::Error,
)
.await
.expect("should create shared in-memory persistence store");

// Test-only MCP server constructor
#[cfg(test)]
fn with_data_store(data_store: SessionDataStore) -> Self { /* src/mcp/server.rs */ }

// Test-only model helpers
#[cfg(test)]
pub fn with_incident(mut self, incident_id: Uuid) -> Self { /* src/models/alert.rs */ }
```

Shell stub: `tests/fixtures/mock_tmux.sh` implements `new-session`, `send-keys`, `pipe-pane`, `list-sessions`, `list-panes`. `tests/integration/test_capture.rs` uses `tempfile::NamedTempFile` plus a `MockTmuxSession` helper.

**What to Mock:**
- SQLite: in-memory / shared-cache URI, not on-disk files (`src/buffer/persistence.rs` tests, `tests/test_pipeline_integration.rs`).
- MCP session state: `SessionDataStore::new()` (`src/mcp/data_store.rs`).
- tmux: skip or stub when `!TmuxCommand::is_installed()` (`tests/test_filter.rs` `test_list_panes_all_windows`); mock script for capture scenarios.

**What NOT to Mock:**
- Domain types (`LogEntry`, `Pattern`, `Alert`) — constructed directly.
- MCP protocol E2E — spawn the real `./target/release/logpilot mcp-server` process (`tests/test_mcp_protocol.rs`).
- Regex/parser behavior — feed real log strings (`src/pipeline/parser.rs`, `tests/test_filter.rs`).

## Fixtures and Factories

**Test Data:**
```rust
// tests/integration/test_analyzer.rs
pub struct TestFixture {
    pub session_id: Uuid,
    pub pane_id: Uuid,
    sequence: u64,
}

impl TestFixture {
    pub fn new() -> Self { /* Session::new + Pane::new */ }
    pub fn create_log_entry(&mut self, content: &str, severity: Severity) -> LogEntry { /* ... */ }
    pub fn create_error_entry(&mut self, content: &str) -> LogEntry { /* ... */ }
}

// tests/test_pipeline_integration.rs
fn make_entry(pane_id: Uuid, seq: u64, content: &str) -> LogEntry { /* struct literal */ }
fn make_entry_with_severity(pane_id: Uuid, seq: u64, content: &str, sev: Severity) -> LogEntry { /* ... */ }
```

Many unit tests duplicate a small `create_test_entry` struct literal rather than sharing a crate-wide factory (`src/buffer/ring.rs`, `src/buffer/persistence.rs`, `src/analyzer/alerts.rs`).

**Location:**
- Inline helpers next to `#[cfg(test)]` modules.
- `tests/integration/test_analyzer.rs` `TestFixture`.
- `tests/fixtures/mock_tmux.sh` for tmux CLI simulation.
- Fixed timestamps in parser tests (`"2024-01-01T00:00:00Z"` in `src/pipeline/parser.rs`) for determinism.

## Coverage

**Requirements:** None enforced (no `cargo-llvm-cov`, tarpaulin, or coverage job in `.github/workflows/ci.yml` or `Justfile`).

**View Coverage:**
```bash
# Not configured in-repo. If added later, typical:
cargo tarpaulin --out Html
# or
cargo llvm-cov --html
```

`just audit` / `just tree` exist for security/deps, not coverage. `autoresearch.md` mentions MCP *resource* coverage, not line coverage.

## Test Types

**Unit Tests:**
- Co-located `#[cfg(test)]` covering pure logic: severity parsing (`src/models/severity.rs`), ring eviction (`src/buffer/ring.rs`), JSON-RPC serialization (`src/mcp/protocol.rs`), tmux target/path injection guards (`src/capture/tmux.rs`), filter matchers (`src/cli/filter.rs`), metrics (`src/observability.rs`).
- Sync `#[test]` for CPU-only code; `#[tokio::test]` when the type is async (`src/buffer/manager.rs`, `src/mcp/data_store.rs`, `src/analyzer/incidents.rs`).

**Integration Tests:**
- `tests/test_alerts.rs`: alert threshold, dedup, ack (tokio).
- `tests/test_filter.rs`: severity/pattern matching; optional live tmux `list_panes` (skips if tmux missing).
- `tests/test_pipeline_integration.rs`: capture → buffer → SQLite persist filter → MCP `SessionDataStore` / `ResourceHandler` → dedup.
- Library is the public surface (`use logpilot::...`).

**E2E Tests:**
- Process-level: `tests/test_mcp_protocol.rs` spawns release binary, writes JSON-RPC on stdin, asserts stdout (`initialize`, `ping`, `resources/list`, unknown method `-32601`). Closest thing to E2E.
- Intended tmux E2E in `tests/integration/test_capture.rs` (`MockTmuxSession`, attach, &lt;2s latency, concurrent sessions) is not in the default Cargo test graph (see organization).
- Manual: `docs/MCP_TESTING.md` (`echo '{"jsonrpc":...}' | ./target/release/logpilot mcp-server`).

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_store_and_query() {
    let store = PersistenceStore::new_in_memory().await.unwrap();
    let stored = store.store_entry(&entry, Severity::Error).await.unwrap();
    assert!(stored);
    let entries = store
        .query_entries(
            Utc::now() - chrono::Duration::minutes(1),
            Utc::now() + chrono::Duration::minutes(1),
            None,
        )
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
}
```

From `src/buffer/persistence.rs`. MCP E2E uses `std::thread::sleep` plus `std::process::Command`, not tokio (`tests/test_mcp_protocol.rs`).

**Error Testing:**
```rust
#[test]
fn test_validate_target_invalid() {
    assert!(validate_target("session;rm -rf /").is_err());
    assert!(validate_target("session|cat /etc/passwd").is_err());
}

#[test]
fn test_mcp_unknown_method() {
    // spawn ./target/release/logpilot mcp-server, send unknown/method
    assert_eq!(error["code"], -32601, "Error code should be Method not found (-32601)");
}

#[tokio::test]
async fn test_list_panes_all_windows() {
    if !TmuxCommand::is_installed() {
        println!("Skipping: tmux not installed");
        return;
    }
    let panes = TmuxCommand::list_panes(&session).await;
    assert!(panes.is_ok(), "list_panes should succeed for session {}", session);
}
```

From `src/capture/tmux.rs`, `tests/test_mcp_protocol.rs`, `tests/test_filter.rs`. Failures use `assert!(result.is_err())` or JSON-RPC error objects; no `#[should_panic]`. Soft-skip when tmux is absent rather than `#[ignore]`.

---

*Testing analysis: 2026-08-26*
