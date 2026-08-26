# External Integrations

**Analysis Date:** 2026-08-26

## APIs & External Services

**MCP (Model Context Protocol) — local stdio, not a hosted API:**
- Custom JSON-RPC 2.0 server over stdin/stdout - Exposes log/incident context to AI hosts (Claude Code / Codex). Implemented in `src/mcp/server.rs`, types in `src/mcp/protocol.rs`, resources in `src/mcp/resources.rs`, CLI entry `src/cli/mcp.rs` (`logpilot mcp-server`). Protocol initialize version `"2025-06-18"` (`src/mcp/server.rs`); stderr banner still says `2024-11-05` (`src/cli/mcp.rs`). Tools: `search`, `stats`. Resources: `logpilot://session/{name}/summary|entries|patterns|incidents|alerts`.
- SDK/Client: Official `rmcp` crate is **not** a live dependency (`Cargo.toml` comments it out; `src/mcp/mod.rs` does not compile `rmcp_server`). Fallback hand-rolled server. Host config example: `~/.claude/config.json` with `command: logpilot`, `args: ["mcp-server"]`, `env: {}` (`README.md`, `docs/MCP_TESTING.md`).
- Auth: None (empty `env` in MCP host config; no tokens).

**tmux (local process, not HTTP):**
- tmux CLI - Live pane capture via `pipe-pane` to FIFOs, session/window/pane listing, `capture-pane` snapshots (`src/capture/tmux.rs`, `src/capture/pane.rs`, `src/cli/filter.rs`, `src/cli/ask.rs`, MCP fallbacks in `src/mcp/server.rs` / `src/mcp/rmcp_server.rs`).
- SDK/Client: `tokio::process::Command` / `std::process::Command` spawning binary `tmux`.
- Auth: None (local tmux server / default socket; `src/models/session.rs` has unused `tmux_socket: Option<String>`).

**Unix named pipes:**
- `mkfifo` - Create per-pane FIFOs under `std::env::temp_dir()` (`src/capture/pane.rs`).
- SDK/Client: `tokio::process::Command::new("mkfifo")`.
- Auth: None.

**LLM providers:**
- None called from this crate. `logpilot ask` only **prints a debugging prompt** from tmux logs for paste into Claude/Codex (`src/cli/ask.rs`). No OpenAI/Anthropic HTTP client (`Cargo.toml` has no `reqwest`/`ureq`).
- SDK/Client: N/A
- Auth: N/A

**crates.io (publish only):**
- Package registry for `cargo install logpilot` / `cargo publish` (`README.md`, `Justfile` `publish`, `login-env`).
- SDK/Client: cargo
- Auth: `CRATES_IO_TOKEN` (`Justfile`) — not used at runtime.

## Data Storage

**Databases:**
- SQLite (embedded file, not a hosted DB). Schema `log_entries` in `src/buffer/persistence.rs` (`sqlx` `CREATE TABLE IF NOT EXISTS`). Stores high-severity (ERROR/FATAL) entries when watch persistence initializes (`src/cli/watch.rs`, `src/buffer/manager.rs`).
- Connection: File URL `sqlite:{path}` or `sqlite::memory:` for tests (`src/buffer/persistence.rs`). Path hardcoded in watch as `dirs::data_dir()/logpilot/logs.db` (typically `~/.local/share/logpilot/logs.db`); fallback `.logpilot/logs.db`. Pool: `SqlitePoolOptions` max 5 connections (file) / 1 (memory). No `DATABASE_URL` env var.
- Client: sqlx 0.9.0 (`sqlite` + `runtime-tokio`) in `Cargo.toml`; `sqlx::FromRow` in `src/buffer/persistence.rs`. No ORM beyond sqlx.

**File Storage:**
- Local filesystem only. Config: `dirs::config_dir()/logpilot/config.toml` (`src/lib.rs`). Data: XDG data dir / `config.example.toml` `persist_path = "~/.local/share/logpilot"`. FIFOs: OS temp dir (`src/capture/pane.rs`, `src/cli/filter.rs`). Completions: `completions/` shipped with the crate. No S3/GCS/Azure.

**Caching:**
- None (no Redis/Memcached). In-process ring buffers (`src/buffer/ring.rs`) plus `SessionDataStore` (`src/mcp/data_store.rs`) for live MCP session state. Stale-session cleanup every 300s in `src/mcp/server.rs`.

## Authentication & Identity

**Auth Provider:**
- None / not applicable. Local CLI and stdio MCP; no user accounts, OAuth, or API keys.
- Implementation: tmux target sanitization only (`src/capture/tmux.rs` `validate_target` / `validate_path`). MCP has no auth handshake beyond JSON-RPC `initialize`.

## Monitoring & Observability

**Error Tracking:**
- None (no Sentry/Datadog/OpenTelemetry exporter).

**Logs:**
- `tracing` + `tracing-subscriber` fmt to stderr (`src/main.rs`). Documented `RUST_LOG=info|debug|trace` (`README.md`, `CONTRIBUTING.md`). Structured helper events in `src/observability.rs` (`Metrics`, `log_mcp_request`, etc.) on the library crate; the binary `src/main.rs` does not `mod observability` (dogfooding helpers are not wired from the CLI entrypoint).
- MCP server also writes human stderr banners (`src/cli/mcp.rs`) and tracing `info!`/`debug!` for JSON-RPC (`src/mcp/server.rs`).
- CI: GitHub Actions logs only (`.github/workflows/ci.yml`).

## CI/CD & Deployment

**Hosting:**
- None. Binary is a local tool. Distribution via crates.io / GitHub (`https://github.com/jellydn/logpilot` in `Cargo.toml` `repository`). No Dockerfile, no cloud deploy workflow.

**CI Pipeline:**
- GitHub Actions workflow `CI` in `.github/workflows/ci.yml`: `push`/`pull_request` to `main`/`master`; jobs `test` (ubuntu-latest, `actions/checkout@v7`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `cargo build --release`, `cargo test --all-features`), `fmt`, `clippy -D warnings`.
- Renovate Bot via `renovate.json`.
- Local: `just ci` / `just test` (`Justfile`), pre-commit (`.pre-commit-config.yaml`).
- Publish: manual `just publish` / `cargo publish` (`Justfile`); spec task T100 in `specs/001-tmux-log-copilot/tasks.md` notes crates.io publish needs access.

## Environment Configuration

**Required env vars:**
- None required for runtime. App works from defaults (`src/lib.rs` `Config::default`) if `config.toml` is absent.
- Optional:
  - `RUST_LOG` - tracing level (`README.md`).
  - `CARGO_TERM_COLOR` - CI cargo color (`.github/workflows/ci.yml`).
  - `CRATES_IO_TOKEN` - crates.io login (`Justfile` `login-env` only).
- Config file keys (not env): `buffer.duration_minutes`, `max_memory_mb`, `persist_severity`, `persist_path`; `patterns.custom_patterns`; `alerts.*`; `mcp.enabled`, `mcp.transport` (`config.example.toml`). MCP transport is currently stdio only.

**Secrets location:**
- No runtime secrets. Publish token is operator env (`CRATES_IO_TOKEN`), not stored in-repo. MCP host `env: {}` (`README.md`). No Vault/AWS SM/dotenv.

## Webhooks & Callbacks

**Incoming:**
- None. No HTTP server. MCP is newline-delimited JSON-RPC on stdio (`src/mcp/server.rs` `run_stdio`). Methods: `initialize`, `ping`, `resources/list`, `resources/read`, `tools/list`, `tools/call`. Notifications (no `id`) get no response.

**Outgoing:**
- None. No webhook clients, no LLM HTTP calls, no telemetry exporters. Outbound process calls are local: `tmux`, `mkfifo`.

---

*Integration audit: 2026-08-26*
