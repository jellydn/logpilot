# LogPilot Development Guidelines

CLI tool: `cargo run -- --help`

## Commands

```bash
cargo test && cargo clippy    # Local verification (order matters: test first)
cargo fmt -- --check          # Format check
```

CI runs: fmt, test, clippy (parallel jobs). All must pass.

## Project

- **Binary**: `logpilot` (src/main.rs)
- **Lib**: `logpilot` crate (src/lib.rs)
- **Features**: tmux session capture, log parsing, MCP server mode

## Architecture

- `src/cli/` - CLI subcommands (watch, summarize, ask, mcp-server, status)
- `src/capture/` - tmux interaction (session.rs, pane.rs, tmux.rs)
- `src/analyzer/` - alert and incident detection (alerts.rs, incidents.rs, patterns.rs)
- `src/models/` - data models (log_entry, incident, alert, pattern, severity, etc.)
- `src/mcp/` - MCP server protocol (server.rs, protocol.rs)
- `src/pipeline/` - log parsing and deduplication

MCP server starts but needs full implementation for live data integration.

## Setup

- Config: `~/.config/logpilot/config.toml` (see `config.example.toml`)
- Requires: tmux installed, Rust 1.75+

## Testing

```bash
cargo test --all-features        # Run all tests
cargo test --all-features -- --nocapture  # Debug output
```
