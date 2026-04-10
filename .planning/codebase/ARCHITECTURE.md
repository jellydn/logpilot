# LogPilot Architecture

**Project**: LogPilot - AI-Native tmux Log Copilot for Support Incident Tracking

**Version**: 0.1.0 | **Edition**: Rust 2021 | **Runtime**: Tokio (async)

## System Design Pattern

LogPilot follows a **Producer-Consumer Pipeline** architecture with real-time log analysis and AI context bridging:

```
tmux session panes
       ↓
   Capture layer (SessionRepository)
       ↓
   Log Entry Channel (mpsc)
       ↓
   Analyzer (pattern detection, deduplication, clustering)
       ↓
   Results: Patterns, Incidents, Alerts
       ↓
   Shared Data Store (SessionDataStore)
       ↓
   ├─ CLI output (watch command)
   ├─ MCP server (Claude Code integration)
   └─ SQLite persistence (high-severity events)
```

### Core Design Principles

- **Async-first**: Tokio-based concurrent processing
- **Thread-safe**: DashMap, Arc<RwLock> for concurrent access
- **Real-time analysis**: Streaming log processing without batching
- **AI-native**: MCP protocol for Claude Code context bridge
- **Separation of concerns**: Capture → Pipeline → Analysis → Output

## Component Layers

### Layer 1: Data Models (`src/models/`)

**Purpose**: Type-safe domain structures

- **LogEntry**: Individual log line with metadata (severity, service, fields)
- **Session/Pane**: tmux context (session name, pane IDs)
- **Severity**: Enum (Unknown, Info, Warn, Error, Fatal)
- **Pattern**: Deduplicated log signature with occurrence tracking
- **Incident**: Multi-pattern anomaly events with status (Open/Resolved)
- **Alert**: Alert notifications (type: ErrorRate, RecurringError, RestartLoop)

### Layer 2: Capture (`src/capture/`)

**Purpose**: Extract live logs from tmux panes

Components:
- **SessionRepository**: Main interface; spawns tmux capture goroutine
- **TmuxInterop**: Executes tmux CLI commands (list-sessions, capture-pane)
- **PaneCapture**: Per-pane capture state and sequence tracking
- **SessionCapture**: Session-level metadata and pane registry

Data Flow:
```
SessionRepository::new(log_tx)
  ↓ spawns tmux subprocess
  ↓ captures pane text via tmux capture-pane -p
  ↓ emits LogEntry → channel
```

### Layer 3: Pipeline (`src/pipeline/`)

**Purpose**: Transform raw text → structured, deduplicated logs

Components:
- **Parser**: Regex-based structured field extraction (log level, service, timestamp)
- **FormatParser**: Attempt JSON and logfmt parsing
- **ClusterEngine**: Hash-based deduplication; generates signatures
- **ClusterManager**: Tracks cluster membership (logs → clusters)
- **Deduplicator**: Prevents duplicate entry processing

Pipeline orchestration (in `Analyzer.process_entry()`):
```
1. FormatParser::try_parse_json()
   ↓ if fails → try_parse_logfmt()
2. LogParser::parse() - regex extraction
3. ClusterEngine::cluster() - generate signature, detect new cluster
4. ClusterManager::add_to_cluster() - register membership
5. PatternTracker::track() - frequency analysis
6. IncidentDetector::create_incident() - if threshold exceeded
```

### Layer 4: Analyzer (`src/analyzer/`)

**Purpose**: Real-time anomaly detection and incident correlation

Components:
- **Analyzer**: Orchestrator; manages all sub-analyzers
- **PatternTracker**: Windowed frequency counter (5-min window default)
- **IncidentDetector**: Creates Incident when pattern exceeds threshold
- **AlertEvaluator**: Detects error rate spikes, restart loops
- **ErrorRateCalculator**: Per-minute error count tracker

Key algorithms:
- **Deduplication**: Severity + first 100 chars normalized → signature
- **Clustering**: Track unique signatures; emit new clusters as patterns
- **Incident detection**: Pattern count in time window > threshold → Incident
- **Alert rules**:
  - Recurring Error: 5+ errors in 60s window
  - Error Rate: >10 errors/min
  - Restart Loop: 5+ errors in 30s window

### Layer 5: Shared Data Store (`src/mcp/data_store.rs`)

**Purpose**: Thread-safe live data access for MCP server

- **SessionDataStore**: Global singleton (DashMap<session_name, SessionData>)
- **SessionData**: Vec<LogEntry>, Vec<Pattern>, Vec<Incident>, Vec<Alert>
- **Bounded**: Keeps last 10k log entries per session
- **Updated**: Timestamp tracking for MCP resources/list

### Layer 6: MCP Server (`src/mcp/`)

**Purpose**: Expose live logs/incidents via Model Context Protocol to Claude Code

Components:
- **McpServer**: JSON-RPC handler (stdio transport)
- **Protocol**: JsonRpcRequest/Response, JSON-RPC 2.0 spec
- **ResourceHandler**: Convert session data → MCP resource URIs
- **Resources supported**:
  - `logpilot://session/{name}/summary` - incident summary
  - `logpilot://session/{name}/entries` - recent log entries
  - `logpilot://session/{name}/patterns` - detected patterns
  - `logpilot://session/{name}/incidents` - active incidents
  - `logpilot://session/{name}/alerts` - recent alerts

### Layer 7: CLI (`src/cli/`)

**Purpose**: User-facing commands

Commands:
- **watch**: Attach to tmux session; live log + analysis display
- **summarize**: Aggregate patterns/incidents for time window
- **ask**: AI-formatted query about logs (placeholder for LLM integration)
- **mcp**: Start MCP server for Claude Code integration
- **status**: Show monitored sessions and stats

### Layer 8: Persistence (`src/buffer/`)

**Purpose**: Long-term storage of high-severity events

Components:
- **Ring**: In-memory circular buffer (fixed size, FIFO eviction)
- **Manager**: Coordinates ring + SQLite persistence
- **Persistence**: SQLite schema (entries, patterns, incidents tables)

**Note**: Currently infrastructure not wired to CLI; placeholder for future use.

## Data Flow - Complete Request Cycle

### Watch Command Flow

```
1. CLI: logpilot watch my-session
   ├─ Create: LogEntry channel (mpsc)
   ├─ Create: Analyzer (with patterns, incidents, alerts)
   ├─ Create: SessionDataStore (global singleton)
   ├─ Create: SessionRepository (tmux capture)
   └─ Create: AlertEvaluator

2. SessionRepository spawns tmux capture loop:
   ├─ Run: tmux list-sessions / capture-pane -p
   ├─ Poll every 100ms for new pane text
   ├─ Emit: LogEntry (with sequence, timestamp)
   └─ Send: log_tx channel

3. Watch command reads channel:
   ├─ Recv: LogEntry from log_rx
   ├─ Call: Analyzer::process_entry()
   │   ├─ Parse (JSON, logfmt, regex)
   │   ├─ Cluster (dedup, detect new)
   │   ├─ Track patterns
   │   ├─ Detect incidents
   │   └─ Return: AnalysisResult
   ├─ Evaluate: AlertEvaluator::check()
   │   └─ Emit: Alert via broadcast channel
   ├─ Update: SessionDataStore::add_entry()
   ├─ Display: Live TUI (via crossterm)
   └─ Loop until 'q' key pressed
```

### MCP Server Flow

```
1. CLI: logpilot mcp-server
   ├─ Create: McpServer
   ├─ Get reference: SessionDataStore (global)
   └─ Loop: Read JSON-RPC from stdin

2. Receive: JSON-RPC request
   ├─ Method: "initialize" → return capabilities
   ├─ Method: "resources/list" → return supported URIs
   ├─ Method: "resources/read" → query SessionDataStore
   │   ├─ Parse URI (e.g., logpilot://session/my-session/entries)
   │   ├─ Fetch: SessionData from DashMap
   │   ├─ Format as JSON (uri, mimeType, text)
   │   └─ Return: JSON-RPC success
   └─ Reply: JSON-RPC response to stdout
```

## Key Abstractions & Interfaces

### Error Handling

- **LogPilotError**: Custom error enum with variants
  - Io, Tmux, Database, Config, SessionNotFound
  - Uses `thiserror` for Display + From traits
- **Result<T>**: Type alias for `Result<T, LogPilotError>`
- Pattern: Errors propagate with `?` operator

### Configuration

- **Config struct**: Root configuration (buffer, patterns, alerts, mcp)
- **Load**: From `~/.config/logpilot/config.toml` or defaults
- **Defaults**: 30-min buffer, 100MB memory, ErrorRate threshold 10/min

### Async Patterns

- **tokio::spawn**: Background tasks (SessionRepository, display loop)
- **mpsc channels**: Single producer (tmux), multiple consumers (analyzer, display)
- **broadcast channels**: Alerts → multiple subscribers (display, MCP)
- **RwLock<T>**: Read-heavy analyzer state
- **Mutex<T>**: Quit signal coordination

## Entry Points

1. **main.rs**: CLI dispatch
   - Parses args (subcommand, flags)
   - Routes to appropriate handler
   - Error handling + exit codes

2. **cli/watch.rs**: Watch command entry
   - Spawns all components
   - Coordinates channels and threads
   - Runs event loop

3. **cli/mcp.rs**: MCP server entry
   - Initializes McpServer
   - Loops over stdin JSON-RPC requests
   - Writes responses to stdout

4. **cli/summarize.rs, ask.rs, status.rs**: Query/reporting

## Configuration Schema (config.toml)

```toml
[buffer]
duration_minutes = 30              # Rolling window
max_memory_mb = 100                # In-memory limit
persist_severity = ["ERROR", "FATAL"]
persist_path = "~/.logpilot"

[patterns]
custom_patterns = []               # User regex list

[alerts]
recurring_error_window_seconds = 60
recurring_error_threshold = 5
restart_loop_window_seconds = 30
error_rate_threshold_per_minute = 10

[mcp]
enabled = true
transport = "stdio"
```

## External Dependencies (Key)

- **tokio**: Async runtime, channels, time utilities
- **clap**: CLI argument parsing
- **serde/serde_json**: Serialization
- **sqlx**: SQLite database abstraction
- **regex**: Pattern matching
- **dashmap**: Concurrent HashMap
- **crossterm**: Terminal UI interactions
- **chrono**: Date/time
- **tracing**: Structured logging
- **uuid**: Unique identifiers
