# Technical Concerns - LogPilot

## Critical Issues

### 1. Runtime Regex Unwrap on Every Validation Call
- **File**: `src/capture/tmux.rs:13`
- **Issue**: Regex pattern compiled with `.unwrap()` on every validation call instead of using `once_cell::Lazy` like the rest of the codebase.
- **Risk**: Panic on invalid regex pattern; unnecessary recompilation overhead per call.
- **Fix**: Use `static VALID_TARGET_RE: Lazy<Regex> = Lazy::new(|| Regex::new(...).unwrap());`

---

## High Priority

### 2. TOCTOU Race Condition in Buffer Manager
- **File**: `src/buffer/manager.rs:59-92`
- **Issue**: Double lock acquire with an async call between the check and the mutation. A buffer could be created by another task between the check and the write, leading to lost or duplicated buffer data.
- **Risk**: Data loss or incorrect state under concurrent load.
- **Fix**: Hold the write lock across the entire check-then-create sequence, or use entry-based HashMap APIs.

### 3. Unbounded HashMap Growth in Deduplicator
- **File**: `src/pipeline/dedup.rs:27`
- **Issue**: Signature cache (`HashMap<u64, Instant>`) grows indefinitely. Long-running servers (days/weeks) will accumulate millions of entries.
- **Risk**: OOM crash on long-running deployments.
- **Fix**: Implement LRU eviction or periodic TTL-based cleanup (e.g., remove entries older than the dedup window).

### 4. Global Mutable Singleton Without Cleanup
- **File**: `src/mcp/data_store.rs`
- **Issue**: Session data accumulates in a global `Arc<DashMap>` with no eviction or TTL mechanism. Dead/closed tmux sessions are never removed.
- **Risk**: Memory leak over time; stale data served to AI clients.
- **Fix**: Add session cleanup on tmux session close events, or a periodic background sweep.

---

## Medium Priority

### 5. Unwrap Calls in Test Serialization Code
- **File**: `src/mcp/protocol.rs`
- **Issue**: 17 `.unwrap()` calls in serialization tests. Test failures produce panics with no context about which case failed.
- **Risk**: Debugging friction; masked failures.
- **Fix**: Use `expect("descriptive message")` or `assert!(result.is_ok(), "...")` patterns.

### 6. Hardcoded Alert Thresholds
- **File**: `src/analyzer/alerts.rs`
- **Issue**: Alert thresholds (e.g., error rate limits, pattern frequency cutoffs) are hardcoded constants that require recompilation to change.
- **Risk**: Operationally inflexible; users cannot tune without rebuilding.
- **Fix**: Move to `AlertConfig` section in `~/.config/logpilot/config.toml`.

### 7. SimHash Implementation Unvalidated
- **File**: `src/pipeline/dedup.rs`
- **Issue**: Custom SimHash implementation for log deduplication. Algorithm correctness and collision rate have not been benchmarked or compared against reference implementations.
- **Risk**: Either too-aggressive dedup (missing unique errors) or too-weak dedup (duplicates slip through).
- **Fix**: Add property-based tests or benchmark against known-good inputs. Consider using the `simhash` crate.

### 8. Database Path Not Validated
- **File**: `src/buffer/persistence.rs`
- **Issue**: SQLite database path is resolved via `dirs` crate but directory writability is not checked before attempting to open/create the database.
- **Risk**: Cryptic `sqlx` error on first run if the data directory doesn't exist or isn't writable.
- **Fix**: Check `fs::create_dir_all` and writability before opening the pool; surface a clear `LogPilotError::Config` message.

---

## Low Priority / Future Improvements

### 9. Analyzer Module Entirely Dead Code
- **File**: `src/analyzer/` (all files)
- **Issue**: `#![allow(dead_code)]` suppresses warnings across the entire analyzer module. The infrastructure (clustering, incident tracking, pattern analysis) is built but not wired to any CLI command or MCP resource.
- **Risk**: Code rot; increasing divergence from actual behavior as the rest of the system evolves.
- **Fix**: Wire analyzer output to the `summarize` CLI command and `logpilot://session/{name}/patterns` MCP resource, or remove until needed.

### 10. No Integration Tests
- **File**: `tests/` (absent)
- **Issue**: No integration test suite. All tests are inline unit tests. The capture → pipeline → buffer → MCP flow has no end-to-end coverage.
- **Risk**: Regressions in the wiring between components are not caught until runtime.
- **Fix**: Add `tests/integration_test.rs` covering at minimum: buffer ingestion, MCP resource reads, and severity filtering.

### 11. Clone-Heavy Data Flow
- **File**: Throughout `src/`
- **Issue**: 64 `.clone()` calls identified across the codebase. Most are on `String`, `LogEntry`, and `Arc` — some are avoidable with borrows.
- **Risk**: Minor heap allocation overhead; acceptable for current scale but worth auditing as throughput increases.
- **Fix**: Low priority — profile first before optimizing.

### 12. `chrono::Utc::now()` in Tests
- **File**: `src/pipeline/parser.rs` test helpers
- **Issue**: Test `LogEntry` construction uses `chrono::Utc::now()` for the base timestamp. Tests that check timestamp parsing correctness could produce flaky results near day/hour boundaries.
- **Fix**: Use a fixed sentinel timestamp (e.g., `DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")`) in test helpers.

---

## Positive Security Findings

- **No `unsafe` blocks**: Zero unsafe code in the entire codebase.
- **No SQL injection vectors**: All database queries use parameterized `sqlx` bindings.
- **Strong tmux input validation**: Allowlist regex + metacharacter rejection before subprocess execution.
- **No hardcoded secrets**: No API keys, tokens, or credentials in source.
- **Severity-gated persistence**: Only ERROR/FATAL logs written to disk — limits sensitive log exposure.
