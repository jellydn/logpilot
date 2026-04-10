# Technology Stack

**Analysis Date:** 2025-04-10

## Languages

**Primary:**
- Rust (Edition 2021, minimum version 1.75) - Complete codebase

**Secondary:**
- TOML - Configuration files (`Cargo.toml`, config files)
- YAML - CI/CD workflows (`.github/workflows/ci.yml`)

## Runtime

**Environment:**
- Tokio async runtime (v1.35) - Full features enabled

**Package Manager:**
- Cargo (bundled with Rust)
- Lockfile: `Cargo.lock` present

## Frameworks

**Core:**
- Tokio (v1.35) - Async runtime with full feature set
- Clap (v4.4) - CLI argument parsing with derive macros
- Serde (v1.0) - Serialization framework

**Testing:**
- Built-in Rust test framework (`cargo test`)
- Tokio-test (v0.4) - Async test utilities
- Tempfile (v3.8) - Temporary file management for tests

**Build/Dev:**
- Cargo - Build and package management
- Rustfmt - Code formatting
- Clippy - Linting

## Key Dependencies

**Critical:**
- `tokio` (v1.35) - Async runtime, essential for all I/O operations
- `sqlx` (v0.7) - Async SQLite database with runtime-tokio
- `clap` (v4.4) - CLI parsing with derive macros
- `regex` (v1.10) - Pattern matching for log parsing

**Infrastructure:**
- `serde` + `serde_json` - Serialization
- `tracing` + `tracing-subscriber` - Structured logging
- `crossterm` (v0.27) - Terminal manipulation
- `dashmap` (v5.5) - Concurrent hash map
- `uuid` (v1.6) - UUID generation
- `chrono` (v0.4) - Date/time handling
- `thiserror` + `anyhow` - Error handling
- `toml` (v0.8) - Config file parsing
- `dirs` (v5.0) - Platform-appropriate directories
- `async-trait` (v0.1) - Async trait support (for MCP)

## Configuration

**Environment:**
- Config loaded from `~/.config/logpilot/config.toml` or `logpilot.toml`
- Uses `dirs` crate for platform-appropriate paths
- TOML format with typed Config struct

**Build:**
- `Cargo.toml` - Main manifest
- `.cargo/config.toml` - Cargo configuration (if present)
- Profile settings for release (LTO, strip, opt-level 3)

## Platform Requirements

**Development:**
- Rust 1.75 or later
- tmux installed (for session capture)
- SQLite (bundled via sqlx)

**Production:**
- Same as development - standalone binary deployment
- Targets: Linux, macOS (tmux-dependent)

---

*Stack analysis: 2025-04-10*
