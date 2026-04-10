# Contributing to LogPilot

Thank you for your interest in contributing to LogPilot! This document provides guidelines and instructions for setting up your development environment.

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- tmux installed and running
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/jellydn/logpilot
cd logpilot

# Build debug version
cargo build

# Build release version
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_error_rate_threshold_alert

# Run integration tests only
cargo test --test test_alerts
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check all targets
cargo check --all-targets
```

## Project Structure

```
logpilot/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── error.rs          # Error types
│   ├── analyzer/         # Anomaly detection
│   │   ├── alerts.rs     # Alert triggering
│   │   ├── incidents.rs  # Incident clustering
│   │   ├── patterns.rs   # Pattern detection
│   │   └── mod.rs        # Analyzer orchestration
│   ├── buffer/           # Log storage
│   │   ├── ring.rs       # In-memory ring buffer
│   │   ├── persistence.rs # SQLite storage
│   │   ├── manager.rs    # Buffer lifecycle
│   │   └── mod.rs
│   ├── capture/          # tmux integration
│   │   ├── tmux.rs       # tmux commands
│   │   ├── session.rs    # Session management
│   │   ├── pane.rs       # Pane capture
│   │   └── mod.rs
│   ├── cli/              # CLI commands
│   │   ├── watch.rs      # Watch command
│   │   ├── summarize.rs  # Summarize command
│   │   ├── ask.rs        # Ask command
│   │   ├── status.rs     # Status command
│   │   ├── mcp.rs        # MCP server command
│   │   └── mod.rs
│   ├── mcp/              # MCP protocol
│   │   ├── protocol.rs   # JSON-RPC types
│   │   ├── server.rs     # MCP server
│   │   ├── resources.rs  # Resource handlers
│   │   └── mod.rs
│   ├── models/           # Data structures
│   │   ├── log_entry.rs
│   │   ├── session.rs
│   │   ├── pane.rs
│   │   ├── pattern.rs
│   │   ├── incident.rs
│   │   ├── alert.rs
│   │   ├── severity.rs
│   │   └── mod.rs
│   └── pipeline/         # Log processing
│       ├── parser.rs     # Log parsing
│       ├── formats.rs    # Structured formats
│       ├── dedup.rs      # Deduplication
│       ├── cluster.rs    # Clustering
│       └── mod.rs
├── tests/
│   ├── integration/    # Integration tests
│   ├── contract/         # Contract tests
│   └── fixtures/         # Test fixtures
├── specs/                # Design documents
├── benches/              # Benchmarks
├── Cargo.toml
├── README.md
└── CONTRIBUTING.md
```

## Architecture Principles

### 1. Local-First
- All processing happens locally
- No cloud dependencies or external APIs
- SQLite for persistence, not external databases

### 2. Real-Time Performance
- <2s latency from capture to analysis
- Streaming pipeline with async/await
- O(1) operations for hot paths

### 3. CLI-Native
- Text I/O, no GUI
- Unix philosophy: do one thing well
- Pipe-friendly output

### 4. AI Context Bridge
- MCP protocol for Claude/Codex integration
- Token-aware summaries
- Structured JSON output

## Adding New Features

### Adding a CLI Command

1. Create file in `src/cli/<command>.rs`
2. Add module export to `src/cli/mod.rs`
3. Add subcommand variant to `Commands` enum in `src/main.rs`
4. Wire up handler in `main()` match statement

Example:

```rust
// src/cli/mycommand.rs
use clap::Args;

#[derive(Args, Clone)]
pub struct MyCommandArgs {
    #[arg(short, long)]
    pub option: String,
}

pub async fn handle(args: MyCommandArgs) -> anyhow::Result<()> {
    println!("Running mycommand with {}", args.option);
    Ok(())
}
```

### Adding a Model

1. Create file in `src/models/<model>.rs`
2. Implement struct with required fields
3. Add `pub use` to `src/models/mod.rs`
4. Derive Serialize/Deserialize for JSON support

### Adding Tests

Unit tests go in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(), expected);
    }
}
```

Integration tests go in `tests/`:

```rust
// tests/test_feature.rs
#[tokio::test]
async fn test_feature() {
    // Test code here
}
```

## Testing with tmux

For integration tests that require tmux:

```bash
# Create a test session
tmux new-session -d -s test-session
tmux send-keys -t test-session "while true; do echo \"test log $(date)\"; sleep 1; done" Enter

# Run LogPilot
logpilot watch test-session

# Clean up
tmux kill-session -t test-session
```

## Debugging

Enable logging:

```bash
RUST_LOG=debug cargo run -- watch my-session
RUST_LOG=trace cargo run -- mcp-server
```

Log levels:
- `error` - Critical failures
- `warn` - Anomalies and recoverable issues
- `info` - Normal operations (default)
- `debug` - Detailed flow information
- `trace` - Very verbose (use sparingly)

## Pull Request Process

1. **Fork and branch**: Create a feature branch from `main`
2. **Write tests**: Add tests for new functionality
3. **Update docs**: Update README.md if needed
4. **Run checks**: Ensure `cargo test`, `cargo fmt`, `cargo clippy` pass
5. **Commit**: Write clear commit messages
6. **PR**: Open pull request with description

## Commit Message Format

```
feat: Add feature X

- Detailed description
- Another point

Closes #123
```

Types:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation
- `test:` Tests
- `refactor:` Code refactoring
- `perf:` Performance
- `chore:` Maintenance

## Code Style

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Document public APIs with `///`
- Keep functions small and focused
- Prefer composition over inheritance
- Handle errors explicitly (no unwrap in production code)

## Getting Help

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- Provide minimal reproducible examples

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT/Apache-2.0 dual license).
