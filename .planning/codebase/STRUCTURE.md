# Codebase Structure

**Analysis Date:** 2026-08-26

## Directory Layout

```
2026-07-29-jellydn-logpilot/
├── src/                          # Rust library + CLI sources
│   ├── main.rs                   # Binary entry: clap CLI dispatch
│   ├── lib.rs                    # Library crate: modules + Config
│   ├── error.rs                  # LogPilotError / Result
│   ├── observability.rs          # Unused metrics helpers (lib-only)
│   ├── analyzer/                 # Patterns, incidents, alerts
│   ├── buffer/                   # Ring buffer + SQLite
│   ├── capture/                  # tmux pipe-pane / session managers
│   ├── cli/                      # Subcommand handlers
│   ├── mcp/                      # JSON-RPC MCP server
│   ├── models/                   # Domain entities
│   └── pipeline/                 # Parse, dedup, cluster
├── tests/                        # Integration / protocol tests
│   ├── fixtures/                 # mock_tmux.sh
│   └── integration/              # Analyzer + capture tests
├── docs/                         # Manual MCP testing notes
├── specs/001-tmux-log-copilot/   # Feature spec, data model, MCP schema
├── completions/                  # bash/zsh/fish completions
├── .github/workflows/            # CI (fmt, clippy, test)
├── .planning/codebase/           # Generated architecture maps
├── config.example.toml           # Sample ~/.config/logpilot/config.toml
├── Cargo.toml                    # Package, bin, deps (Rust 1.86+)
├── Cargo.lock                    # Locked deps
├── rust-toolchain.toml           # Pin channel 1.98 + rustfmt/clippy
├── Justfile                      # Dev recipes (test, mcp, watch, publish)
├── AGENTS.md / Claude.md         # Agent-oriented project facts
├── README.md / CONTRIBUTING.md
├── LICENSE
├── renovate.json                 # Dependency updates
├── autoresearch.*                # Local research artifacts
└── target/                       # Cargo build output (generated)
```

## Directory Purposes

**src/:**
- Purpose: All production Rust code for the `logpilot` library and `logpilot` binary
- Contains: module trees listed in `src/lib.rs` / `src/main.rs` (binary omits `observability`)
- Key files: `src/main.rs`, `src/lib.rs`, `src/error.rs`, `src/observability.rs`

**src/analyzer/:**
- Purpose: Anomaly detection after parse/cluster
- Contains: orchestrator + pattern/incident/alert engines
- Key files: `src/analyzer/mod.rs`, `src/analyzer/patterns.rs`, `src/analyzer/incidents.rs`, `src/analyzer/alerts.rs`

**src/buffer/:**
- Purpose: In-memory retention and SQLite persistence of high-severity logs
- Contains: ring, persistence, manager
- Key files: `src/buffer/mod.rs`, `src/buffer/ring.rs`, `src/buffer/persistence.rs`, `src/buffer/manager.rs`

**src/capture/:**
- Purpose: tmux process integration and session lifecycle
- Contains: command wrapper, FIFO pane capture, session repository
- Key files: `src/capture/mod.rs`, `src/capture/tmux.rs`, `src/capture/pane.rs`, `src/capture/session.rs`

**src/cli/:**
- Purpose: User-facing command implementations
- Contains: one file per subcommand plus `mod.rs`
- Key files: `src/cli/mod.rs`, `src/cli/watch.rs`, `src/cli/filter.rs`, `src/cli/summarize.rs`, `src/cli/ask.rs`, `src/cli/mcp.rs`, `src/cli/status.rs`

**src/mcp/:**
- Purpose: MCP resources/tools over stdio
- Contains: hand-rolled server; `rmcp_server.rs` present but not compiled (`mod.rs` comments it out)
- Key files: `src/mcp/mod.rs`, `src/mcp/server.rs`, `src/mcp/protocol.rs`, `src/mcp/resources.rs`, `src/mcp/data_store.rs`, `src/mcp/rmcp_server.rs`

**src/models/:**
- Purpose: Shared domain types (Session 1:N Pane 1:N LogEntry → Pattern → Incident → Alert)
- Contains: one type file per entity
- Key files: `src/models/mod.rs`, `src/models/log_entry.rs`, `src/models/session.rs`, `src/models/pane.rs`, `src/models/severity.rs`, `src/models/pattern.rs`, `src/models/incident.rs`, `src/models/alert.rs`

**src/pipeline/:**
- Purpose: Log processing primitives
- Contains: parser, structured formats, SimHash dedup, clustering; stub `Pipeline`
- Key files: `src/pipeline/mod.rs`, `src/pipeline/parser.rs`, `src/pipeline/formats.rs`, `src/pipeline/dedup.rs`, `src/pipeline/cluster.rs`

**tests/:**
- Purpose: Crate-level tests that import `logpilot::` (library) or spawn the release binary
- Contains: alerts, filter, pipeline integration, MCP protocol, integration/ capture+analyzer, fixtures
- Key files: `tests/test_alerts.rs`, `tests/test_filter.rs`, `tests/test_pipeline_integration.rs`, `tests/test_mcp_protocol.rs`, `tests/integration/test_analyzer.rs`, `tests/integration/test_capture.rs`, `tests/fixtures/mock_tmux.sh`

**docs/:**
- Purpose: Operator-facing protocol notes
- Contains: markdown
- Key files: `docs/MCP_TESTING.md`

**specs/001-tmux-log-copilot/:**
- Purpose: Product spec, data model, tasks, MCP JSON schema contract
- Contains: spec-kit style design docs
- Key files: `specs/001-tmux-log-copilot/spec.md`, `specs/001-tmux-log-copilot/data-model.md`, `specs/001-tmux-log-copilot/plan.md`, `specs/001-tmux-log-copilot/contracts/mcp-schema.json`

**completions/:**
- Purpose: Shell completion scripts packaged with the crate (`Cargo.toml` `include`)
- Contains: bash, zsh, fish
- Key files: `completions/logpilot.bash`, `completions/logpilot.zsh`, `completions/logpilot.fish`

**.github/workflows/:**
- Purpose: CI
- Contains: GitHub Actions YAML
- Key files: `.github/workflows/ci.yml`

**.planning/codebase/:**
- Purpose: Architecture/structure maps for planning (this document)
- Contains: markdown analyses
- Key files: `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`

**target/:**
- Purpose: Cargo build artifacts including `target/release/logpilot` required by MCP tests
- Contains: debug/release binaries and incremental compile cache
- Key files: `target/release/logpilot` (generated)

## Key File Locations

**Entry Points:**
- `src/main.rs`: CLI binary; clap `Commands` for watch/filter/summarize/ask/mcp-server/status
- `src/lib.rs`: Library root, `Config` TOML types, public module graph
- `src/cli/watch.rs`: Live capture + analyze loop
- `src/cli/mcp.rs`: Starts `McpServer::run_stdio`
- `src/mcp/server.rs`: JSON-RPC request loop
- `src/capture/pane.rs`: FIFO capture worker (first `LogEntry` producer)

**Configuration:**
- `config.example.toml`: Documented user config (buffer, patterns, alerts, mcp)
- `src/lib.rs` `Config::load()`: Reads `dirs::config_dir()/logpilot/config.toml` or `logpilot.toml`; defaults if missing
- `Cargo.toml`: crate metadata, rust-version 1.86, dependencies (tokio, clap, sqlx, tracing, …); `rmcp` commented out
- `rust-toolchain.toml`: toolchain channel 1.98
- Runtime data: `dirs::data_dir()/logpilot/logs.db` (watch persistence); FIFOs under `std::env::temp_dir()`

**Core Logic:**
- `src/analyzer/mod.rs`: `Analyzer::process_entry`
- `src/pipeline/parser.rs` / `src/pipeline/formats.rs` / `src/pipeline/dedup.rs` / `src/pipeline/cluster.rs`: parse and cluster
- `src/buffer/manager.rs`: dual-write memory/SQLite
- `src/capture/tmux.rs` / `src/capture/session.rs`: tmux attach and reconnect
- `src/mcp/data_store.rs`: in-memory MCP projection + global singleton
- `src/mcp/resources.rs`: `logpilot://session/{name}/...` URIs
- `src/error.rs`: error taxonomy

**Testing:**
- Inline `#[cfg(test)]` in most `src/**/*.rs` modules
- `tests/test_pipeline_integration.rs`: buffer persistence, MCP resource reads, dedup
- `tests/test_mcp_protocol.rs`: spawns `./target/release/logpilot`
- `tests/test_alerts.rs`, `tests/test_filter.rs`
- `tests/integration/test_analyzer.rs`, `tests/integration/test_capture.rs` (capture tests assume mock tmux)
- `Justfile` `test` / `ci` / `lint`; `.github/workflows/ci.yml`; `.pre-commit-config.yaml`

## Naming Conventions

**Files:**
- Module files: snake_case matching the type cluster (`session.rs`, `data_store.rs`, `rmcp_server.rs`)
- Integration tests: `test_<area>.rs` under `tests/`
- CLI: one file per subcommand matching clap command (`watch.rs`, `mcp.rs`)

**Directories:**
- Domain layers as crate modules: `analyzer`, `buffer`, `capture`, `cli`, `mcp`, `models`, `pipeline`
- Specs under `specs/001-<feature-slug>/`
- Tests: crate root `tests/` plus `tests/integration/` and `tests/fixtures/`

**Types:**
- PascalCase structs/enums (`LogEntry`, `SessionStatus`, `AlertType`)
- CLI arg structs `*Args` / `WatchOptions`
- Errors: `LogPilotError` + `Result<T>` alias

## Where to Add New Code

**New Feature:**
- Primary code: new submodule under the matching layer (`src/cli/` for user commands, `src/analyzer/` for detection, `src/mcp/` for AI tools/resources)
- Tests: unit tests in the same file under `#[cfg(test)]`; crate tests in `tests/test_<feature>.rs`; MCP contract updates in `specs/001-tmux-log-copilot/contracts/mcp-schema.json` and `docs/MCP_TESTING.md`

**New Component/Module:**
- Implementation: add `src/<layer>/<name>.rs` and `pub mod` in that layer’s `mod.rs`; if the binary must see it, also ensure `src/main.rs` module list stays in sync with `src/lib.rs` (they are duplicated)
- Register CLI in `src/main.rs` `Commands` and `src/cli/mod.rs`

**Utilities:**
- Shared helpers: `src/error.rs` for errors; `src/lib.rs` for config; `src/observability.rs` for structured metrics (currently unused); tmux sanitization belongs in `src/capture/tmux.rs`

## Special Directories

**target/:**
- Purpose: Cargo debug/release build products (`logpilot` binary, incremental artifacts)
- Generated: Yes
- Committed: No

**.planning/:**
- Purpose: Planning-time architecture/structure documents
- Generated: Yes (analysis artifacts)
- Committed: Depends on repo policy; currently local planning output

**specs/:**
- Purpose: Feature specification, data model, tasks, MCP contract
- Generated: No
- Committed: Yes

**docs/:**
- Purpose: Human MCP testing guide
- Generated: No
- Committed: Yes

**completions/:**
- Purpose: Shell completions shipped in crate `include`
- Generated: No
- Committed: Yes

**tests/fixtures/:**
- Purpose: Mock tmux helper for capture integration tests
- Generated: No
- Committed: Yes

**.github/:**
- Purpose: CI workflow
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-08-26*
