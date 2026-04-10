# AGENTS.md - LogPilot Development

## Quick Commands

```bash
# Development
just test          # cargo test && cargo clippy
just ci           # fmt + test (parallel)
just lint         # cargo clippy --all-features -- -D warnings
just fmt          # cargo fmt -- --check

# Run
cargo run -- watch <session>   # Start a watch session
cargo run -- mcp-server     # Start MCP server
cargo run -- status         # Show monitored sessions
```

## Key Facts

- **Rust**: Min 1.86 (from `.github/workflows/ci.yml`)
- **MCP**: Uses `rmcp` crate (the official Rust MCP SDK), not legacy custom impl
- **Pre-commit**: Runs `cargo fmt`, `cargo test`, `cargo clippy` (install via `pre-commit install`)
- **CLI subcommands**: `watch`, `filter`, `summarize`, `ask`, `mcp-server`, `status` (see `src/main.rs`)

## Testing Quirk

MCP protocol tests (`tests/test_mcp_protocol.rs`) require pre-built release binary:

```bash
cargo build --release
# Then run tests
cargo test --test test_mcp_protocol
```

## Architecture Overview

- `src/analyzer` - Anomaly detection, pattern analysis
- `src/buffer` - Ring buffer + SQLite persistence
- `src/capture` - tmux integration
- `src/cli` - CLI command handlers
- `src/mcp` - MCP server (rmcp-based)
- `src/models` - Data structures
- `src/pipeline` - Log processing (parse, dedup, cluster)

## MCP Testing

See `docs/MCP_TESTING.md` for manual JSON-RPC testing over stdio.

## Configuration

Config file: `~/.config/logpilot/config.toml` (see `config.example.toml`)
