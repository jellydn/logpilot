# External Integrations - LogPilot

## MCP (Model Context Protocol)

### Overview
LogPilot implements a full MCP server to expose tmux log data to AI assistants (Claude Code, Codex, etc.).

### Transport
- **Protocol**: JSON-RPC 2.0 over stdio (stdin/stdout)
- **Mode**: Synchronous request/response on a single thread
- **Entry point**: `src/mcp/server.rs`

### Capabilities Exposed
- `resources/list` - List available resource URIs
- `resources/read` - Read resource content by URI
- `initialize` / `initialized` - MCP handshake
- `tools/list` - List available tools
- `ping` - Liveness check

### Resource URIs
Defined in `src/mcp/resources.rs`:
- `logpilot://sessions` - Active tmux sessions list
- `logpilot://session/{name}/logs` - Recent log entries for a session
- `logpilot://session/{name}/errors` - ERROR/FATAL log entries
- `logpilot://session/{name}/patterns` - Detected log patterns
- `logpilot://session/{name}/incidents` - Active incidents
- `logpilot://session/{name}/alerts` - Alert state

### Protocol Compliance
- Preserves `id` field in JSON-RPC responses (required for MCP compliance)
- Handles both integer and string request IDs
- Returns proper JSON-RPC error objects on failure

---

## tmux Integration

### Overview
LogPilot monitors tmux sessions and panes in real-time to capture log output.

### Integration Points (`src/capture/tmux.rs`)
- **Command builder**: Wraps `tmux` subprocess calls with safe argument construction
- **Validation**: Strict allowlist-based input sanitization for session/pane targets
  - Rejects shell metacharacters: `;`, `|`, `&`, `$`, `` ` ``, `>`, `<`, `(`, `)`, `{`, `}`, `[`, `]`, `*`, `?`, `\`, `'`, `"`
  - Prevents path traversal (`..`)
  - Regex-based allowlist: alphanumeric, `-`, `_`, `.`, `:`, `@`
- **Session listing**: `tmux list-sessions -F "#{session_name}:#{session_windows}:..."`
- **Pane capture**: `tmux capture-pane` with FIFO output
- **Live monitoring**: Continuous pane output capture

### Session Lifecycle (`src/capture/session.rs`)
- Session creation, attachment, and cleanup
- Status tracking: `SessionStatus` enum

### Pane I/O (`src/capture/pane.rs`)
- FIFO-based pane output streaming
- Line-by-line log entry ingestion into the pipeline

---

## SQLite Persistence

### Overview
High-severity log entries (ERROR, FATAL) are persisted to a local SQLite database.

### Configuration
- **Location**: User data directory via `dirs` crate
- **Pool size**: Max 5 concurrent connections
- **Driver**: `sqlx` with async SQLite backend

### Schema (`src/buffer/persistence.rs`)
```sql
CREATE TABLE IF NOT EXISTS log_entries (
    id           TEXT PRIMARY KEY,  -- UUID v4
    pane_id      TEXT NOT NULL,
    sequence     INTEGER NOT NULL,
    timestamp    TEXT,              -- ISO8601 or NULL
    severity     TEXT NOT NULL,
    service      TEXT,              -- nullable
    raw_content  TEXT NOT NULL,
    parsed_fields TEXT NOT NULL,    -- JSON blob
    received_at  TEXT NOT NULL      -- ISO8601 insertion time
);
```

### Persistence Policy
- Only ERROR and FATAL severity logs are persisted (configurable via `persist_severity`)
- In-memory DashMap holds all log levels for the rolling buffer window

---

## Log Format Support

### Multi-Format Parsing (`src/pipeline/formats.rs`, `src/pipeline/parser.rs`)
LogPilot parses several log formats using pre-compiled lazy-static regexes:

| Format | Example |
|--------|---------|
| ISO 8601 timestamp | `2024-01-15T10:30:00Z INFO [service] message` |
| Standard timestamp | `2024-01-15 10:30:00 ERROR Something failed` |
| Syslog-style | `Jan 15 10:30:00 hostname service[pid]: message` |
| Bracket service | `[service-name] ERROR message` |
| Key=value fields | `level=error service=api msg="request failed"` |
| JSON log lines | `{"level":"error","msg":"...","service":"..."}` |
| logfmt | `level=info ts=2024-01-15T10:30:00Z msg="started"` |

### Extracted Fields
- `severity`: Mapped to `Severity` enum (Fatal, Error, Warn, Info, Debug, Unknown)
- `timestamp`: Parsed to `chrono::DateTime<Utc>`
- `service`: Extracted from bracket notation or key=value
- `parsed_fields`: HashMap of all key=value pairs found in the line

---

## Anomaly Detection Pipeline

### Pattern Clustering (`src/pipeline/cluster.rs`)
- Groups similar error messages by structural similarity
- Deduplicates repetitive errors
- Tracks cluster frequency and first/last seen timestamps

### Deduplication (`src/pipeline/dedup.rs`)
- Fingerprints log entries to suppress exact duplicates
- Configurable dedup window

### Analyzer (`src/analyzer/`)
- Pattern tracking across sessions
- Incident lifecycle management
- Alert threshold evaluation
- Currently marked `#![allow(dead_code)]` — infrastructure in place, not yet wired to CLI

---

## AI Context Exchange

### How AI Tools Consume LogPilot Data
1. AI assistant starts LogPilot MCP server: `logpilot mcp-server`
2. Server reads from `stdin`, writes responses to `stdout`
3. AI sends `resources/list` to discover available data URIs
4. AI reads `logpilot://session/{name}/errors` to get recent errors
5. AI uses log context to assist with debugging or incident analysis

### Data Store (`src/mcp/data_store.rs`)
- Thread-safe `Arc<DashMap>` store shared between the capture pipeline and MCP server
- Live session data flows from tmux capture → pipeline → data store → MCP resources
