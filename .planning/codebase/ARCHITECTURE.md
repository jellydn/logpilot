# Architecture

**Analysis Date:** 2025-04-10

## Pattern Overview

**Overall:** Layered Architecture with Pipeline Processing

**Key Characteristics:**
- Modular design with clear separation of concerns
- Pipeline pattern for log processing (parse → cluster → dedup)
- Async-first with Tokio runtime
- Repository pattern for data access
- Observer pattern for session monitoring

## Layers

**CLI Layer:**
- Purpose: Command-line interface and user interaction
- Location: `src/cli/`
- Contains: Subcommand handlers (`watch`, `summarize`, `ask`, `mcp-server`, `status`)
- Depends on: All other layers
- Used by: End users via command line

**Capture Layer:**
- Purpose: tmux session/pane interaction and log ingestion
- Location: `src/capture/`
- Contains: Session management, pane capture, tmux command execution
- Depends on: tokio (process), models
- Used by: CLI watch command

**Pipeline Layer:**
- Purpose: Log parsing, formatting, clustering, and deduplication
- Location: `src/pipeline/`
- Contains: Parser, format handlers, cluster manager, deduplicator
- Depends on: regex, models
- Used by: Buffer manager

**Buffer Layer:**
- Purpose: In-memory log storage with persistence
- Location: `src/buffer/`
- Contains: Ring buffer, persistence manager, buffer manager
- Depends on: pipeline, models, sqlx
- Used by: Capture, Analyzer

**Analyzer Layer:**
- Purpose: Pattern detection, incident detection, alerting
- Location: `src/analyzer/`
- Contains: Pattern matcher, incident repository, alert generator
- Depends on: buffer, models
- Used by: CLI commands

**MCP Layer:**
- Purpose: AI assistant integration via Model Context Protocol
- Location: `src/mcp/`
- Contains: JSON-RPC server, protocol types, resource handlers
- Depends on: models, tokio (sync)
- Used by: External AI tools

**Models Layer:**
- Purpose: Domain models and data types
- Location: `src/models/`
- Contains: LogEntry, Incident, Alert, Pattern, Severity, etc.
- Depends on: serde, chrono, uuid
- Used by: All layers

## Data Flow

**Log Capture Flow:**
1. `src/cli/watch.rs` → initiates watch with session/pane options
2. `src/capture/session.rs` → manages tmux session lifecycle
3. `src/capture/pane.rs` → captures raw pane output
4. `src/buffer/manager.rs` → receives raw lines
5. `src/pipeline/parser.rs` → parses log entries (timestamp, severity, service)
6. `src/pipeline/cluster.rs` → groups similar entries by signature
7. `src/pipeline/dedup.rs` → removes near-duplicates
8. `src/buffer/ring.rs` → stores in rolling buffer
9. `src/buffer/persistence.rs` → persists high-severity entries
10. `src/analyzer/` → detects patterns, incidents, generates alerts

**MCP Data Flow:**
1. External AI tool → stdin JSON-RPC request
2. `src/mcp/server.rs` → parses and routes request
3. `src/mcp/resources.rs` → handles resource URI resolution
4. Session data → retrieved from in-memory state
5. JSON response → written to stdout

**State Management:**
- In-memory: DashMap for concurrent access, Arc<RwLock<>> for MCP state
- Persistent: SQLite via sqlx for incidents/alerts
- Config: TOML file with Config struct

## Key Abstractions

**LogEntry:**
- Purpose: Core domain model representing a parsed log line
- Examples: `src/models/log_entry.rs`
- Pattern: Rich domain model with metadata (timestamp, severity, service, raw content)

**Pipeline:**
- Purpose: Composable log processing stages
- Examples: `src/pipeline/mod.rs`, `src/pipeline/parser.rs`
- Pattern: Iterator-like chain with parse → cluster → dedup stages

**Repository:**
- Purpose: Data access abstraction for persistent storage
- Examples: `src/analyzer/incidents.rs`, `src/buffer/persistence.rs`
- Pattern: Async trait-based repository with SQLite backend

**McpServer:**
- Purpose: AI integration endpoint
- Examples: `src/mcp/server.rs`
- Pattern: JSON-RPC 2.0 server with resource-based API

## Entry Points

**CLI Binary:**
- Location: `src/main.rs`
- Triggers: User command execution
- Responsibilities: Parse CLI args, dispatch to subcommand handlers

**Library:**
- Location: `src/lib.rs`
- Triggers: External crate usage (`use logpilot::*`)
- Responsibilities: Export public API modules and types

**MCP Server:**
- Location: `src/mcp/server.rs` (started via `src/cli/mcp.rs`)
- Triggers: `logpilot mcp-server` command
- Responsibilities: Handle JSON-RPC requests, provide session data to AI tools

## Error Handling

**Strategy:** Thiserror-based enum with anyhow for propagation

**Patterns:**
- `LogPilotError` enum covers all error variants (Io, Tmux, Parse, Database, Config, Mcp, etc.)
- `Result<T>` type alias for `std::result::Result<T, LogPilotError>`
- `?` operator propagation with `#[from]` conversions
- Helper constructors: `LogPilotError::tmux()`, `LogPilotError::parse()`, etc.

## Cross-Cutting Concerns

**Logging:** Tracing framework with structured events

**Validation:** Regex-based log parsing with fallback patterns

**Authentication:** None - local tool only

---

*Architecture analysis: 2025-04-10*
