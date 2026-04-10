# Implementation Plan: tmux Log Copilot

**Branch**: `001-tmux-log-copilot` | **Date**: 2026-04-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-tmux-log-copilot/spec.md`

## Summary

Build LogPilot: a CLI tool that captures live tmux pane output, performs real-time log analysis (anomaly detection, deduplication, clustering), and exposes structured incident context via MCP (Model Context Protocol) to Claude Code/Codex for AI-assisted incident response. Core value: bridge the gap between terminal-based log monitoring and AI-native troubleshooting workflows.

Technical approach: Rust implementation for performance (10k lines/min target), tmux capture-pane integration, streaming log processing pipeline, pattern detection engine, and MCP server for AI context bridge.

## Technical Context

**Language/Version**: Rust 1.75+ (system-level performance, memory safety, excellent CLI tooling)
**Primary Dependencies**:
- `tokio` (async runtime for concurrent pane capture)
- `serde` + `serde_json` (structured MCP output)
- `regex` (log pattern matching)
- `clap` (CLI argument parsing)
- `crossterm` (terminal UI for visual alerts)
- `dashmap` (concurrent hashmap for session management)
**Storage**: In-memory ring buffer (configurable) + disk persistence for high-severity events (SQLite for structured queries)
**Testing**: `cargo test` with `tokio-test`, integration tests using tmux in Docker
**Target Platform**: Linux, macOS (primary tmux platforms)
**Project Type**: CLI tool with MCP server component
**Performance Goals**: <2s latency from tmux to AI, 10k lines/min per pane, <100MB RAM per 30min buffer
**Constraints**: Local-first (no cloud), no credential storage in plain text, secure IPC only
**Scale/Scope**: Single-user concurrent sessions (up to 10 tmux panes)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Local-First Architecture | ✅ PASS | Spec explicitly requires local-only execution |
| II. Real-Time Performance | ✅ PASS | <2s latency and 10k lines/min targets defined |
| III. CLI-Native Interface | ✅ PASS | CLI-first with `watch`, `summarize`, `ask` commands |
| IV. AI Context Bridge | ✅ PASS | MCP integration specified, JSON schema defined |
| V. Test-First & Observability | ✅ PASS | TDD mandated, integration tests required for tmux |

**Gate Result**: ✅ ALL PRINCIPLES SATISFIED — Proceed to Phase 0

## Project Structure

### Documentation (this feature)

```text
specs/001-tmux-log-copilot/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output (technology decisions)
├── data-model.md        # Phase 1 output (entities, relationships)
├── quickstart.md        # Phase 1 output (user getting-started guide)
├── contracts/           # Phase 1 output (MCP schema definitions)
│   └── mcp-schema.json
└── tasks.md             # Phase 2 output (NOT created by this command)
```

### Source Code (repository root)

```text
# Single CLI project with MCP server
src/
├── main.rs              # CLI entry point
├── cli/                 # Command handling
│   ├── mod.rs
│   ├── watch.rs
│   ├── summarize.rs
│   └── ask.rs
├── capture/             # tmux integration
│   ├── mod.rs
│   ├── session.rs
│   └── pane.rs
├── pipeline/            # Log processing
│   ├── mod.rs
│   ├── parser.rs
│   ├── deduplicator.rs
│   └── cluster.rs
├── analyzer/            # Anomaly detection
│   ├── mod.rs
│   ├── patterns.rs
│   └── alerts.rs
├── mcp/                 # MCP server
│   ├── mod.rs
│   ├── server.rs
│   └── resources.rs
├── buffer/              # Rolling buffer
│   ├── mod.rs
│   ├── ring.rs
│   └── persistence.rs
└── models/              # Data structures
    ├── mod.rs
    ├── log_entry.rs
    ├── pattern.rs
    ├── incident.rs
    └── summary.rs

tests/
├── integration/         # tmux integration tests
│   ├── test_capture.rs
│   ├── test_pipeline.rs
│   └── test_analyzer.rs
├── contract/            # MCP contract tests
│   └── test_mcp_schema.rs
└── unit/                # Unit tests (co-located in src/ via #[cfg(test)])

Cargo.toml
```

**Structure Decision**: Single Rust CLI project with modular architecture aligned with the data pipeline (capture → pipeline → analyzer → MCP). Each module maps to a user story: capture = US1, pipeline/analyzer = US2, MCP = US3, alerts = US4.

## Complexity Tracking

> No constitution violations anticipated. All design decisions align with principles.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | — | — |

---

## Phase 0: Research & Decisions

**Goal**: Resolve all technical unknowns and document technology choices.

**Deliverable**: `research.md`

### Research Tasks

1. **tmux capture-pane mechanisms**: Compare `capture-pane` vs `pipe-pane` for streaming efficiency
2. **MCP protocol implementation**: Review Model Context Protocol specification for resource exposure patterns
3. **Log parsing strategies**: Evaluate regex vs nom parser for performance at 10k lines/min
4. **Pattern detection algorithms**: Research sliding window counting for recurring errors, fingerprinting for deduplication
5. **Rust async patterns**: Tokio channels for producer/consumer pipeline architecture

### Expected Decisions

| Area | Options | Preliminary Choice |
|------|---------|-------------------|
| tmux capture | capture-pane polling vs pipe-pane streaming | pipe-pane for lower latency |
| MCP implementation | mcp-sdk crate vs manual protocol | mcp-sdk if available, else manual |
| Log parsing | regex-based vs structured nom parser | regex for flexibility, nom for known formats |
| Pattern storage | In-memory hashmap vs embedded DB | In-memory with disk overflow |
| Alert threshold | Fixed counts vs sliding window rate | Sliding window for accuracy |

---

## Phase 1: Design & Contracts

**Prerequisites**: research.md complete

### 1.1 Data Model (`data-model.md`)

**Entities to document**:
- `Session` (tmux session metadata, connection state)
- `Pane` (pane identifier, capture configuration)
- `LogEntry` (timestamp, severity, service, raw content, parsed fields)
- `Pattern` (signature, frequency, first/last seen, associated entries)
- `Incident` (cluster ID, severity, timeline, related patterns)
- `Alert` (trigger type, threshold, current value, status)
- `Summary` (MCP resource format: incident overview, error clusters, timeline)

**Relationships**:
- Session 1:N Pane
- Pane 1:N LogEntry
- LogEntry N:1 Pattern (via signature hash)
- Pattern N:M Incident
- Incident 1:N Alert

### 1.2 Contracts (`contracts/mcp-schema.json`)

**MCP Resources to expose**:
- `logpilot://session/{name}/summary` — Current incident summary
- `logpilot://session/{name}/entries?since={timestamp}` — Log entries since time
- `logpilot://session/{name}/patterns` — Detected patterns
- `logpilot://session/{name}/incidents` — Active incidents

**JSON Schema for Summary resource**:
```json
{
  "type": "object",
  "properties": {
    "session": { "type": "string" },
    "timestamp": { "type": "string", "format": "date-time" },
    "window_start": { "type": "string", "format": "date-time" },
    "window_end": { "type": "string", "format": "date-time" },
    "total_entries": { "type": "integer" },
    "incidents": { "type": "array", "items": { "$ref": "#/definitions/incident" } },
    "patterns": { "type": "array", "items": { "$ref": "#/definitions/pattern" } },
    "alerts_active": { "type": "array", "items": { "type": "string" } }
  }
}
```

### 1.3 Quickstart (`quickstart.md`)

**Sections**:
1. Installation (cargo install logpilot)
2. First watch session (`logpilot watch my-session`)
3. Summarizing logs (`logpilot summarize --last 10m`)
4. MCP setup in Claude Code
5. Configuration file format

### 1.4 Agent Context Update

Run: `.specify/scripts/bash/update-agent-context.sh pi`

Add to `.pi/context.md`:
- Technology: Rust, tokio, serde, MCP
- Architecture: Streaming pipeline with async capture
- Key patterns: Producer/consumer, ring buffer, sliding window

---

## Phase 2: Task Generation (Deferred)

**Next Command**: `/speckit.tasks` generates `tasks.md` based on this plan and spec.

**Will create**:
- Task breakdown by user story (US1-US4)
- Parallel task identification
- Test-first task ordering
- Dependency mapping

---

## Post-Design Constitution Check

| Principle | Verification |
|-----------|--------------|
| I. Local-First | ✅ SQLite local-only, no cloud dependencies |
| II. Real-Time | ✅ pipe-pane streaming, tokio async, ring buffer O(1) eviction |
| III. CLI-Native | ✅ clap-based CLI, no GUI, text I/O |
| IV. AI Context Bridge | ✅ MCP server with structured JSON resources |
| V. Test-First | ✅ Integration test structure defined for tmux |

**Final Gate**: ✅ PASS — Ready for task generation
