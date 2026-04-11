# Technical Concerns - LogPilot

> Last updated: April 2026

## Critical Issues

### 1. TOCTOU Race Condition in Buffer Manager
- **File**: `src/buffer/manager.rs:74-104`
- **Issue**: Double lock acquire pattern with async call between check and mutation. When `add_entry` is called, it first reads to check if buffer exists, drops the read lock, then acquires write lock to create buffer. Another task could create the same buffer between these operations.
- **Risk**: Data loss, duplicates, or corrupted state under concurrent load.
- **Fix**: Use `HashMap::entry()` API or hold write lock across entire check-create sequence.

### 2. Unbounded HashMap Growth in Deduplicator
- **File**: `src/pipeline/dedup.rs:27`
- **Issue**: `signatures: HashMap<String, u64>` grows indefinitely with no eviction. Long-running servers (days/weeks) accumulate millions of entries.
- **Risk**: OOM crash on long-running deployments.
- **Fix**: Implement LRU eviction or periodic TTL-based cleanup (e.g., remove entries older than dedup window).

### 3. Session Data Store Memory Leak
- **File**: `src/mcp/data_store.rs:80-102`
- **Issue**: Global `DashMap` accumulates sessions indefinitely. Dead/closed tmux sessions are never removed. While `entries` are capped at 10k, `patterns`, `incidents`, and `alerts` accumulate without bounds.
- **Risk**: Memory leak over time; stale data served to AI clients.
- **Fix**: Add session cleanup on tmux session close events, or periodic background sweep for sessions not seen in N minutes.

---

## High Priority

### 4. Analyzer Module Completely Dead Code
- **File**: `src/analyzer/mod.rs:2`
- **Issue**: `#![allow(dead_code)]` on entire module. Infrastructure (clustering, incidents, patterns, alerts) is built but not wired to any CLI command or MCP resource.
- **Risk**: Code rot; increasing divergence from actual behavior; misleading about production-readiness.
- **Fix**: Wire to `summarize` CLI command and `logpilot://session/{name}/patterns` MCP resource, or remove until needed.

### 5. Integration Tests Incomplete
- **File**: `tests/integration/test_analyzer.rs`
- **Issue**: 7 test functions with TODOs, all asserting placeholder checks only. PatternTracker, RestartLoopDetector, NewExceptionDetector, ErrorRateCalculator, IncidentDetector are not actually tested.
- **Risk**: Regressions in analyzer wiring not caught; refactoring without coverage.
- **Fix**: Implement actual analyzer unit tests or remove placeholder tests.

### 6. Database Path Not Validated
- **File**: `src/buffer/persistence.rs:19-28`
- **Issue**: SQLite path resolved via `dirs` crate but directory writability not checked before `SqlitePoolOptions::connect`. Cryptic `sqlx` error on first run if directory doesn't exist or isn't writable.
- **Risk**: Poor user experience on fresh install; confusing error messages.
- **Fix**: Check `fs::create_dir_all` and writability before opening pool; surface clear `LogPilotError::Config` message.

### 7. Hardcoded Alert Thresholds
- **File**: `src/analyzer/alerts.rs` (throughout)
- **Issue**: Alert thresholds (error rate limits, pattern frequency cutoffs) are hardcoded constants requiring recompilation to change.
- **Risk**: Operationally inflexible; users cannot tune without rebuilding.
- **Fix**: Move to `AlertConfig` section in `~/.config/logpilot/config.toml`.

---

## Medium Priority

### 8. SimHash Implementation Unvalidated
- **File**: `src/pipeline/dedup.rs:41-86`
- **Issue**: Custom SimHash implementation for log deduplication. Algorithm correctness and collision rate not benchmarked against reference or proven implementations.
- **Risk**: Either too-aggressive dedup (missing unique errors) or too-weak (duplicates slip through).
- **Fix**: Add property-based tests or benchmark against known-good inputs. Consider using the `simhash` crate.

### 9. Unwrap in Serialization Tests
- **File**: `src/mcp/protocol.rs:252-450`
- **Issue**: 18+ `.unwrap()` calls in test functions. Test failures produce panics with no context about which case failed.
- **Risk**: Debugging friction; masked failures in CI.
- **Fix**: Use `expect("descriptive message")` or `assert!(result.is_ok(), "...")` patterns.

### 10. Buffer Manager Double Lock Pattern
- **File**: `src/buffer/manager.rs:77-84`
- **Issue**: `add_entry` acquires read lock, checks key, drops, then acquires write lock. This is a common anti-pattern - should use `entry()` API.
- **Risk**: Suboptimal performance; potential edge cases under load.
- **Fix**: Refactor to use `buffers.entry(pane_id).or_insert_with(...)`.

---

## Low Priority

### 11. Clone-Heavy Data Flow
- **File**: Throughout `src/`
- **Issue**: ~64 `.clone()` calls identified. Many on `String`, `LogEntry`, and `Arc` - some avoidable with borrows.
- **Risk**: Minor heap allocation overhead; acceptable for current scale.
- **Fix**: Profile before optimizing; focus on hot paths in `add_entry`.

### 12. Timestamp in Tests Using `chrono::Utc::now()`
- **File**: `tests/integration/test_analyzer.rs:34`
- **Issue**: Test `LogEntry` construction uses `chrono::Utc::now()` for base timestamp. Tests checking timestamp parsing could produce flaky results near day/hour boundaries.
- **Fix**: Use fixed sentinel timestamp (e.g., `DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")`).

### 13. MCP Server Session Lookup Blocking
- **File**: `src/mcp/server.rs:220-242`
- **Issue**: `get_session` clones entire `SessionData` (including up to 10k entries) on every read. With high-frequency log ingestion, this creates contention.
- **Risk**: Performance degradation under load; memory spikes from cloning.
- **Fix**: Consider read-through caching or copy-on-write structures.

### 14. No Graceful Shutdown for MCP Server
- **File**: `src/mcp/server.rs:449-483`
- **Issue**: `run_stdio` loops indefinitely with no signal handling. No cleanup of global data store on exit.
- **Risk**: Resource leaks on restart; no opportunity to flush state.
- **Fix**: Add SIGTERM/SIGINT handler; implement graceful shutdown protocol.

---

## Positive Security Findings

- **No `unsafe` blocks**: Zero unsafe code in the entire codebase.
- **No SQL injection**: All queries use parameterized `sqlx` bindings.
- **Strong tmux input validation**: Allowlist regex + metacharacter rejection before subprocess execution.
- **No hardcoded secrets**: No API keys, tokens, or credentials in source.
- **Severity-gated persistence**: Only ERROR/FATAL logs written to disk.

---

## Quick Wins

| Priority | Issue | Effort | Impact |
|----------|-------|--------|--------|
| High | Remove dead analyzer code or wire it | Medium | High (clarity) |
| High | Fix TOCTOU in buffer manager | Low | High (correctness) |
| Medium | Add DB path validation | Low | Medium (UX) |
| Medium | Replace unwrap in tests | Low | Medium (DX) |
| Low | Add graceful shutdown | Medium | Low (reliability) |
