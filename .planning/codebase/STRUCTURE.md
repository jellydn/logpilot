# Directory Structure - LogPilot

## Top-Level Layout

```
logpilot/
├── src/                    # All Rust source code
├── tests/                  # Integration and contract tests
│   ├── contract/           # MCP protocol contract tests
│   ├── fixtures/           # Shared test data
│   └── integration/        # End-to-end integration tests
├── docs/                   # Project documentation
├── specs/                  # Feature specifications
│   └── 001-tmux-log-copilot/
├── completions/            # Shell completion scripts
├── Cargo.toml              # Package manifest and dependencies
├── Cargo.lock              # Pinned dependency versions
├── rust-toolchain.toml     # Enforced Rust 1.86 + components
├── Justfile                # Development task runner recipes
├── renovate.json           # Automated dependency update config
└── .github/workflows/      # CI/CD pipelines
```

## Source Tree (`src/`)

```
src/
├── main.rs                 # Binary entry point — Tokio runtime, CLI dispatch
├── lib.rs                  # Library root — public re-exports
├── error.rs                # LogPilotError enum, Result<T> alias
├── observability.rs        # tracing instrumentation and self-metrics
│
├── models/                 # Pure data types — no logic
│   ├── mod.rs              # Re-exports all model types
│   ├── log_entry.rs        # LogEntry struct (core record)
│   ├── severity.rs         # Severity enum with FromStr + Display
│   ├── session.rs          # SessionInfo, SessionStatus enum
│   ├── pane.rs             # PaneInfo struct
│   ├── pattern.rs          # Pattern, ClusterInfo structs
│   ├── incident.rs         # Incident, IncidentStatus enum
│   └── alert.rs            # Alert, AlertRule structs
│
├── capture/                # tmux integration — log ingestion
│   ├── mod.rs              # Module re-exports
│   ├── tmux.rs             # Subprocess builder + input validation
│   ├── session.rs          # Session lifecycle management
│   └── pane.rs             # Pane output streaming via FIFO
│
├── pipeline/               # Log processing pipeline
│   ├── mod.rs              # Pipeline orchestration
│   ├── parser.rs           # Regex-based field extraction (500+ lines, 18 tests)
│   ├── formats.rs          # Multi-format detection (JSON, logfmt, standard)
│   ├── cluster.rs          # Error pattern clustering
│   └── dedup.rs            # SimHash-based deduplication
│
├── buffer/                 # Storage layer
│   ├── mod.rs              # Module re-exports
│   ├── manager.rs          # Buffer lifecycle, rolling 10k-entry window
│   ├── persistence.rs      # SQLite async writes via sqlx
│   └── ring.rs             # Ring buffer implementation
│
├── analyzer/               # Analysis engine (⚠ dead_code, not yet wired)
│   ├── mod.rs              # Analyzer struct — Arc<RwLock> state
│   ├── patterns.rs         # PatternTracker — frequency and recency
│   ├── incidents.rs        # Incident correlation and lifecycle
│   └── alerts.rs           # Threshold evaluation
│
├── mcp/                    # Model Context Protocol server
│   ├── mod.rs              # Module re-exports
│   ├── server.rs           # JSON-RPC 2.0 stdio request loop
│   ├── protocol.rs         # MCP message types (Request, Response, Error)
│   ├── resources.rs        # Resource URI → data handler mapping
│   └── data_store.rs       # Arc<DashMap> shared live session store
│
└── cli/                    # CLI command handlers
    ├── mod.rs              # Subcommand enum dispatch
    ├── watch.rs            # `logpilot watch` — live monitoring loop
    ├── summarize.rs        # `logpilot summarize` — session summary
    ├── ask.rs              # `logpilot ask` — AI query interface
    ├── mcp.rs              # `logpilot mcp-server` — start MCP server
    └── status.rs           # `logpilot status` — session status display
```

## Key File Locations

| Purpose | File |
|---------|------|
| CLI entry point | `src/main.rs` |
| Public library API | `src/lib.rs` |
| Error types | `src/error.rs` |
| Core log record | `src/models/log_entry.rs` |
| Log parser (most logic) | `src/pipeline/parser.rs` |
| tmux validation | `src/capture/tmux.rs` |
| MCP server loop | `src/mcp/server.rs` |
| MCP message types | `src/mcp/protocol.rs` |
| SQLite persistence | `src/buffer/persistence.rs` |
| Shared data store | `src/mcp/data_store.rs` |
| Buffer lifecycle | `src/buffer/manager.rs` |

## Naming Conventions

### Files
- Lowercase with underscores: `log_entry.rs`, `data_store.rs`
- Module root: `mod.rs` for sub-modules
- Feature named: file name mirrors the primary type it defines

### Types
- Structs / Enums: `PascalCase` — `LogEntry`, `Severity`, `BufferManager`
- Traits: `PascalCase` — follows std conventions
- Config structs: `<Domain>Config` — `BufferConfig`, `McpConfig`

### Functions & Methods
- `snake_case` throughout
- Constructors: `new()`, `new_in_memory()`, `with_persistence()`
- Boolean queries: `is_*`, `has_*` prefix

### Modules
- Single responsibility: each module owns one domain
- `mod.rs` declares sub-modules and re-exports public surface

## Test Locations

```
tests/
├── contract/               # MCP JSON-RPC protocol conformance
├── fixtures/               # Shared log samples, expected outputs
└── integration/            # Full pipeline tests (capture → MCP)

src/**/                     # Inline unit tests via #[cfg(test)] mod tests
```

## Configuration Paths (Runtime)

| Resource | Path |
|----------|------|
| User config | `~/.config/logpilot/config.toml` |
| Fallback config | `./logpilot.toml` |
| SQLite database | `{data_dir}/logpilot/logs.db` |
| Shell completions | `completions/{bash,zsh,fish}` |
