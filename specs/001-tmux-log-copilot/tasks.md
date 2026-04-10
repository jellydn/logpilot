# Tasks: tmux Log Copilot

**Input**: Design documents from `/specs/001-tmux-log-copilot/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/mcp-schema.json

**Tests**: INCLUDED (per Constitution Principle V - Test-First & Observability)

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Create project directory structure per plan.md (src/cli, src/capture, src/pipeline, src/analyzer, src/mcp, src/buffer, src/models, tests/)
- [x] T002 Initialize Rust project with Cargo.toml (edition 2021, rust-version 1.75)
- [x] T003 [P] Add core dependencies to Cargo.toml: tokio (full), serde, serde_json, regex, clap, crossterm, dashmap, sqlx (sqlite), uuid, chrono
- [x] T004 [P] Configure development tools: rustfmt, clippy, cargo-audit in rust-toolchain.toml
- [x] T005 Create .gitignore for Rust project (target/, Cargo.lock, *.log)
- [x] T006 [P] Setup CI workflow template (.github/workflows/ci.yml) for test, clippy, fmt

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Base Models (Required by all stories)

- [x] T007 [P] Create Severity enum in src/models/severity.rs (TRACE, DEBUG, INFO, WARN, ERROR, FATAL, UNKNOWN)
- [x] T008 [P] Create SessionStatus enum in src/models/session.rs (Active, Stale, Disconnected)
- [x] T009 [P] Create PaneStatus enum in src/models/pane.rs (Capturing, Paused, Error)
- [x] T010 [P] Create AlertType enum in src/models/alert.rs (RecurringError, RestartLoop, NewException, ErrorRate)
- [x] T011 [P] Create AlertStatus enum in src/models/alert.rs (Active, Acknowledged, Resolved)
- [x] T012 [P] Create IncidentStatus enum in src/models/incident.rs (Active, Mitigating, Resolved)

### Core Models with Full Implementation

- [x] T013 Create Session struct in src/models/session.rs (id, name, tmux_socket, status, created_at, last_seen, pane_ids)
- [x] T014 Create Pane struct in src/models/pane.rs (id, session_id, tmux_id, status)
- [x] T015 Create LogEntry struct in src/models/log_entry.rs (id, pane_id, sequence, timestamp, severity, service, raw_content, parsed_fields, received_at)
- [x] T016 Create Pattern struct in src/models/pattern.rs (id, signature, regex, severity, first_seen, last_seen, occurrence_count, window_count, window_start, sample_entry)
- [x] T017 Create Incident struct in src/models/incident.rs (id, title, severity, status, started_at, resolved_at, pattern_ids, affected_services, entry_count)
- [x] T018 Create Alert struct in src/models/alert.rs (id, type, incident_id, pattern_id, threshold, current_value, triggered_at, acknowledged_at, status, message)
- [x] T019 Create mod.rs exports in src/models/mod.rs

### Configuration & Error Handling

- [x] T020 Create Config struct in src/lib.rs for TOML configuration (buffer duration, persistence, alert thresholds)
- [x] T021 Create LogPilotError enum in src/error.rs (Io, Tmux, Parse, Database, Config, Mcp)
- [x] T022 Create shared utility module src/util.rs (defer to when needed) (timestamp parsing, hash functions)

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Watch and Capture tmux Logs (Priority: P1) 🎯 MVP

**Goal**: Enable LogPilot to attach to running tmux sessions and capture live log output with <2s latency

**Independent Test**: Run `logpilot watch test-session` against a tmux session producing logs, verify logs appear in buffer within 2 seconds, test multiple concurrent captures

### Tests for User Story 1 (Test-First)

- [x] T023 [P] [US1] Create integration test scaffold in tests/integration/test_capture.rs
- [x] T024 [P] [US1] Create mock tmux fixture in tests/fixtures/mock_tmux.sh for testing without real tmux
- [x] T025 [P] [US1] Write test: `test_watch_attach_to_session` - verifies session attachment
- [x] T026 [P] [US1] Write test: `test_capture_latency_under_2s` - verifies <2s latency requirement
- [x] T027 [P] [US1] Write test: `test_multiple_concurrent_captures` - verifies independent pane capture
- [x] T028 [P] [US1] Write test: `test_session_stale_on_disconnect` - verifies standby mode behavior

### Implementation for User Story 1

- [x] T029 [P] [US1] Create CaptureProcess struct in src/capture/process.rs (handle to pipe-pane process)
- [x] T030 [P] [US1] Create tmux command builder in src/capture/tmux.rs (pipe-pane, list-sessions, list-panes)
- [x] T031 [US1] Implement pipe-pane streaming in src/capture/pane.rs (spawn process, read stdout, tokio async)
- [x] T032 [US1] Implement Session manager in src/capture/session.rs (track sessions, handle reconnect, stale detection)
- [x] T033 [US1] Implement SessionRepository in src/capture/repository.rs (dashmap concurrent storage)
- [x] T034 [US1] Create watch command handler in src/cli/watch.rs (clap args, session validation, start capture)
- [x] T035 [US1] Implement standby mode logic (detect disconnect, mark stale, auto-retry every 5s)
- [x] T036 [US1] Add visual status output in src/cli/watch.rs (color-coded session status)
- [x] T037 [US1] Wire up watch command in src/main.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Intelligent Log Analysis and Anomaly Detection (Priority: P1)

**Goal**: Automatically detect patterns, anomalies, and errors in log streams (recurring errors, restart loops, new exceptions)

**Independent Test**: Feed known error patterns into captured log stream, verify pattern detection, deduplication, and incident clustering work correctly

### Tests for User Story 2 (Test-First)

- [x] T038 [P] [US2] Create integration test scaffold in tests/integration/test_analyzer.rs
- [x] T039 [P] [US2] Write test: `test_recurring_error_detection` - same error 5+ times in 60s window
- [x] T040 [P] [US2] Write test: `test_restart_loop_detection` - starting → stopping → starting within 30s
- [x] T041 [P] [US2] Write test: `test_new_exception_detection` - first-seen signature flagged
- [x] T042 [P] [US2] Write test: `test_deduplication_simhash` - similar stack traces deduplicated
- [x] T043 [P] [US2] Write test: `test_pattern_sliding_window_decay` - window_count resets after 60s

### Implementation for User Story 2

#### Pipeline Components

- [x] T044 [P] [US2] Create LogParser in src/pipeline/parser.rs (regex-based timestamp/severity/service extraction)
- [x] T045 [P] [US2] Create structured format parsers in src/pipeline/formats.rs (JSON, logfmt)
- [x] T046 [US2] Implement Deduplicator in src/pipeline/dedup.rs (SimHash for fuzzy matching)
- [x] T047 [US2] Implement Cluster engine in src/pipeline/cluster.rs (group similar errors into patterns)
- [x] T048 [US2] Create tokio channels in src/pipeline/mod.rs (mpsc between components)
- [x] T049 [US2] Implement pipeline orchestrator in src/pipeline/mod.rs (producer-consumer flow)

#### Analyzer Components

- [x] T050 [P] [US2] Create PatternTracker in src/analyzer/patterns.rs (sliding window frequency counting)
- [x] T051 [P] [US2] Create PatternRepository in src/analyzer/repository.rs (dashmap concurrent storage)
- [x] T052 [US2] Implement RestartLoopDetector in src/analyzer/patterns.rs (state machine for start/stop patterns)
- [x] T053 [US2] Implement NewExceptionDetector in src/analyzer/patterns.rs (track first-seen signatures)
- [x] T054 [US2] Create IncidentDetector in src/analyzer/incidents.rs (auto-create incidents from pattern spikes)
- [x] T055 [US2] Create IncidentRepository in src/analyzer/incidents.rs (store and query incidents)

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - AI Context Bridge for Claude/Codex (Priority: P1)

**Goal**: Expose structured, token-aware summaries via MCP for AI-assisted root cause analysis

**Independent Test**: Run `logpilot summarize --last 10m` and verify output conforms to MCP schema and can be consumed by Claude Code

### Tests for User Story 3 (Test-First)

- [x] T056 [P] [US3] Create contract test scaffold in tests/contract/test_mcp_schema.rs
- [x] T057 [P] [US3] Write test: `test_summary_json_schema_valid` - validates against contracts/mcp-schema.json
- [x] T058 [P] [US3] Write test: `test_token_aware_truncation` - verifies 4000 token budget respected
- [x] T059 [P] [US3] Write test: `test_mcp_resource_list` - verifies resources/list endpoint
- [x] T060 [P] [US3] Write test: `test_mcp_resource_read_summary` - verifies resources/read for summary

### Implementation for User Story 3

#### MCP Server

- [x] T061 [P] [US3] Create MCP protocol types in src/mcp/protocol.rs (JSON-RPC 2.0 message structures)
- [x] T062 [P] [US3] Create McpServer in src/mcp/server.rs (stdio transport, request handler)
- [x] T063 [US3] Implement resources/list handler in src/mcp/server.rs
- [x] T064 [US3] Implement resources/read handler for all 5 resource types in src/mcp/resources.rs
- [x] T065 [US3] Create resource URI parser in src/mcp/resources.rs (logpilot://session/{name}/...)

#### Buffer & Persistence

- [x] T066 [P] [US3] Create RingBuffer in src/buffer/ring.rs (circular buffer with O(1) eviction)
- [x] T067 [P] [US3] Create HybridEvictor in src/buffer/evictor.rs (time-based FIFO + severity persistence) - SKIPPED (merged into manager)
- [x] T068 [US3] Implement SQLite persistence in src/buffer/persistence.rs (ERROR/FATAL table schema)
- [x] T069 [US3] Create PaneBuffer in src/buffer/mod.rs (combines ring buffer + persistence) - SKIPPED (merged into manager)
- [x] T070 [US3] Create BufferManager in src/buffer/manager.rs (per-pane buffer lifecycle)

#### Summarization & CLI

- [x] T071 [P] [US3] Create SummaryBuilder in src/models/summary.rs (aggregate entries, patterns, incidents)
- [x] T072 [US3] Implement token-aware truncation in src/models/summary.rs (prioritize by severity/recency)
- [x] T073 [US3] Create summarize command in src/cli/summarize.rs (--last duration, format output)
- [x] T074 [US3] Create ask command in src/cli/ask.rs (format query + context for LLM)
- [x] T075 [US3] Create mcp-server command in src/cli/mcp.rs (start MCP server mode)
- [x] T076 [US3] Wire up commands in src/main.rs

**Checkpoint**: All P1 user stories should now be independently functional

---

## Phase 6: User Story 4 - Alert Triggers for Proactive Response (Priority: P2)

**Goal**: Automatically alert engineers when error thresholds or anomaly conditions are met

**Independent Test**: Configure alert thresholds, simulate error patterns, verify CLI visual indicators appear

### Tests for User Story 4 (Test-First)

- [x] T077 [P] [US4] Create integration test in tests/integration/test_alerts.rs
- [x] T078 [P] [US4] Write test: `test_error_rate_threshold_alert` - error rate > threshold triggers alert
- [x] T079 [P] [US4] Write test: `test_alert_deduplication` - no duplicate alerts for same incident
- [x] T080 [P] [US4] Write test: `test_alert_acknowledgment` - acknowledged alerts marked correctly
- [x] T081 [P] [US4] Write test: `test_visual_indicator_output` - color codes appear in terminal

### Implementation for User Story 4

- [x] T082 [P] [US4] Create AlertEvaluator in src/analyzer/alerts.rs (check thresholds, trigger conditions)
- [x] T083 [US4] Create AlertRepository in src/analyzer/alerts.rs (store and query alerts)
- [x] T084 [US4] Implement ErrorRateCalculator in src/analyzer/alerts.rs (sliding window errors/min)
- [x] T085 [US4] Create AlertNotifier in src/analyzer/alerts.rs (publish to broadcast channel)
- [x] T086 [US4] Implement visual alert indicators in src/cli/watch.rs (color/highlight in terminal)
- [x] T087 [US4] Add alert acknowledgment handler (keypress 'a' in watch mode)
- [x] T088 [US4] Create status command in src/cli/status.rs (list active alerts, incidents)
- [x] T089 [US4] Wire up status command in src/main.rs

**Checkpoint**: All user stories complete and independently testable

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T090 [P] Create configuration file loader in src/config.rs (TOML parsing for ~/.config/logpilot/config.toml) - ALREADY IMPLEMENTED
- [x] T091 [P] Add structured logging/self-observability in src/observability.rs (dogfooding - log own metrics)
- [ ] T092 Add performance benchmarks in benches/pipeline.rs (10k lines/min throughput test) - FUTURE WORK
- [x] T093 Create comprehensive README.md with architecture overview
- [x] T094 Create CONTRIBUTING.md with development setup instructions
- [x] T095 Add security audit: verify no credentials in logs, sanitize content - HANDLED IN IMPLEMENTATION
- [x] T096 Run quickstart.md validation - verify all commands work as documented - COMPLETED
- [ ] T097 Add man page generation for CLI commands - FUTURE WORK
- [x] T098 Create shell completions (bash, zsh, fish) for CLI
- [x] T099 Final code review: check constitution compliance, remove TODOs - COMPLETED
- [ ] T100 Tag v0.1.0 release and publish to crates.io - REQUIRES PUBLISH ACCESS

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 1 (Setup) ──► Phase 2 (Foundational) ──► Phase 3 (US1) ──► Phase 4 (US2)
                                                      │            │
                                                      ▼            ▼
                                               Phase 5 (US3) ◄──┘
                                                      │
                                                      ▼
                                               Phase 6 (US4)
                                                      │
                                                      ▼
                                               Phase 7 (Polish)
```

### User Story Dependencies

- **US1 (P1)**: Can start after Phase 2 complete. No dependencies on other stories.
- **US2 (P1)**: Can start after Phase 2 complete. Integrates with US1 (feeds from capture) but testable with mock input.
- **US3 (P1)**: Can start after Phase 2 complete. Depends on US1 (capture) and US2 (patterns) for full functionality, but MCP server testable standalone.
- **US4 (P2)**: Can start after Phase 2 + US2 complete. Depends on patterns/alerts from US2.

### Within-Story Dependencies

**US1**: T029, T030 → T031 → T032 → T033 → T034 → T035, T036, T037
**US2**: T044, T045 → T046 → T047 → T048, T049 → T050, T051 → T052, T053 → T054, T055
**US3**: T066, T067 → T068 → T069 → T070 → T071 → T072 → T073, T074, T075, T076 (T061-T065 parallel)
**US4**: T082, T083 → T084 → T085 → T086, T087 → T088, T089

### Parallel Opportunities by Phase

**Phase 2 (Foundational)**: T007-T012 (enums), T013-T018 (models) — 12 parallel tasks  
**Phase 3 (US1)**: T023-T027 (tests), T029-T030 (capture components) — 7 parallel tasks  
**Phase 4 (US2)**: T038-T043 (tests), T044-T045, T050-T051 — 8 parallel tasks  
**Phase 5 (US3)**: T056-T060 (tests), T061-T062, T066-T067, T071 — 8 parallel tasks  
**Phase 6 (US4)**: T077-T081 (tests), T082-T083 — 7 parallel tasks  
**Phase 7 (Polish)**: T090, T091, T093-T098 — 8 parallel tasks

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T006)
2. Complete Phase 2: Foundational (T007-T022) — CRITICAL BLOCKER
3. Complete Phase 3: User Story 1 (T023-T037)
4. **STOP and VALIDATE**: Test `logpilot watch <session>` independently
5. Demo: Show live tmux capture with <2s latency

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1 (Watch/Capture) → Test independently → Demo (MVP!)
3. US2 (Analysis) → Test independently → Demo (Pattern detection!)
4. US3 (AI Bridge) → Test independently → Demo (Claude Code integration!)
5. US4 (Alerts) → Test independently → Demo (Proactive alerting!)
6. Polish → Release v0.1.0

### Parallel Team Strategy

With 4 developers post-Foundational:
- **Dev A**: US1 (capture layer) — critical path
- **Dev B**: US2 (pipeline/analyzer) — parallel with US1 using mock data
- **Dev C**: US3 (MCP/buffer) — parallel, integrate as US1/US2 complete
- **Dev D**: US4 (alerts) — starts after US2 patterns ready

---

## Task Count Summary

| Phase | Tasks | Parallel Tasks |
|-------|-------|----------------|
| Phase 1: Setup | 6 | 4 |
| Phase 2: Foundational | 16 | 12 |
| Phase 3: US1 (P1) | 15 | 7 |
| Phase 4: US2 (P1) | 18 | 8 |
| Phase 5: US3 (P1) | 21 | 8 |
| Phase 6: US4 (P2) | 12 | 7 |
| Phase 7: Polish | 11 | 8 |
| **Total** | **100** | **54** |

---

## Notes

- [P] tasks = different files, no dependencies — safe to parallelize
- Each user story includes independent test criteria from spec.md
- Constitution compliance: TDD enforced (tests before implementation)
- Critical path: T001-T022 (Foundation) → T031 (capture) → T046-T047 (pipeline) → T072 (summarization)
- Estimated MVP (US1 only): 37 tasks, ~2 weeks with 2 developers
- Estimated Full Feature (all stories): 100 tasks, ~6 weeks with 4 developers
