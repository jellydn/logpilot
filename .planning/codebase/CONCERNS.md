# Codebase Concerns

**Analysis Date:** 2025-04-10

## Tech Debt

**MCP Server Live Data Integration:**
- Status: **FIXED** ✅
- Files: `src/mcp/server.rs`, `src/mcp/data_store.rs`, `src/cli/watch.rs`
- Solution: Implemented `SessionDataStore` - a thread-safe shared data store using `DashMap` and `RwLock` for concurrent access
- How it works:
  1. Watch command creates a `SessionDataStore` for each monitored session
  2. Log entries, patterns, incidents, and alerts are synced to the store as they're processed
  3. MCP server reads from the same store when serving resource requests
  4. Data is automatically cleaned up when the watch command exits
- Result: MCP server now serves real-time session data to AI assistants

**unwrap() Usage in Tests and Production:**
- Status: **PARTIALLY FIXED** ✅
- Files: `src/buffer/manager.rs` (test-only), `src/mcp/resources.rs` (test-only), `src/pipeline/parser.rs` ✅, `src/pipeline/dedup.rs` ✅
- Fix applied: Replaced regex compilation unwrap() with `once_cell::Lazy` for static regex caching
- Remaining: Buffer manager and MCP resources use unwrap() only in test code

## Known Bugs

**None explicitly documented** - No open issues file or bug tracker observed

## Security Considerations

**Command Injection via tmux:**
- Status: **FIXED** ✅
- Files: `src/capture/tmux.rs`
- Mitigation applied: 
  - `validate_target()` - validates tmux session/pane identifiers (alphanumeric + limited punctuation)
  - `validate_path()` - prevents path traversal and shell injection
  - Blocks shell metacharacters (`;`, `|`, `&`, `$`, etc.) and path traversal (`..`)
- Tests: 6 security validation tests added

**SQL Injection:**
- Risk: Low (uses sqlx with parameterized queries)
- Current mitigation: sqlx query builder
- Recommendations: Continue using sqlx, avoid raw SQL strings

**Path Traversal in Config:**
- Risk: User-controlled paths in config could access arbitrary files
- Files: `src/lib.rs` (Config::load)
- Current mitigation: Limited to config directories
- Recommendations: Validate paths are within expected directories

## Performance Bottlenecks

**Regex Compilation:**
- Status: **FIXED** ✅
- Files: `src/pipeline/parser.rs` ✅, `src/pipeline/dedup.rs` ✅
- Fix applied: Implemented `once_cell::sync::Lazy` for all regex patterns
- Result: Regexes compiled once at startup, eliminating ~12 recompilations per log entry

**Buffer Memory Usage:**
- Problem: Unbounded memory growth in ring buffer
- Files: `src/buffer/ring.rs`
- Cause: Time-based expiration only, no byte-size limit enforcement
- Improvement path: Implement byte-size eviction in addition to time-based

**SQLite Concurrent Access:**
- Problem: Single writer bottleneck for persistence
- Files: `src/buffer/persistence.rs`
- Cause: SQLite with default locking
- Improvement path: Connection pooling or write batching

## Fragile Areas

**tmux Dependency:**
- Files: `src/capture/`
- Why fragile: External process dependency, tmux version differences
- Safe modification: Test with multiple tmux versions, wrap tmux commands
- Test coverage: Limited - requires tmux environment

**MCP Protocol Compliance:**
- Files: `src/mcp/protocol.rs`, `src/mcp/server.rs`
- Why fragile: Custom protocol implementation may drift from spec
- Safe modification: Reference MCP specification, add protocol tests
- Test coverage: Basic tests present but may not cover edge cases

**Pattern Matching:**
- Files: `src/pipeline/parser.rs`, `src/analyzer/patterns.rs`
- Why fragile: Regex patterns may not match all log formats
- Safe modification: Add more test cases for different log formats
- Test coverage: Unit tests exist but may miss edge cases

## Scaling Limits

**In-Memory Buffer:**
- Current capacity: Configurable (default 30 min, 100MB)
- Limit: Single process memory
- Scaling path: Externalize to Redis or similar for multi-process

**SQLite Database:**
- Current capacity: Single-file, single-writer
- Limit: Write concurrency, file size
- Scaling path: PostgreSQL or other server database for high volume

## Dependencies at Risk

**None identified** - All dependencies are widely-used, actively maintained crates

## Missing Critical Features

**Real-time MCP Data:**
- Problem: MCP server may serve stale data
- Blocks: AI assistants getting current session state

**Configuration Hot-Reload:**
- Problem: Config changes require restart
- Blocks: Dynamic configuration updates

**Log Format Auto-Detection:**
- Problem: Manual pattern configuration required
- Blocks: Zero-config log parsing

**Alerting Integration:**
- Problem: No external notification system (email, Slack, PagerDuty)
- Blocks: Production incident response workflows

## Test Coverage Gaps

**tmux Integration:**
- What's not tested: Actual tmux command execution
- Files: `src/capture/tmux.rs`, `src/capture/session.rs`
- Risk: tmux version incompatibilities
- Priority: High

**MCP Server Full Protocol:**
- What's not tested: All JSON-RPC edge cases, error handling
- Files: `src/mcp/server.rs`
- Risk: Protocol non-compliance
- Priority: Medium

**Error Recovery Paths:**
- What's not tested: Network failures, disk full, permission errors
- Files: Various
- Risk: Silent failures or crashes
- Priority: Medium

**Concurrent Access:**
- What's not tested: Multiple simultaneous sessions
- Files: `src/buffer/manager.rs`
- Risk: Race conditions
- Priority: Medium

---

*Concerns audit: 2025-04-10*
