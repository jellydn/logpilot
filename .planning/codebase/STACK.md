# Technology Stack

**Analysis Date:** 2026-08-26

## Languages

**Primary:**
- Rust 1.86 (MSRV, `edition = "2021"`) - Entire application in `src/` (`Cargo.toml` `rust-version`; binary `src/main.rs`, library `src/lib.rs`). Pinned local toolchain is 1.98 in `rust-toolchain.toml` (`channel = "1.98"`, components `rustfmt`, `clippy`). CI uses `dtolnay/rust-toolchain@stable` in `.github/workflows/ci.yml` (not a pinned 1.86 image). `CONTRIBUTING.md` still says “Rust 1.75+”, which is stale vs `Cargo.toml`.

**Secondary:**
- TOML - App config (`config.example.toml`, runtime `~/.config/logpilot/config.toml` via `src/lib.rs`) and crate manifest (`Cargo.toml`).
- JSON - MCP JSON-RPC 2.0 over stdio (`src/mcp/protocol.rs`, `src/mcp/server.rs`), structured log parsing (`src/pipeline/formats.rs`), MCP resource payloads (`src/mcp/resources.rs`).
- Shell (bash / zsh / fish) - Completions in `completions/logpilot.bash`, `completions/logpilot.zsh`, `completions/logpilot.fish`; test fixture `tests/fixtures/mock_tmux.sh`.
- YAML - GitHub Actions (`.github/workflows/ci.yml`) and pre-commit (`.pre-commit-config.yaml`).
- JSON schema / Renovate - `renovate.json` extends `config:recommended`.

## Runtime

**Environment:**
- Native compiled `logpilot` binary (`[[bin]]` in `Cargo.toml`, path `src/main.rs`). Async runtime Tokio 1.53.1 (`Cargo.lock`; `tokio` 1.51 in `Cargo.toml` with `features = ["full"]`, `#[tokio::main]` in `src/main.rs`). No Node/Python/JVM runtime.
- Unix process tools at runtime: `tmux` (pane capture in `src/capture/tmux.rs`) and `mkfifo` (named pipes in `src/capture/pane.rs`).

**Package Manager:**
- Cargo (crates.io registry). Optional install: `cargo install logpilot` or `cargo install --git https://github.com/jellydn/logpilot --locked` (`README.md`).
- Lockfile: present (`Cargo.lock`, included in crate via `Cargo.toml` `include`).

## Frameworks

**Core:**
- Tokio 1.53.1 (`Cargo.lock`) - Async runtime, process spawn, channels, filesystem (`src/capture/`, `src/cli/watch.rs`, `src/mcp/server.rs`).
- clap 4.6.6 (`Cargo.lock`; `clap` 4.6 derive in `Cargo.toml`) - CLI parser and subcommands in `src/main.rs` (`watch`, `filter`, `summarize`, `ask`, `mcp-server`, `status`).
- serde 1.0.229 + serde_json 1.0.151 (`Cargo.lock`) - Serialization for config, MCP, SQLite JSON fields.
- Custom MCP JSON-RPC server - `src/mcp/server.rs` + `src/mcp/protocol.rs` (stdio). Official `rmcp` crate is **commented out** in `Cargo.toml` (“disabled due to Rust 1.86 compatibility”) and `src/mcp/mod.rs` (`rmcp_server` module not compiled). `src/mcp/rmcp_server.rs` still exists as unused source. Startup banner in `src/cli/mcp.rs` prints protocol `2024-11-05`; initialize result in `src/mcp/server.rs` advertises `protocol_version: "2025-06-18"`.
- crossterm 0.29.0 (`Cargo.lock`) - Terminal input for watch TUI (`src/cli/watch.rs`).
- sqlx 0.9.0 (`Cargo.lock`; `features = ["sqlite", "runtime-tokio"]` in `Cargo.toml`) - SQLite persistence (`src/buffer/persistence.rs`). No `bundled` feature; `libsqlite3-sys` 0.30.1 uses `pkg-config` / `cc` / `vcpkg`.

**Testing:**
- Rust `#[test]` / `cargo test` - Unit tests colocated in `src/`; integration tests in `tests/` (`test_alerts.rs`, `test_filter.rs`, `test_mcp_protocol.rs`, `test_pipeline_integration.rs`, `tests/integration/test_analyzer.rs`, `tests/integration/test_capture.rs`).
- tokio-test 0.4.5 (`Cargo.lock`) - Async test helpers (`Cargo.toml` `[dev-dependencies]`).
- tempfile 3.27.0 (`Cargo.lock`) - Temporary dirs/files in tests (`Cargo.toml` `[dev-dependencies]`).
- MCP protocol tests spawn `./target/release/logpilot` (`tests/test_mcp_protocol.rs`); CI builds release first (`.github/workflows/ci.yml`, `AGENTS.md`).

**Build/Dev:**
- rustc / cargo - `just build`, `just release` in `Justfile`; release profile `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = false` (macOS Sequoia LINKEDIT workaround in `Cargo.toml`).
- just - Task runner (`Justfile`: `test`, `ci`, `lint`, `fmt`, `mcp`, `watch`, `publish`).
- rustfmt + clippy - `rust-toolchain.toml` components; `just fmt` / `just lint`; CI jobs in `.github/workflows/ci.yml`.
- pre-commit - Local hooks in `.pre-commit-config.yaml` (`cargo fmt -- --check`, `cargo test`, `cargo clippy`).
- Optional: `cargo-watch`, `cargo-audit`, `cargo-tree` (`Justfile` `watch-test`, `audit`, `tree`).
- Renovate - `renovate.json`.

## Key Dependencies

**Critical:**
- tokio 1.53.1 + tokio-util 0.7.19 (`Cargo.lock`) - Async I/O, codecs, process, mpsc (`Cargo.toml`).
- clap 4.6.6 - CLI surface (`src/main.rs`).
- sqlx 0.9.0 + libsqlite3-sys 0.30.1 - Persist ERROR/FATAL logs (`src/buffer/persistence.rs`).
- serde / serde_json - MCP, config, parsed log fields.
- regex 1.13.1 (`Cargo.lock`) - Log parsing and tmux target validation (`src/pipeline/`, `src/capture/tmux.rs`).
- tracing 0.1.44 + tracing-subscriber 0.3.23 (`Cargo.lock`; `env-filter` feature in `Cargo.toml`) - Logging; `tracing_subscriber::fmt::init()` in `src/main.rs` (honors `RUST_LOG` via default fmt subscriber).
- dashmap 6.2.1 (`Cargo.lock`) - Concurrent maps (`Cargo.toml`).
- uuid 1.25.0 (`Cargo.lock`; v4 + serde) - Session/pane/entry IDs (`src/models/`).
- chrono 0.4.45 (`Cargo.lock`) - Timestamps (`src/models/log_entry.rs`, MCP windows).
- thiserror 2.0.20 + anyhow 1.0.104 (`Cargo.lock`) - Error types (`src/error.rs`) and CLI handlers (`src/cli/`).
- toml 1.1.4 (`Cargo.lock`) + dirs 6.0.0 - Config load (`src/lib.rs`).
- url 2.5.8 (`Cargo.lock`) - MCP resource query percent-decoding (`src/mcp/resources.rs` `url::form_urlencoded`).
- schemars 1.2.2 (`Cargo.lock`) - Declared in `Cargo.toml` (intended for rmcp JSON Schema); rmcp itself is not a live dependency.
- once_cell 1.21.4 (`Cargo.lock`) - Lazy regex in `src/capture/tmux.rs`.

**Infrastructure:**
- SQLite (via sqlx, file `logs.db` under XDG data dir) - `src/cli/watch.rs` path `dirs::data_dir()/logpilot/logs.db`; in-memory `sqlite::memory:` for tests (`src/buffer/persistence.rs`).
- tmux CLI - `Command::new("tmux")` for `pipe-pane`, `list-sessions`, `list-windows`, `list-panes`, `capture-pane` (`src/capture/tmux.rs`, `src/cli/filter.rs`, `src/cli/ask.rs`, `src/mcp/server.rs`).
- Named FIFOs - `mkfifo` + `std::env::temp_dir()` (`src/capture/pane.rs`).
- Shell completions - `completions/` shipped with crate (`Cargo.toml` `include`).

## Configuration

**Environment:**
- Optional `RUST_LOG` for tracing (`README.md`, `CONTRIBUTING.md`). `src/main.rs` uses `tracing_subscriber::fmt::init()` (env-filter crate feature is declared but not wired to a custom `EnvFilter` builder).
- Optional `CRATES_IO_TOKEN` for `just login-env` (`Justfile`) — publish only, not runtime.
- CI sets `CARGO_TERM_COLOR=always` (`.github/workflows/ci.yml`).
- No API keys, cloud credentials, or `.env` files in the repo.
- App config: `dirs::config_dir()/logpilot/config.toml` (typically `~/.config/logpilot/config.toml`); fallback `logpilot.toml` (`src/lib.rs` `Config::load`). Example: `config.example.toml` (`[buffer]`, `[patterns]`, `[alerts]`, `[mcp]`). Defaults if file missing.
- Data dir: `dirs::data_dir()/logpilot` (typically `~/.local/share/logpilot`); `config.example.toml` `persist_path`. Watch currently hardcodes `dirs::data_dir()/logpilot/logs.db` (`src/cli/watch.rs`) rather than `Config.buffer.persist_path`.

**Build:**
- `Cargo.toml` / `Cargo.lock` - Dependencies and profiles.
- `rust-toolchain.toml` - rustc 1.98 + rustfmt/clippy.
- `Justfile` - Dev/CI/publish recipes.
- `.github/workflows/ci.yml` - test (release build then `cargo test --all-features`), fmt, clippy.
- `.pre-commit-config.yaml` - Local quality gates.
- `renovate.json` - Dependency updates.

## Platform Requirements

**Development:**
- rustup + Rust ≥ 1.86 (local pin 1.98). `just` recommended (`AGENTS.md`).
- tmux installed and a running server (`README.md`, `CONTRIBUTING.md`; tests skip if missing in `tests/test_filter.rs`).
- Unix `mkfifo` for live pane capture (`src/capture/pane.rs`).
- System SQLite development library likely required (sqlx sqlite without `bundled`).
- pre-commit optional (`README.md`).
- macOS: release `strip = false` to avoid Sequoia linker issues (`Cargo.toml`).

**Production:**
- Local CLI / MCP stdio process — no container, k8s, or hosted service in-repo.
- Distribution: `cargo install` from crates.io or GitHub (`README.md`); `just publish` to crates.io (`Justfile`).
- Runtime: Unix-like OS with `tmux` + `mkfifo`; writable XDG config/data dirs; SQLite file under `~/.local/share/logpilot/logs.db` (or `.logpilot/logs.db` fallback).
- MCP hosts (Claude Code / Codex) spawn `logpilot mcp-server` (`README.md`, `docs/MCP_TESTING.md`).
- License: `Cargo.toml` `MIT OR Apache-2.0`; on-disk `LICENSE` is MIT only.

---

*Stack analysis: 2026-08-26*
