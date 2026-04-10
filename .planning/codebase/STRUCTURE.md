# Codebase Structure

**Analysis Date:** 2025-04-10

## Directory Layout

```
2026-04-10-tmux-mcp/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── error.rs             # Error types
│   ├── observability.rs     # Tracing/logging setup
│   ├── analyzer/            # Pattern/incident detection
│   ├── buffer/              # In-memory storage + persistence
│   ├── capture/             # tmux interaction
│   ├── cli/                 # Subcommand handlers
│   ├── mcp/                 # MCP server protocol
│   ├── models/              # Domain types
│   └── pipeline/            # Log processing pipeline
├── .github/workflows/       # CI configuration
│   └── ci.yml               # GitHub Actions workflow
├── Cargo.toml               # Package manifest
├── Cargo.lock               # Dependency lock
└── .planning/codebase/      # This documentation
```

## Directory Purposes

**src/analyzer/:**
- Purpose: Log analysis and anomaly detection
- Contains: Pattern matching, incident tracking, alerting
- Key files: `src/analyzer/patterns.rs`, `src/analyzer/incidents.rs`, `src/analyzer/alerts.rs`

**src/buffer/:**
- Purpose: Log storage and retrieval
- Contains: Ring buffer, persistence layer, manager
- Key files: `src/buffer/ring.rs`, `src/buffer/persistence.rs`, `src/buffer/manager.rs`

**src/capture/:**
- Purpose: tmux session and pane interaction
- Contains: Session management, pane capture, tmux command wrappers
- Key files: `src/capture/session.rs`, `src/capture/pane.rs`, `src/capture/tmux.rs`

**src/cli/:**
- Purpose: Command-line subcommand implementations
- Contains: Watch, summarize, ask, mcp-server, status handlers
- Key files: `src/cli/watch.rs`, `src/cli/summarize.rs`, `src/cli/ask.rs`, `src/cli/mcp.rs`, `src/cli/status.rs`

**src/mcp/:**
- Purpose: Model Context Protocol server
- Contains: JSON-RPC protocol, server implementation, resource handlers
- Key files: `src/mcp/server.rs`, `src/mcp/protocol.rs`, `src/mcp/resources.rs`

**src/models/:**
- Purpose: Domain models and data structures
- Contains: LogEntry, Incident, Alert, Pattern, Severity, etc.
- Key files: `src/models/log_entry.rs`, `src/models/incident.rs`, `src/models/alert.rs`, `src/models/pattern.rs`

**src/pipeline/:**
- Purpose: Log processing pipeline
- Contains: Parser, format handlers, clustering, deduplication
- Key files: `src/pipeline/parser.rs`, `src/pipeline/cluster.rs`, `src/pipeline/dedup.rs`, `src/pipeline/formats.rs`

## Key File Locations

**Entry Points:**
- `src/main.rs`: CLI binary entry point
- `src/lib.rs`: Library crate entry point

**Configuration:**
- `Cargo.toml`: Package and dependency configuration
- `.github/workflows/ci.yml`: CI/CD configuration
- Config file (runtime): `~/.config/logpilot/config.toml`

**Core Logic:**
- `src/error.rs`: Error types and handling
- `src/observability.rs`: Logging and tracing setup

**Testing:**
- Tests co-located with source files (Rust convention)
- `#[cfg(test)]` modules in each file
- Dev dependencies in `Cargo.toml`

## Naming Conventions

**Files:**
- `snake_case.rs`: Module files (`log_entry.rs`, `session.rs`)
- `mod.rs`: Module index files

**Directories:**
- `snake_case/`: Module directories (`buffer/`, `capture/`)

**Types:**
- PascalCase: Structs, Enums, Traits (`LogEntry`, `LogPilotError`)

**Functions/Variables:**
- snake_case: Functions, variables, modules (`add_entry`, `session_name`)

## Where to Add New Code

**New Feature:**
- Primary code: Add to appropriate `src/{module}/` directory
- CLI command: Add to `src/cli/{command}.rs` and register in `src/cli/mod.rs`
- Tests: Co-located in same file under `#[cfg(test)]`

**New Component/Module:**
- Implementation: Create `src/{new_module}/mod.rs`
- Export: Add `pub mod {new_module};` to `src/lib.rs`

**Utilities:**
- Shared helpers: Add to existing module or create `src/util/` if needed

## Special Directories

**.cargo_cache/:**
- Purpose: Local cargo registry cache
- Generated: Yes (by cargo)
- Committed: No (typically gitignored)

**.planning/:**
- Purpose: Project planning documentation
- Generated: No (manual)
- Committed: Yes

---

*Structure analysis: 2025-04-10*
