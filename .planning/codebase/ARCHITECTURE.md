# Architecture

**Analysis Date:** 2026-08-26

## Pattern Overview

**Overall:** Layered async CLI pipeline with a dual crate (library + binary), tmux-backed capture, in-process analysis, and a hand-rolled MCP JSON-RPC adapter

**Key Characteristics:**
- Single Rust package (`logpilot` 0.1.3) exposing both `src/lib.rs` (integration tests, shared API) and `src/main.rs` (CLI binary that re-declares the same modules instead of depending on the library)
- Tokio async runtime with unbounded `mpsc` channels from capture tasks into a watch-loop consumer; concurrent maps (`dashmap`) and `tokio::sync::RwLock` for session/analyzer state
- Intended producer-consumer pipeline (`Capture -> Parser -> Deduplicator -> Cluster -> Analyzer`) is implemented as cooperating types, but the `Pipeline` orchestrator in `src/pipeline/mod.rs` is a channel stub; live `watch` wires components manually
- Persistence is hybrid: per-pane in-memory `VecDeque` ring buffer plus SQLite (`sqlx`) for ERROR/FATAL; MCP/summarize are meant to share a process-global `SessionDataStore` (`once_cell::OnceCell`)
- MCP is currently a custom JSON-RPC 2.0 stdio server (`src/mcp/server.rs`); official `rmcp` SDK is commented out in `Cargo.toml` and `src/mcp/mod.rs` due to Rust 1.86 compatibility

## Layers

**CLI / Presentation:**
- Purpose: Parse clap subcommands, print human-readable logs/alerts, and start long-running modes (`watch`, `mcp-server`)
- Location: `src/main.rs`, `src/cli/`
- Contains: `Commands` enum; handlers in `src/cli/watch.rs`, `src/cli/filter.rs`, `src/cli/summarize.rs`, `src/cli/ask.rs`, `src/cli/mcp.rs`, `src/cli/status.rs`; crossterm key handling in watch
- Depends on: capture, buffer, analyzer, models, mcp, pipeline parser (ask), tracing
- Used by: binary entry `src/main.rs` only (not re-exported as a public library API except tests that import `logpilot::cli::filter`)

**Capture (tmux I/O):**
- Purpose: Attach to tmux sessions/panes, stream pane output, detect disconnect/reconnect
- Location: `src/capture/`
- Contains: `TmuxCommand` (`src/capture/tmux.rs`) wrapping `tmux pipe-pane` / `list-sessions` / `list-panes` / `capture-pane`; `PaneCapture` FIFO reader (`src/capture/pane.rs`); `SessionManager` / `SessionRepository` (`src/capture/session.rs`)
- Depends on: `src/models/session.rs`, `src/models/pane.rs`, `src/models/log_entry.rs`, `src/error.rs`
- Used by: `src/cli/watch.rs` (live stream), `src/cli/filter.rs` and `src/cli/ask.rs` (session existence + pane list; they also call `tmux` directly for snapshots)

**Pipeline (parse / dedup / cluster):**
- Purpose: Turn raw pane lines into structured `LogEntry`s and cluster similar messages
- Location: `src/pipeline/`
- Contains: regex `LogParser` (`src/pipeline/parser.rs`); JSON/logfmt `FormatParser` (`src/pipeline/formats.rs`); SimHash `Deduplicator` (`src/pipeline/dedup.rs`); `ClusterEngine` / `ClusterManager` (`src/pipeline/cluster.rs`); stub `Pipeline` channel holder (`src/pipeline/mod.rs`)
- Depends on: `src/models/log_entry.rs`, `src/models/severity.rs`
- Used by: `Analyzer::process_entry` in `src/analyzer/mod.rs` (formats + parser + cluster); `src/cli/ask.rs` (`LogParser` only); integration tests in `tests/test_pipeline_integration.rs`

**Buffer (memory + SQLite):**
- Purpose: Time/capacity-bounded in-memory storage plus durable high-severity history
- Location: `src/buffer/`
- Contains: `RingBuffer` (`src/buffer/ring.rs`); `PersistenceStore` (`src/buffer/persistence.rs`); `BufferManager` (`src/buffer/manager.rs`)
- Depends on: models, `sqlx` SQLite pool, `src/error.rs`
- Used by: `src/cli/watch.rs` (`BufferManager::with_persistence` → `~/.local/share/logpilot/logs.db` or `.logpilot/logs.db`); tests in `tests/test_pipeline_integration.rs`

**Analyzer (anomaly / incident / alert):**
- Purpose: Pattern frequency, incident auto-creation, and alert evaluation
- Location: `src/analyzer/`
- Contains: `Analyzer` orchestrator (`src/analyzer/mod.rs`); `PatternTracker` / restart-loop detector (`src/analyzer/patterns.rs`); `IncidentDetector` (`src/analyzer/incidents.rs`); `AlertEvaluator`, `AlertRepository`, `ErrorRateCalculator` (`src/analyzer/alerts.rs`)
- Depends on: pipeline cluster/formats/parser, `src/models/{pattern,incident,alert,log_entry,severity}.rs`
- Used by: `src/cli/watch.rs`; tests in `tests/test_alerts.rs` and `tests/integration/test_analyzer.rs`

**MCP adapter:**
- Purpose: Expose session logs/patterns/incidents/alerts to AI hosts over stdio JSON-RPC
- Location: `src/mcp/`
- Contains: `McpServer` (`src/mcp/server.rs`); JSON-RPC types (`src/mcp/protocol.rs`); URI handlers (`src/mcp/resources.rs`); `SessionDataStore` (`src/mcp/data_store.rs`); unused `rmcp` implementation (`src/mcp/rmcp_server.rs`, not compiled)
- Depends on: models, serde_json, tracing, `url` query parsing
- Used by: `src/cli/mcp.rs`; `src/cli/summarize.rs` (global store); `src/cli/watch.rs` (local store instance); `tests/test_mcp_protocol.rs` (release binary)

**Domain models:**
- Purpose: Shared entities matching `specs/001-tmux-log-copilot/data-model.md`
- Location: `src/models/`
- Contains: `LogEntry`, `Session`/`SessionStatus`, `Pane`/`PaneStatus`, `Severity`, `Pattern`, `Incident`/`IncidentStatus`, `Alert`/`AlertType`/`AlertStatus`
- Depends on: serde, chrono, uuid
- Used by: all other layers

**Configuration / errors / observability:**
- Purpose: TOML config types, typed errors, dogfood metrics
- Location: `src/lib.rs` (`Config`, `BufferConfig`, `PatternConfig`, `AlertConfig`, `McpConfig`), `src/error.rs`, `src/observability.rs`
- Contains: `Config::load()` from `~/.config/logpilot/config.toml`; `LogPilotError`; `Metrics` counters (never imported by CLI/MCP production paths)
- Depends on: toml, dirs, thiserror, tracing
- Used by: library consumers/tests; CLI watch currently hardcodes buffer/alert thresholds instead of calling `Config::load()`

## Data Flow

**Watch live capture (primary path):**
1. `src/main.rs` dispatches `Commands::Watch` to `cli::watch::run` with session, optional pane, buffer minutes, min severity (`src/cli/watch.rs`).
2. `SessionRepository::create_session` verifies the tmux session (`TmuxCommand::session_exists`) and builds a `SessionManager` (`src/capture/session.rs`).
3. Capture starts: either `add_pane` for one target or `start_capture_all_panes` via `TmuxCommand::list_panes` (window `#I` then pane `#D`).
4. `PaneCapture::start` creates a temp FIFO, runs `tmux pipe-pane -t <id> 'exec cat >> fifo'`, and reads lines into `LogEntry::new` sent on an unbounded `mpsc` (`src/capture/pane.rs`).
5. The watch consumer prints entries at/above min severity, records errors in `ErrorRateCalculator`, persists via `BufferManager::add_entry` (SQLite if severity ≥ Error, always ring buffer), then `Analyzer::process_entry`.
6. Analyzer: JSON then logfmt (`FormatParser`), regex parse (`LogParser`), SimHash cluster (`ClusterEngine`), sliding-window pattern track, optional `Incident` creation.
7. Results are upserted into a **local** `SessionDataStore` (`src/mcp/data_store.rs`); `AlertEvaluator` checks recurring error, new exception, and error-rate; alerts print via `broadcast` and upsert.
8. Parallel tasks: 5s connection poll (`check_connection` → Active/Stale/Disconnected), crossterm keys (`a`/`s`/`?`/`q`), Ctrl+C / quit oneshot; cleanup aborts tasks and `remove_session`.

**Filter snapshot / follow:**
1. `cli::filter::handle` validates the session and lists panes (`src/cli/filter.rs`).
2. Snapshot: `tmux capture-pane -p -S -1000` per pane; keyword `detect_severity` + optional regex; print matches (does not use `LogParser` or `BufferManager`).
3. Follow: per-pane FIFO + `pipe-pane` like capture, but only severity/pattern matching on raw strings.

**Ask prompt generation:**
1. `cli::ask::handle` lists panes and `capture-pane` last 1000 lines (`src/cli/ask.rs`).
2. Each line becomes a `LogEntry` parsed by `LogParser`; filtered by min severity and `--last` window (timestamp extracted from content when possible).
3. Prints a markdown debugging prompt (up to 50 errors); does not read SQLite or `SessionDataStore`.

**Summarize:**
1. `cli::summarize::handle` parses `--last` duration (`src/cli/summarize.rs`).
2. Reads `get_or_init_global_store()`; if empty, prints a placeholder demo summary. Because `watch` constructs `SessionDataStore::new()` rather than the global singleton, live watch data is not visible to summarize/MCP in a separate process.

**MCP JSON-RPC:**
1. `cli::mcp::handle` constructs `McpServer::new()` and `run_stdio` (`src/mcp/server.rs`).
2. Line-delimited JSON-RPC 2.0 on stdin; methods: `initialize` (protocol `2025-06-18`), `resources/list`, `resources/read`, `tools/list`, `tools/call` (`search`, `stats`), `ping`; notifications (no `id`) get no response.
3. Resources: `logpilot://session/{name}/summary|entries|patterns|incidents|alerts` (`src/mcp/resources.rs`, schema in `specs/001-tmux-log-copilot/contracts/mcp-schema.json`).
4. Data comes from the process-global `OnceCell` store (`src/mcp/data_store.rs`); a 5-minute task drops sessions idle > 60 minutes.

**State Management:**
- Session/pane identity: UUID models plus tmux names/IDs; `SessionRepository` is an in-process `HashMap<String, Arc<SessionManager>>` (not shared across CLI invocations).
- Logs: per-pane `RingBuffer` (default 10k / N minutes) plus SQLite `log_entries` table with indexes on timestamp, severity, pane (`src/buffer/persistence.rs`). Watch DB path: `dirs::data_dir()/logpilot/logs.db`.
- Analysis: `Analyzer` holds `Arc<RwLock<...>>` cluster/pattern/incident state for the watch process lifetime.
- MCP/summarize: intended global `SessionDataStore` (`DashMap<String, RwLock<SessionData>>`, last 10k entries). Watch currently does not attach to that global instance.
- Config: `Config` in `src/lib.rs` documents `~/.config/logpilot/config.toml` (`config.example.toml`); live CLI paths do not load it; alert thresholds in watch use `AlertEvaluator::new()` defaults (10 errors/min, recurring 5).

## Key Abstractions

**LogEntry:**
- Purpose: Canonical captured line with pane_id, sequence, timestamp, severity, optional service, raw_content, parsed_fields
- Examples: `src/models/log_entry.rs`
- Pattern: serde DTO + builder helpers; created at capture time with `Severity::Unknown`, enriched by parsers

**Analyzer:**
- Purpose: Single-entry analysis facade combining format parse, regex parse, clustering, pattern windows, incidents
- Examples: `src/analyzer/mod.rs`
- Pattern: orchestrator over `Arc<RwLock<T>>` engines; returns `AnalysisResult`

**BufferManager:**
- Purpose: Per-pane ring buffers plus optional SQLite persistence gated by severity
- Examples: `src/buffer/manager.rs`, `src/buffer/ring.rs`, `src/buffer/persistence.rs`
- Pattern: write-through for ERROR+ (persist first, then memory); query APIs exist but watch does not query them after insert

**SessionManager / SessionRepository:**
- Purpose: Lifecycle of tmux-backed capture for one or many named sessions
- Examples: `src/capture/session.rs`, `src/capture/pane.rs`, `src/capture/tmux.rs`
- Pattern: repository of managers; FIFO + pipe-pane workers; reconnect task on stale (5 attempts / 5s)

**SessionDataStore:**
- Purpose: In-memory projection of entries/patterns/incidents/alerts for MCP and summarize
- Examples: `src/mcp/data_store.rs`
- Pattern: concurrent map + optional process singleton (`GLOBAL_DATA_STORE`)

**McpServer:**
- Purpose: MCP-shaped JSON-RPC server without the `rmcp` crate
- Examples: `src/mcp/server.rs`, `src/mcp/protocol.rs`, `src/mcp/resources.rs`; dormant SDK port `src/mcp/rmcp_server.rs`
- Pattern: request dispatch + async resource/tool handlers

**AlertEvaluator / ErrorRateCalculator:**
- Purpose: Threshold alerts (recurring error, new exception, restart loop, error rate) with broadcast fan-out
- Examples: `src/analyzer/alerts.rs`
- Pattern: DashMap of active alerts, `broadcast::channel(100)`

**Pipeline (stub):**
- Purpose: Documented producer-consumer wiring
- Examples: `src/pipeline/mod.rs`
- Pattern: unbounded sender only; real clustering lives in `ClusterEngine` used by `Analyzer`

## Entry Points

**CLI binary `logpilot`:**
- Location: `src/main.rs` (`[[bin]]` in `Cargo.toml`)
- Triggers: `cargo run -- <subcommand>`, `just run`, installed binary; clap subcommands `watch`, `filter`, `summarize`, `ask`, `mcp-server`, `status`
- Responsibilities: `tracing_subscriber::fmt::init()`, dispatch to `src/cli/*`; `watch` returns `LogPilotError`; other commands `eprintln` `anyhow`/`Result` errors and still exit 0 from `main`

**Library crate `logpilot`:**
- Location: `src/lib.rs`
- Triggers: integration tests (`tests/*.rs`, `tests/integration/*.rs`) via `use logpilot::...`; potential downstream crates
- Responsibilities: public modules including `observability` (not compiled into the binary), `Config::load()`, re-exports `Analyzer`, `Pipeline`, `LogPilotError`

**MCP stdio server:**
- Location: `src/cli/mcp.rs` → `src/mcp/server.rs` `run_stdio`
- Triggers: `logpilot mcp-server`, `just mcp`; protocol tests spawn `./target/release/logpilot mcp-server` (`tests/test_mcp_protocol.rs`)
- Responsibilities: JSON-RPC initialize/resources/tools; stderr banners; stale-session cleanup

**Watch interactive loop:**
- Location: `src/cli/watch.rs` `run`
- Triggers: `logpilot watch <session>`
- Responsibilities: full capture→buffer→analyze→print loop, keyboard UX, persistence init

**Direct tmux snapshot commands:**
- Location: `src/cli/filter.rs`, `src/cli/ask.rs`
- Triggers: `logpilot filter`, `logpilot ask`
- Responsibilities: one-shot or streamed pane text without `SessionRepository` / `Analyzer` (ask uses `LogParser` only)

**CI / test harness:**
- Location: `.github/workflows/ci.yml`, `Justfile`, `tests/`
- Triggers: GitHub Actions (fmt, clippy `-D warnings`, `cargo test --all-features` after `cargo build --release`); `just test` / `just ci`
- Responsibilities: unit tests colocated in `src/**`; integration tests against library + MCP binary

## Error Handling

**Strategy:** Domain `LogPilotError` (`thiserror`) for capture/buffer/config; several CLI handlers use `anyhow::Result` and print to stderr; MCP maps failures to JSON-RPC error codes

**Patterns:**
- Typed variants in `src/error.rs`: `Io`, `Tmux`, `Database` (from `sqlx::Error`), `DatabaseOp`, `Config`, `SessionNotFound`; constructors `tmux` / `config` / `db_op`
- Watch persistence failure degrades to in-memory (`BufferManager::new_in_memory`) with `tracing::warn`
- tmux target/path validation in `src/capture/tmux.rs` rejects shell metacharacters and `..` traversal before spawning commands
- MCP: parse errors → `invalid_request` (-32600); unknown method → `method_not_found` (-32601); bad params → `invalid_params` (-32602); serde failures → `internal_error` (-32603)
- Filter/watch spawn paths often `continue` on per-pane failures rather than aborting the whole session
- `main` for filter/summarize/ask/mcp/status swallows handler errors after `eprintln`, so process exit status is success unless `watch` fails

## Cross-Cutting Concerns

**Logging:** `tracing` + `tracing_subscriber::fmt` initialized in `src/main.rs`. Capture/MCP/watch use `info`/`warn`/`error`/`debug`. `src/observability.rs` defines structured events (`log_capture_event`, `log_mcp_request`, `Metrics`) but no production module imports it. MCP protocol chatter is also `eprintln` from `src/cli/mcp.rs`.

**Validation:** Clap for CLI args; duration parsers in summarize/ask; severity string maps in watch/filter/ask; regex compile errors become `LogPilotError::config`; MCP tools require non-empty `session`/`pattern`; tmux identifiers validated with `^[a-zA-Z0-9_\-\.:%]+$`.

**Authentication:** None. Local process, stdio MCP, filesystem SQLite under the user data dir. No tokens, TLS, or multi-tenant isolation.

---

*Architecture analysis: 2026-08-26*
