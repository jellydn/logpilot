# External Integrations

**Analysis Date:** 2025-04-10

## APIs & External Services

**tmux Integration:**
- Local tmux process - Primary data source for log capture
- Shell commands via `tmux` binary (`tmux capture-pane`, `tmux show-options`)
- No SDK - Direct command execution

**MCP (Model Context Protocol):**
- JSON-RPC 2.0 over stdio
- Provides AI assistant context exchange
- Custom URI scheme: `logpilot://session/{name}/...`

## Data Storage

**Databases:**
- SQLite - Persistent storage for incidents and alerts
- Connection: Via `sqlx` with runtime-tokio
- Location: Platform data directory (`~/.local/share/logpilot/` or equivalent)

**File Storage:**
- Local filesystem only
- Config: `~/.config/logpilot/config.toml`
- Data: `~/.local/share/logpilot/`

**Caching:**
- In-memory ring buffer (`src/buffer/ring.rs`)
- Time-based expiration (default 30 minutes)
- Size-limited by memory (default 100MB)

## Authentication & Identity

**Auth Provider:**
- None - Local-only tool
- No authentication mechanisms

## Monitoring & Observability

**Error Tracking:**
- Tracing framework for structured logging
- Console output via `tracing-subscriber`

**Logs:**
- Structured logging via `tracing` crate
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE

## CI/CD & Deployment

**Hosting:**
- GitHub (source repository)
- No deployment target - CLI tool distributed via cargo/git

**CI Pipeline:**
- GitHub Actions (`.github/workflows/ci.yml`)
- Jobs: test, fmt, clippy (parallel)
- Rust version: 1.75 (pinned)

## Environment Configuration

**Required env vars:**
- None strictly required
- Uses default config if no config file present

**Optional env vars:**
- `RUST_LOG` - Tracing/log level control (standard tracing-subscriber behavior)

**Secrets location:**
- No secrets required - local tool only

## Webhooks & Callbacks

**Incoming:**
- None - CLI tool

**Outgoing:**
- None - No external callbacks

---

*Integration audit: 2025-04-10*
