# Technology Stack - LogPilot

AI-Native tmux Log Copilot for Support Incident Tracking

## Languages & Runtimes

- **Language**: Rust 1.86+
- **Edition**: 2021
- **Platform**: Cross-platform (Linux, macOS, tested on darwin/amd64)
- **Async Runtime**: Tokio 1.35 (full features: async, networking, concurrency, signals, time, sync)

## Core Frameworks & Libraries

### Async & Concurrency
- **tokio**: Async runtime with full features (networking, time, sync primitives)
- **tokio-util**: Utilities for codec and stream handling
- **tokio-test**: Testing utilities for async code

### CLI & User Interface
- **clap** (v4.4): Command-line argument parser with derive macros
- **crossterm** (v0.29): Cross-platform terminal UI manipulation
- **tracing** (v0.1): Structured logging and instrumentation
- **tracing-subscriber** (v0.3): Logging backend with environment filtering

### Data Processing & Storage
- **serde** (v1.0): Serialization framework with derive macros
- **serde_json** (v1.0): JSON serialization/deserialization
- **regex** (v1.10): Pattern matching for log parsing
- **dashmap** (v5.5): Concurrent hashmap for thread-safe session data
- **sqlx** (v0.8.3): SQL toolkit with SQLite driver and tokio runtime
- **toml** (v0.9): TOML configuration file parsing
- **once_cell** (v1.19): Lazy static initialization for compiled regex patterns

### Utilities
- **uuid** (v1.6): UUID generation (v4) with serde support
- **chrono** (v0.4): Date/time handling with timezone support and serde
- **thiserror** (v1.0): Error type derivation
- **anyhow** (v1.0): Flexible error handling
- **dirs** (v5.0): Cross-platform user directory resolution
- **async-trait** (v0.1): Async trait support

### System Integration
- **tokio::process::Command**: Subprocess execution (tmux integration)
- **std::process::{Command, Stdio}**: Process spawning and I/O
- **std::io::{BufRead, Write}**: Standard I/O operations (MCP stdio transport)

## Build & Tooling

### Build Configuration
- **Cargo**: Standard Rust package manager
- **Cargo.lock**: Dependency version pinning
- **Cargo.toml**: Package manifest with binary target
- **rust-toolchain.toml**: Enforced Rust 1.86 with rustfmt & clippy

### Development Tools
- **just**: Command runner (Justfile-based recipes)
- **cargo fmt**: Code formatter with pre-commit hook
- **cargo clippy**: Linter with strict warnings-as-errors mode
- **cargo test**: Unit and integration testing
- **cargo check**: Fast compilation without codegen
- **cargo watch** (optional): File watching for auto-recompile/retest
- **cargo audit** (optional): Security dependency scanning
- **cargo tree** (optional): Dependency graph visualization

### Release Optimization
- **Profile.release**:
  - opt-level: 3 (aggressive optimization)
  - LTO: enabled (link-time optimization)
  - codegen-units: 1 (single-codegen pass)
  - strip: enabled (binary stripping)

### CI/CD
- **GitHub Actions**: CI/CD pipeline
  - Rust 1.86 via dtolnay/rust-toolchain
  - Rust cache for build artifact reuse
  - Parallel jobs: test, fmt, clippy
  - Triggers: push to main/master, pull requests

### Code Quality
- **Pre-commit hooks**: fmt → test → clippy
- **Renovate**: Automated dependency updates (renovate.json)

## Database Layer

### Storage
- **SQLite**: Primary persistent storage for high-severity logs (ERROR, FATAL)
- **sqlx query builder**: Type-safe SQL with compile-time verification
- **Connection pooling**: Up to 5 concurrent connections
- **Schema**: Single `log_entries` table with indexed fields

### Schema
```
log_entries:
  - id (TEXT PRIMARY KEY, UUID)
  - pane_id (TEXT)
  - sequence (INTEGER)
  - timestamp (TEXT)
  - severity (TEXT)
  - service (TEXT, nullable)
  - raw_content (TEXT)
  - parsed_fields (TEXT, JSON)
  - received_at (TEXT, ISO8601)
```

### In-Memory Store
- **DashMap**: Concurrent hashmap for live session data (entries, patterns, incidents, alerts)
- **Arc<RwLock>**: For synchronized access to mutable state
- **Rolling buffer**: 10,000 entry limit per session with FIFO eviction

## Configuration Management

### Config File Format
- **TOML**: User configuration at `~/.config/logpilot/config.toml`
- **Settings**:
  - Buffer duration (minutes)
  - Max memory (MB)
  - Persistence severity levels
  - Custom regex patterns
  - Alert thresholds
  - MCP transport settings

### Logging Configuration
- **Environment-based**: RUST_LOG for tracing-subscriber filtering
- **Structured logging**: JSON-compatible event logging with contextual fields

## Application Architecture

### Module Structure
- **main.rs**: CLI entry point (Tokio main async runtime)
- **capture/**: tmux session and pane monitoring
  - tmux.rs: tmux command builder with target validation
  - session.rs: Session lifecycle management
  - pane.rs: Pane I/O and FIFO handling
- **pipeline/**: Log processing pipeline
  - parser.rs: Regex-based log line parsing (timestamps, severity, services)
  - formats.rs: Multi-format log support (JSON, logfmt, standard)
  - cluster.rs: Error pattern clustering
  - dedup.rs: Duplicate elimination
- **buffer/**: In-memory and persistent storage
  - manager.rs: Buffer lifecycle and rotation
  - persistence.rs: SQLite integration
- **mcp/**: Model Context Protocol implementation
  - server.rs: JSON-RPC 2.0 request/response handler
  - protocol.rs: MCP message types and JSON-RPC structures
  - resources.rs: Resource URI handlers
  - data_store.rs: Thread-safe session data store
- **analyzer/**: Log analysis and anomaly detection
- **cli/**: Command handlers (watch, summarize, ask, mcp-server, status)
- **models/**: Data structures (LogEntry, Pattern, Incident, Alert, Severity)
- **observability.rs**: Tracing instrumentation
- **error.rs**: Custom error types and Result<T> type alias

## Security Features

- **Input validation**: tmux target and file path sanitization
  - Rejects metacharacters (;, |, &, $, `, etc.)
  - Prevents path traversal (..)
  - Regex-based allowlist matching
- **Process isolation**: Subprocess execution with restricted stdio
- **Sensitive log handling**: Severity-based persistence filtering

## Performance Characteristics

- **Log parsing**: Compiled lazy-static regex patterns
- **Concurrency**: Tokio async/await with DashMap for lock-free operations
- **Memory**: Bounded rolling buffers (10k entry limit)
- **Database**: Connection pooling with max 5 connections
- **Release build**: Aggressive optimization with LTO and stripping
