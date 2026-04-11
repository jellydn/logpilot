# Welcome to LogPilot 🚀

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000)](https://github.com/jellydn/logpilot/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://rust-lang.org)
[![CI](https://github.com/jellydn/logpilot/workflows/CI/badge.svg)](https://github.com/jellydn/logpilot/actions)

> AI-Native tmux Log Copilot for Support Incident Tracking

LogPilot captures live tmux pane output, performs real-time log analysis (anomaly detection, deduplication, clustering), and exposes structured incident context via MCP (Model Context Protocol) to Claude Code/Codex for AI-assisted incident response.

## 🏠 [Homepage](https://github.com/jellydn/logpilot)

### ✨ [Demo](https://github.com/jellydn/logpilot#quick-start)

## Pre-requirements

- [Rust — A language empowering everyone to build reliable and efficient software](https://rustup.rs/)
- [tmux — A terminal multiplexer](https://github.com/tmux/tmux)

## 💻 Stack

- [tokio: A runtime for writing reliable asynchronous applications with Rust.](https://tokio.rs/)
- [clap: A full featured, fast Command Line Argument Parser for Rust](https://github.com/clap-rs/clap)
- [serde: A generic serialization/deserialization framework](https://serde.rs/)
- [crossterm: A crossplatform terminal manipulation library](https://github.com/crossterm-rs/crossterm)
- [dashmap: A fast concurrent hashmap](https://github.com/xacrimon/dashmap)
- [sqlx: The Rust SQL Toolkit](https://github.com/launchbadge/sqlx)
- [uuid: A library to generate and parse UUIDs](https://github.com/uuid-rs/uuid)
- [chrono: Date and time library for Rust](https://github.com/chronotope/chrono)
- [tracing: A scoped, structured logging and diagnostics system](https://github.com/tokio-rs/tracing)
- [regex: An implementation of regular expressions for Rust](https://github.com/rust-lang/regex)

## 📝 Project Summary

- [**src/analyzer**](src/analyzer): Anomaly detection and pattern analysis.
- [**src/buffer**](src/buffer): Log storage with ring buffer and SQLite persistence.
- [**src/capture**](src/capture): tmux integration for streaming log capture.
- [**src/cli**](src/cli): CLI commands (watch, summarize, ask, status, mcp-server).
- [**src/mcp**](src/mcp): MCP protocol implementation for AI context bridge.
- [**src/models**](src/models): Data structures (LogEntry, Session, Pattern, Alert, etc.).
- [**src/pipeline**](src/pipeline): Log processing (parse, dedup, cluster).
- [**src/observability.rs**](src/observability.rs): Structured logging and metrics.
- [**tests/**](tests/): Integration and unit tests.
- [**completions/**](completions/): Shell completions for bash, zsh, fish.

## Install

```sh
cargo install logpilot
```

Or install directly from GitHub:

```sh
cargo install --git https://github.com/jellydn/logpilot --locked
```

Or build from source:

```sh
git clone https://github.com/jellydn/logpilot
cd logpilot
cargo build --release
```

## Usage

### 1. Watch a tmux session

```sh
# Watch an existing tmux session
logpilot watch my-session

# Watch with custom buffer duration
logpilot watch my-session --buffer 10

# Watch specific pane
logpilot watch my-session --pane my-session:1.0
```

### 2. Get a summary

```sh
# Summary of last 10 minutes
logpilot summarize --last 10m

# JSON output for scripting
logpilot summarize --last 5m --format json
```

### 3. Ask AI-assisted questions

```sh
# Format query for Claude/Codex
logpilot ask "Why are checkout requests failing?"

# Include raw logs in context
logpilot ask "What changed in the last hour?" --include-logs
```

### 4. Check status

```sh
logpilot status
```

## Configuration

Create `~/.config/logpilot/config.toml` (see [config.example.toml](config.example.toml)):

```toml
[buffer]
duration_minutes = 30
max_memory_mb = 100
persist_severity = ["ERROR", "FATAL"]

[alerts]
recurring_error_threshold = 5
error_rate_threshold_per_minute = 10

[mcp]
enabled = true
transport = "stdio"
```

## MCP Server Mode

LogPilot exposes an MCP server for Claude Code integration:

```sh
logpilot mcp-server --verbose
```

Add to your Claude Code configuration (`~/.claude/config.json`):

```json
{
  "mcp_servers": [
    {
      "name": "logpilot",
      "command": "logpilot",
      "args": ["mcp-server"],
      "env": {}
    }
  ]
}
```

## Run tests

```sh
cargo test
```

## Run with logging

```sh
RUST_LOG=info cargo run -- watch my-session
```

## Shell Completions

```sh
# Bash
source completions/logpilot.bash

# Zsh
source completions/logpilot.zsh

# Fish
source completions/logpilot.fish
```

## Pre-commit

This project uses [pre-commit](https://pre-commit.com/) to enforce code quality. To install hooks:

```sh
pre-commit install
```

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

## Author

- Website: https://productsway.com/
- Twitter: [@jellydn](https://twitter.com/jellydn)
- Github: [@jellydn](https://github.com/jellydn)

## Star History 🌟

[![Star History Chart](https://api.star-history.com/svg?repos=jellydn/logpilot&type=Date)](https://star-history.com/#jellydn/logpilot)

## Show your support

Give a ⭐️ if this project helped you!

## Contributors ✨

Thanks goes to these wonderful people:

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->
<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
