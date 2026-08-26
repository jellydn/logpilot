# Codebase Concerns

**Analysis Date:** 2026-08-26

## Tech Debt

**Hand-rolled MCP server while rmcp is disabled:**
- Issue: The running server is a custom JSON-RPC loop. `rmcp` is commented out “due to Rust 1.86 compatibility,” but the pin is already 1.98 and `AGENTS.md` still claims the official SDK. Dead `rmcp_server.rs` would not compile if re-enabled (it imports `rmcp`, which is not a dependency).
- Files: `src/mcp/mod.rs`, `src/mcp/server.rs`, `src/mcp/rmcp_server.rs`, `src/cli/mcp.rs`, `Cargo.toml`, `AGENTS.md`, `docs/MCP_TESTING.md`
- Impact: Protocol drift vs MCP clients; two implementations to keep in sync; docs/tests still mention rmcp error codes.
- Fix approach: Either restore `rmcp` on the current toolchain or delete `rmcp_server.rs` and rewrite `AGENTS.md` / MCP tests to match the legacy server. Advertise one protocol version.

**Binary vs library dual compile:**
- Issue: `src/main.rs` redeclares every module instead of using the `logpilot` library. `Config` and `observability` live only on the lib crate.
- Files: `src/main.rs`, `src/lib.rs`, `src/observability.rs`
- Impact: Tests exercise `logpilot::*` while the CLI is a second compilation unit. Config/metrics never reach the binary. Changes can land in one crate and miss the other.
- Fix approach: Make `main.rs` a thin clap wrapper over `logpilot` (`use logpilot::...`). Compile modules once.

**CLI flags defined twice and not wired:**
- Issue: `src/main.rs` hand-builds clap subcommands and constructs `*Args` structs. Module-level `#[derive(Args)]` fields (`summarize --format/--tokens/--errors-only`, `status --detailed/--session`) are never parsed. Completions and README still advertise those flags.
- Files: `src/main.rs`, `src/cli/summarize.rs`, `src/cli/status.rs`, `src/cli/ask.rs`, `completions/logpilot.fish`, `README.md`
- Impact: Documented commands fail (`summarize --format json`, `ask --include-logs`, `status -d`). Users get silent hardcoded defaults.
- Fix approach: Parse with the module `Args` types (`#[command(flatten)]` or `enum Commands { Summarize(SummarizeArgs) }`). Regenerate completions from clap.

**Config file is unused:**
- Issue: `Config::load()` reads `~/.config/logpilot/config.toml`, but no CLI path calls it. Alert windows, persist path, `max_memory_mb`, custom parse patterns, and MCP enablement are hardcoded in constructors.
- Files: `src/lib.rs`, `config.example.toml`, `src/cli/watch.rs`, `src/analyzer/alerts.rs`, `src/analyzer/patterns.rs`
- Impact: User config is a no-op. Thresholds in `config.example.toml` do not change runtime behavior.
- Fix approach: Load config at process start and pass it into `AlertEvaluator::with_thresholds`, `BufferManager`, parser, and persist path.

**Pipeline / capture / MCP marked “not wired”:**
- Issue: Module-level `#![allow(dead_code)]` on `buffer`, `pipeline`, `capture`, `analyzer`, and `mcp`. `Pipeline` is a channel stub, not Capture → Parse → Dedup → Cluster → Analyzer.
- Files: `src/buffer/mod.rs`, `src/pipeline/mod.rs`, `src/capture/mod.rs`, `src/analyzer/mod.rs`, `src/mcp/mod.rs`
- Impact: Clippy cannot catch unused API. Watch reimplements a partial pipeline inline.
- Fix approach: Wire `Pipeline` from `watch`, drop blanket allows, and `allow` only leftover planned types.

**Duplicated tmux snapshot capture:**
- Issue: `capture-pane -p -S -1000/-100` is copy-pasted in filter, ask, and both MCP servers, bypassing `TmuxCommand`. Duration parsing is duplicated in `ask` and `summarize`. Severity string parsing is duplicated in watch/filter/ask.
- Files: `src/cli/filter.rs`, `src/cli/ask.rs`, `src/mcp/server.rs`, `src/mcp/rmcp_server.rs`, `src/cli/summarize.rs`, `src/cli/watch.rs`
- Impact: Validation and error handling diverge; injection checks in `TmuxCommand` do not apply to snapshot paths.
- Fix approach: Add `TmuxCommand::capture_pane(target, lines)` and one `parse_duration` / `parse_severity` helper.

**SQLite schema and `AssertSqlSafe`:**
- Issue: `log_entries` has no `session_name` (ask comments that the DB cannot query by session). Dynamic SQL uses `sqlx::AssertSqlSafe`. Connect URL is `sqlite:{path}` with no `create_if_missing`, WAL, or busy timeout.
- Files: `src/buffer/persistence.rs`, `src/cli/ask.rs`, `src/cli/watch.rs`
- Impact: First-run DB create can fail and watch silently falls back to memory. Historical queries cannot filter by session. Schema is hard to migrate.
- Fix approach: `SqliteConnectOptions::create_if_missing(true)` + WAL; add `session_name`; use bound static SQL (the extra `AND severity = ?3` branch can be two queries).

**Docs / toolchain drift:**
- Issue: README version badge is 0.1.0 (crate is 0.1.3). Protocol banner says `2024-11-05` while initialize returns `2025-06-18`. `rust-version = "1.86"`, `rust-toolchain.toml` is `1.98`, CI uses `dtolnay/rust-toolchain@stable`. `strip = false` is a macOS Sequoia workaround in release profile.
- Files: `README.md`, `src/cli/mcp.rs`, `src/mcp/server.rs`, `docs/MCP_TESTING.md`, `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/ci.yml`
- Impact: MCP clients and humans disagree on protocol; MSRV is untested.
- Fix approach: Single source of version/protocol; CI matrix on claimed MSRV; document the strip workaround.

## Known Bugs

**Watch and MCP do not share live session data:**
- Symptoms: `resources/read` returns “Session not found. Is the watch command running?” even while `watch` is capturing. `summarize` prints “No active watch sessions” or a fake demo summary. `status` always says no sessions.
- Files: `src/cli/watch.rs` (local `SessionDataStore::new()`), `src/mcp/data_store.rs` (`OnceCell` global), `src/mcp/server.rs`, `src/cli/summarize.rs`, `src/cli/status.rs`
- Trigger: Run `logpilot watch SESSION` in one process and `logpilot mcp-server` / `summarize` / `status` in another (the intended Claude Code layout).
- Workaround: MCP `search`/`stats` fall back to a one-shot `capture-pane` snapshot (no patterns/incidents/alerts). There is no workaround for live MCP resources.

**`Severity::Unknown` sorts above `Fatal`:**
- Symptoms: Unparsed lines (`LogEntry::new` defaults to `Unknown`) satisfy `severity >= Error` and `>= Warn`. Watch persists and prints them; filter snapshot/stream treats them as matches (`⚫`); error-rate counting includes them. `ask` special-cases `sev != Unknown` — other paths do not.
- Files: `src/models/severity.rs`, `src/models/log_entry.rs`, `src/cli/watch.rs`, `src/cli/filter.rs` (`line_matches`), `src/buffer/manager.rs`, `src/cli/ask.rs`
- Trigger: Watch or filter a pane whose lines lack TRACE/DEBUG/INFO/WARN/ERROR/FATAL tokens.
- Workaround: None in watch/filter. Use `ask`, which excludes `Unknown`.

**Watch analyzes a clone and stores the unparsed original:**
- Symptoms: SQLite and the in-process data store keep `severity: Unknown` and empty `parsed_fields`. Analyzer JSON/logfmt/regex parse results are discarded except for pattern/incident side effects.
- Files: `src/cli/watch.rs` (`add_entry(entry.clone())` then `process_entry(entry.clone())` then `data_store.add_entry(entry)`), `src/analyzer/mod.rs`, `src/capture/pane.rs`
- Trigger: Any `watch` session.
- Workaround: None. MCP snapshot path does parse; live path does not.

**CLI commands swallow errors and exit 0:**
- Symptoms: Failed `filter` / `summarize` / `ask` / `mcp-server` / `status` print `Error: ...` and still return success. Scripts and CI cannot detect failure.
- Files: `src/main.rs`
- Trigger: Missing tmux session, invalid duration, MCP stdio error.
- Workaround: None. `watch` is the only subcommand that uses `?`.

**`summarize` fabricates data when the store is empty:**
- Symptoms: After printing “No active watch sessions,” it still emits 49 entries, INFO/WARN/ERROR counts, and services `api-service` / `db-service` from `generate_summary_placeholder`.
- Files: `src/cli/summarize.rs`
- Trigger: `logpilot summarize --last 10m` without a co-process watch (always, given the store split).
- Workaround: Ignore the placeholder; it is not real logs.

**Watch `a` does not acknowledge alerts:**
- Symptoms: Key `a` prints “Acknowledged N alerts” but never calls `AlertEvaluator::acknowledge`.
- Files: `src/cli/watch.rs`, `src/analyzer/alerts.rs`
- Trigger: Press `a` during watch after alerts fire.
- Workaround: None.

**Stopping watch does not stop `tmux pipe-pane`:**
- Symptoms: After quit, tmux may keep piping to a deleted FIFO. Next `pipe-pane` on that pane can fail or attach to a stale pipe.
- Files: `src/capture/pane.rs` (`MultiPaneCapture::stop_all` only signals shutdown and `clear`s, never `TmuxCommand::stop_pipe`), `src/capture/session.rs` (`SessionManager::stop`)
- Trigger: `q` / Ctrl+C on `watch`.
- Workaround: Manually `tmux pipe-pane -t <pane>` to clear.

**`SessionManager::session_id()` always returns nil; reconnect does not reattach panes:**
- Symptoms: `session_id()` is `Uuid::nil()`. Disconnect marks stale and spawns a reconnect task that only flips status; it does not call `start_capture_all_panes` again. Reconnect handle is not stored (`&self` cannot write `reconnect_handle`).
- Files: `src/capture/session.rs`
- Trigger: Kill/recreate the tmux session while watch is running.
- Workaround: Restart `watch`.

**SQLite persistence can fail open:**
- Symptoms: If `PersistenceStore::new` fails (missing `create_if_missing`, permissions), watch logs a warning and continues in-memory only. ERROR/FATAL history is gone after exit.
- Files: `src/cli/watch.rs`, `src/buffer/persistence.rs`
- Trigger: First run before the DB file exists, depending on sqlx URL behavior; or unwritable data dir.
- Workaround: Pre-create `~/.local/share/logpilot/logs.db`.

**Filter `--context` is ignored; user `--pane` skips target validation:**
- Symptoms: `-C` is accepted and unused (`_context`). `--pane` is passed straight to `tmux capture-pane` / `pipe-pane`.
- Files: `src/cli/filter.rs`
- Trigger: `logpilot filter SESSION -C 3` or `--pane` with a crafted target.
- Workaround: None for context. Prefer listing panes via `TmuxCommand`.

**Restart-loop detection is never invoked from watch:**
- Symptoms: `PatternTracker::check_restart_loop` / `AlertEvaluator::check_restart_loop` exist and have unit tests, but the watch loop never calls them.
- Files: `src/cli/watch.rs`, `src/analyzer/patterns.rs`, `src/analyzer/alerts.rs`
- Trigger: Service start/stop loops in a watched pane.
- Workaround: None.

## Security Considerations

**tmux command injection (partially mitigated, inconsistently applied):**
- Risk: Session/pane names or FIFO paths interpolated into `tmux pipe-pane`’s shell command (`exec cat >> 'path'`). A crafted target could attach to the wrong pane or run a pipe command.
- Files: `src/capture/tmux.rs` (`validate_target`, `validate_path`, `pipe_pane`), `src/cli/filter.rs`, `src/cli/ask.rs`, `src/mcp/server.rs`, `src/mcp/rmcp_server.rs`
- Current mitigation: `TmuxCommand` allows only `[A-Za-z0-9_.:%-]`, rejects `..` and shell metacharacters in paths, and passes argv (no `/bin/sh -c`) for tmux itself. Pane IDs from `list-panes -F #D` are `%N`.
- Recommendations: Route all capture-pane/pipe-pane through `TmuxCommand`. Validate `--pane`. Prefer tmux `-I`/`pipe-pane` without a shell command if a version supports it. Never concatenate user strings into the pipe command beyond the already-quoted path.

**World-writable FIFOs in `/tmp`:**
- Risk: `logpilot-fifo-{pane}-{pid}` and `logpilot-filter-...fifo` in `std::env::temp_dir()`. Another local user may open the FIFO or replace it (TOCTOU between `mkfifo` and `pipe-pane`).
- Files: `src/capture/pane.rs`, `src/cli/filter.rs`
- Current mitigation: UUID/pid in the name; FIFO removed on capture-loop exit (not always on `stop_all`).
- Recommendations: Create FIFOs under `XDG_RUNTIME_DIR` with mode `0600` (`nix::sys::stat::umask` or `libc::mkfifo`). Stop pipe-pane before unlink.

**MCP is an unauthenticated local tmux reader:**
- Risk: Any local client on the stdio MCP server can `tools/call` `search`/`stats` for any tmux session name that passes `validate_target`, dumping pane contents. No session allowlist.
- Files: `src/mcp/server.rs`, `src/cli/mcp.rs`
- Current mitigation: stdio transport (Claude Code spawns the process). Target charset filter on session_exists.
- Recommendations: Optional allowlist of session prefixes; never dump full pane history by default; document that MCP equals “read all local tmux.”

**SQLite path and shared DB:**
- Risk: DB path is `dirs::data_dir()/logpilot/logs.db` with no canonicalize; all sessions share one file with no session column, so one process’s ERROR lines mix with another’s. `format!("sqlite:{}", db_path)` can break on special characters.
- Files: `src/cli/watch.rs`, `src/buffer/persistence.rs`
- Current mitigation: Parent dir `create_dir_all`; parameterized INSERTs.
- Recommendations: `SqliteConnectOptions` with explicit filename; include `session_name`; do not interpolate raw paths into URLs.

**User regex (ReDoS) on every streamed line:**
- Risk: `filter -R` compiles arbitrary regex and runs it per line on live FIFO output.
- Files: `src/cli/filter.rs`
- Current mitigation: `Regex::new` error is returned as config error (no timeout).
- Recommendations: Size-limit the pattern; consider `regex::RegexBuilder` size limits; document untrusted-pattern risk.

**No `unsafe` blocks** were found in `src/` or `tests/`. Production `unwrap`/`expect` are limited to static `Lazy` regex compiles (plus tests).

## Performance Bottlenecks

**Per-line clone + lock + SQLite on the watch hot path:**
- Problem: Each captured line is cloned for print, persist, analyze, and MCP store. Analyzer takes write locks on cluster engine, cluster manager, pattern tracker, and incident detector. `check_error_rate` runs on every line. Persistence `INSERT`s synchronously when `severity >= persist_threshold` (and `Unknown` currently qualifies).
- Files: `src/cli/watch.rs`, `src/analyzer/mod.rs`, `src/buffer/manager.rs`, `src/buffer/persistence.rs`, `src/mcp/data_store.rs`
- Cause: No pipeline batching; analysis is inline in the receive loop; SQLite is on the same task.
- Improvement path: Parse once, store `Arc<LogEntry>`; batch SQLite writes; move analysis to a worker; skip error-rate checks except on a timer.

**Unbounded in-memory growth:**
- Problem: `mpsc::unbounded_channel` for log lines. `ErrorRateCalculator` appends timestamps and never runs `cleanup()` from watch. `SessionData` caps entries at 10k via `drain(0..len-10000)` (copies the tail). Patterns/incidents/alerts in the store are uncapped. Deduplicator caps at 100k signatures.
- Files: `src/cli/watch.rs`, `src/analyzer/alerts.rs`, `src/mcp/data_store.rs`, `src/pipeline/dedup.rs`
- Cause: Retention is implemented (`RingBuffer::cleanup`, `PersistenceStore::cleanup_before`) but not scheduled.
- Improvement path: Bounded channel with drop/metrics; periodic cleanup task; ring buffer for error timestamps.

**Blocking I/O on the tokio runtime:**
- Problem: MCP `run_stdio` uses sync `stdin.lock().lines()` then `.await`s handlers on the same runtime. Watch key handler calls `crossterm::event::read()` (blocking) inside `tokio::spawn`. Connection check polls `tmux` every 5s.
- Files: `src/mcp/server.rs`, `src/cli/watch.rs`, `src/capture/session.rs`
- Cause: Stdio MCP and TUI input were not moved to `spawn_blocking` / async readers.
- Improvement path: `tokio::io::AsyncBufRead` for MCP; `spawn_blocking` or crossterm event stream for keys.

**Snapshot capture scales with pane count × 1000 lines:**
- Problem: `filter`/`ask`/`mcp` walk every pane and `capture-pane -S -1000` (MCP stats uses `-100`). Ask sorts all parsed errors in memory.
- Files: `src/cli/filter.rs`, `src/cli/ask.rs`, `src/mcp/server.rs`
- Cause: No reuse of the ring buffer/SQLite (schema has no session name).
- Improvement path: Query persistence by session; cap panes; stream parse.

**Ring buffer is a `VecDeque`, not time-evicted on push:**
- Problem: Capacity eviction is O(1); time retention only runs if `cleanup()` is called (watch never does). Default 10k entries × N panes.
- Files: `src/buffer/ring.rs`, `src/buffer/manager.rs`, `src/cli/watch.rs`
- Cause: `max_memory_mb` in config is unused.
- Improvement path: Evict by age on `push`; honor memory budget.

## Fragile Areas

**tmux `pipe-pane` exclusivity and FIFO lifecycle:**
- Files: `src/capture/tmux.rs`, `src/capture/pane.rs`, `src/cli/filter.rs`
- Why fragile: tmux allows one pipe-pane per pane. LogPilot replaces any existing pipe. FIFOs in `/tmp` plus reopen-on-EOF loops hide disconnects. `stop_all` does not stop pipes.
- Safe modification: Always pair `pipe_pane` with `stop_pipe` in `Drop`/stop; test against real tmux; do not add a second capture method on the same pane without coordination.
- Test coverage: `tests/integration/test_capture.rs` is not a Cargo integration crate (nested under `tests/integration/` with no `main.rs`/`lib.rs`), so attach/latency/stale-session tests never run. `src/capture/tmux.rs` only tests validators and `is_installed`.

**MCP protocol surface vs spec/docs:**
- Files: `src/mcp/protocol.rs`, `src/mcp/server.rs`, `src/mcp/resources.rs`, `docs/MCP_TESTING.md`, `specs/001-tmux-log-copilot/contracts/mcp-schema.json`
- Why fragile: Capabilities use non-standard `resources.supportedUris` instead of MCP `resources.listChanged` / resource templates. `resources/list` returns URI templates, not live sessions. `handle_resources_read` (sync) always returns -32002. Banner/docs say protocol `2024-11-05`; initialize says `2025-06-18`. `notifications/initialized` is an unknown method (no response because id is null).
- Safe modification: Change protocol types only with `tests/test_mcp_protocol.rs` and a real client (Claude Code). Keep stderr off stdout.
- Test coverage: Protocol tests cover initialize/ping/unknown method/list tools/unknown tool. They do not cover `resources/read` success, `tools/call` search/stats, or initialize notification.

**Watch TUI + capture tasks:**
- Files: `src/cli/watch.rs`
- Why fragile: Multiple spawned tasks, abort-on-quit, blocking stdin read, emoji UI, no raw-mode setup documented. Data store is discarded on exit so MCP never saw it anyway.
- Safe modification: Keep processing off the UI thread; do not add more work to the `log_rx` loop without backpressure.
- Test coverage: No watch end-to-end test in `cargo test`.

**Analyzer integration tests are stubs:**
- Files: `tests/integration/test_analyzer.rs`
- Why fragile: Tests create fixtures then `// TODO: Feed entries to PatternTracker` and assert only vector lengths. Even if discovered by Cargo, they would not catch detector regressions.
- Safe modification: Drive `Analyzer::process_entry` / `AlertEvaluator` with deterministic clocks.
- Test coverage: Real alert unit tests live in `src/analyzer/alerts.rs` and `tests/test_alerts.rs`. Recurring-error/restart-loop/dedup/window-decay/error-rate/incident-auto-create integration tests are incomplete.

## Scaling Limits

**In-memory ring / MCP session store:**
- Current capacity: 10_000 entries per pane (`BufferManager` / `RingBuffer` default) and 10_000 entries per `SessionData`. Dedup map 100_000 signatures, 1h TTL.
- Limit: High-volume panes wrap the ring; MCP store `drain` copies ~10k `LogEntry`s; unbounded `mpsc` can OOM before the ring drops.
- Scaling path: Shared ring between watch and MCP (IPC); bounded queues; time-based eviction on push.

**SQLite persistence:**
- Current capacity: Pool `max_connections(5)` (file) or 1 (memory tests). Indexes on timestamp, severity, pane_id. No WAL, no size cap, no scheduled `cleanup_before`.
- Limit: Every qualifying line is an INSERT on the capture task. Mixed sessions in one table. `Unknown` currently inflates writes.
- Scaling path: WAL + batched inserts; session column; TTL job; `create_if_missing`.

**tmux polling and pane fan-out:**
- Current capacity: Connection poll every 5s; reconnect max 5 attempts × 5s. Snapshot commands iterate all panes.
- Limit: Sessions with many panes multiply `capture-pane` and FIFO tasks. `list_panes` does list-windows then list-panes per window.
- Scaling path: `list-panes -s -t session`; event-driven attach if tmux control mode is acceptable.

**Config `max_memory_mb = 100`:**
- Current capacity: Documented only; unused. Memory is “10k × panes × clone fan-out.”
- Limit: No hard stop.
- Scaling path: Enforce budget in `BufferManager::stats`.

## Dependencies at Risk

**`rmcp` (commented out):**
- Risk: Official Rust MCP SDK disabled over a stale 1.86 note. Dead file still in tree. Tests comment about rmcp error codes.
- Impact: Custom protocol will lag MCP; re-enable is a compile break.
- Migration plan: Add `rmcp` on toolchain 1.98, compile `rmcp_server.rs`, switch `cli/mcp.rs`, delete `server.rs` protocol types that duplicate the SDK.

**`sqlx` 0.9.0:**
- Risk: Uses `AssertSqlSafe` and string URL connect without `SqliteConnectOptions`. 0.9 is new relative to many examples still on 0.7/0.8.
- Impact: First-run DB open, API churn, compile-time check bypass.
- Migration plan: Typed `SqliteConnectOptions`; static queries; pin and watch release notes.

**`once_cell` 1.21:**
- Risk: Redundant on edition 2021 + MSRV 1.86 (`std::sync::LazyLock` / `OnceLock`).
- Impact: Extra dep; `Lazy` regexes and global store.
- Migration plan: Replace with std.

**`tokio` with `features = ["full"]`:**
- Risk: Pulls unused macros/fs/process/signal/rt-multi-thread surface; larger binary.
- Impact: Compile time; harder to reason about enabled features.
- Migration plan: Enable only `rt-multi-thread`, `macros`, `process`, `io-util`, `fs`, `sync`, `time`, `signal`.

**Release `strip = false`:**
- Risk: Workaround for macOS Sequoia “mis-aligned LINKEDIT string pool”; larger binaries on all targets.
- Impact: CI Linux artifacts also unstripped.
- Migration plan: Target-specific profile or `cargo-binutils` strip only on Linux.

## Missing Critical Features

**Cross-process live context for MCP / status / summarize:**
- Problem: Watch, MCP, and other CLIs are separate processes with an in-process `OnceCell`. No Unix socket, no SQLite reader for MCP resources, no session registry.
- Blocks: Claude Code MCP resources, `logpilot status`, real `summarize`, the product’s “AI context bridge” claim.

**Config-driven behavior:**
- Problem: `~/.config/logpilot/config.toml` is documented and parsed in the library but never loaded by the CLI.
- Blocks: Custom log patterns, alert thresholds, persist path, MCP enable flag.

**Session-scoped persistence:**
- Problem: SQLite rows have `pane_id` (random UUID per capture) and no session name.
- Blocks: `ask`/`summarize` from history without live tmux; multi-session support on one machine.

**Wired analysis pipeline:**
- Problem: Dedup/cluster/parser/format parsers exist; watch only runs `Analyzer::process_entry` on a clone and skips restart-loop checks. `Pipeline` is unused.
- Blocks: Accurate patterns/incidents on stored data; consistent parse between watch and snapshot tools.

**Honest CLI/status:**
- Problem: `status` is a placeholder. `summarize` can emit demo numbers. README/completions describe flags that `main.rs` does not parse.
- Blocks: Operational use and scripting.

**MCP resource listing of real sessions and protocol negotiate:**
- Problem: `resources/list` returns templates with `{name}`; no `notifications/initialized` handler; protocol version mismatch in UX vs initialize.
- Blocks: Clients that discover resources dynamically.

## Test Coverage Gaps

**`tests/integration/` is not executed by Cargo:**
- What's not tested: tmux attach, <2s capture latency, multi-session capture, stale-session standby (`test_capture.rs`); analyzer TODOs (`test_analyzer.rs`). Cargo only auto-discovers `tests/*.rs`, not nested files without a crate root.
- Files: `tests/integration/test_capture.rs`, `tests/integration/test_analyzer.rs`, `tests/fixtures/mock_tmux.sh`
- Risk: Capture/US1 regressions never fail CI.
- Priority: High

**MCP protocol tests require a pre-built release binary:**
- What's not tested: `just test` / `cargo test` without `cargo build --release` spawn `./target/release/logpilot` and fail. Success path for `tools/call` search/stats and `resources/read` with a populated store is untested. Tests still mention rmcp.
- Files: `tests/test_mcp_protocol.rs`, `Justfile`, `.github/workflows/ci.yml` (CI does build release first)
- Risk: Local `just test` is red; protocol regressions in live data path go unnoticed.
- Priority: High

**Watch ↔ MCP sharing and first-run SQLite:**
- What's not tested: Global vs local `SessionDataStore`; `create_if_missing`; Unknown-severity persistence; pipe-pane cleanup; CLI exit codes.
- Files: `src/cli/watch.rs`, `src/mcp/data_store.rs`, `src/buffer/persistence.rs`, `src/main.rs`
- Risk: The main product bug (empty MCP during watch) has no failing test.
- Priority: High

**Severity ordering and filter/watch display:**
- What's not tested: `Severity` `PartialOrd` vs “unknown is not an error.” `test_severity_ordering` only checks Error>Warn and Fatal>Error, not Unknown vs Fatal.
- Files: `src/models/severity.rs`, `src/cli/filter.rs`, `tests/test_filter.rs`
- Risk: All unparsed logs treated as high severity in production.
- Priority: High

**Analyzer integration TODOs:**
- What's not tested: Recurring error window, restart loop, new exception, simhash grouping, window decay, error-rate, incident auto-create — comments say feed the detectors; bodies only assert fixture sizes.
- Files: `tests/integration/test_analyzer.rs`
- Risk: Detection logic only covered by isolated unit tests with pre-built `Pattern` objects, not full `Analyzer::process_entry`.
- Priority: Medium

**Filter context, summarize flags, status:**
- What's not tested: `-C` context (unimplemented), `--format json` (unparsed), status output.
- Files: `src/cli/filter.rs`, `src/cli/summarize.rs`, `src/cli/status.rs`
- Risk: Docs lie; no test fails when flags are missing.
- Priority: Medium

**No `unsafe` / production-panic audit gap:**
- What's not tested: N/A — no `unsafe`. Static regex `expect`s are compile-time constants.
- Files: `src/pipeline/parser.rs`, `src/pipeline/dedup.rs`, `src/capture/tmux.rs`
- Risk: Low if regex literals stay valid.
- Priority: Low

---

*Concerns audit: 2026-08-26*
