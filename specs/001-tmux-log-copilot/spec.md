# Feature Specification: tmux Log Copilot

**Feature Branch**: `001-tmux-log-copilot`  
**Created**: 2026-04-10  
**Status**: Draft  
**Input**: PRD: AI-Native tmux Log Copilot for Support Incident Tracking

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch and Capture tmux Logs (Priority: P1)

Support engineers need to attach LogPilot to running tmux sessions and capture live log output from distributed applications they are monitoring during incident response.

**Why this priority**: This is the foundational capability that enables all other features. Without reliable log capture, the AI cannot assist with incident response.

**Independent Test**: Can be fully tested by starting LogPilot with `logpilot watch <session-name>`, verifying it attaches to the tmux pane, and confirming logs are being captured and buffered.

**Acceptance Scenarios**:

1. **Given** a tmux session named "api-prod" is running with `kubectl logs -f`, **When** the engineer runs `logpilot watch api-prod`, **Then** LogPilot attaches to the session and begins streaming output
2. **Given** LogPilot is watching a tmux session, **When** new log lines appear in the pane, **Then** they appear in LogPilot's buffer within 2 seconds
3. **Given** multiple tmux panes are active, **When** the engineer attaches to different sessions, **Then** each session's logs are captured independently without interference

---

### User Story 2 - Intelligent Log Analysis and Anomaly Detection (Priority: P1)

Engineers need LogPilot to automatically detect patterns, anomalies, and errors in the log stream without manual scanning, so they can focus on resolving the incident rather than reading logs.

**Why this priority**: The core value proposition is AI-powered assistance. Anomaly detection reduces mean time to identify (MTTI) and enables proactive incident response.

**Independent Test**: Can be fully tested by feeding known error patterns and anomalies into the log stream and verifying LogPilot identifies and highlights them correctly.

**Acceptance Scenarios**:

1. **Given** a log stream contains recurring error messages, **When** the same error appears 5+ times within 1 minute, **Then** LogPilot flags it as a recurring error pattern
2. **Given** a service is stuck in a restart loop (container restarting repeatedly), **When** LogPilot detects the pattern, **Then** it triggers an alert for "service restart loop detected"
3. **Given** an exception type that has never been seen before appears, **When** LogPilot processes the log, **Then** it highlights this as a "new unseen exception"
4. **Given** log lines contain timestamps and severity levels, **When** LogPilot parses them, **Then** it extracts and enriches the metadata (timestamp, severity, service name)

---

### User Story 3 - AI Context Bridge for Claude/Codex (Priority: P1)

Engineers need structured, token-aware summaries of recent log activity that can be consumed by Claude Code or Codex CLI, enabling AI-assisted root cause analysis.

**Why this priority**: This bridges the gap between raw log capture and AI assistance—the core value proposition of LogPilot.

**Independent Test**: Can be fully tested by running `logpilot summarize --last 10m` and verifying the output can be directly pasted into Claude/Codex and provides meaningful incident context.

**Acceptance Scenarios**:

1. **Given** LogPilot has been capturing logs for 15 minutes, **When** the engineer runs `logpilot summarize --last 10m`, **Then** it outputs a structured summary with incident overview, error patterns, and notable events
2. **Given** the engineer wants AI assistance, **When** they run `logpilot ask "Why are checkout requests failing?"`, **Then** LogPilot provides relevant log context formatted for LLM consumption
3. **Given** log volume exceeds LLM context limits, **When** summarization occurs, **Then** LogPilot applies token-aware windowing to prioritize recent and high-severity events
4. **Given** repeated stack traces appear in logs, **When** LogPilot processes them, **Then** it deduplicates them to reduce noise in the AI context

---

### User Story 4 - Alert Triggers for Proactive Response (Priority: P2)

Engineers want LogPilot to automatically notify them when specific incident conditions are met, so they can respond before the situation escalates.

**Why this priority**: Proactive alerting improves incident response time but depends on the foundational capture and analysis features (P1 stories).

**Independent Test**: Can be fully tested by configuring alert thresholds and simulating conditions that should trigger them.

**Acceptance Scenarios**:

1. **Given** an error rate threshold is configured (e.g., 10 errors/minute), **When** the actual error rate exceeds this threshold, **Then** LogPilot emits an alert notification
2. **Given** alert triggers are enabled, **When** a new unseen exception appears, **Then** LogPilot immediately surfaces this as high-priority information
3. **Given** a service restart loop is detected, **When** the pattern persists for more than 30 seconds, **Then** LogPilot alerts with context about the affected service

---

### Edge Cases

- What happens when the tmux session is killed while LogPilot is watching?
- How does the system handle very high log volume (10k+ lines/minute) for extended periods?
- What occurs when the rolling buffer fills up—how is old data evicted?
- How are multi-line log entries (like stack traces) handled during parsing?
- What happens if tmux is not installed or the session name doesn't exist?
- How does the system handle binary or non-text output in tmux panes?
- What occurs during tmux pane scrollback—are historical lines captured or only new ones?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST allow attachment to named tmux sessions/panes for log capture
- **FR-002**: System MUST stream stdout from tmux panes in near real-time with <2 second latency
- **FR-003**: System MUST maintain a rolling buffer of captured logs (configurable duration, default 30 minutes) with hybrid eviction: time-based FIFO for general logs, high-severity events (ERROR/FATAL) persisted to disk for extended retention
- **FR-004**: System MUST parse timestamps, severity levels, and service names from log lines
- **FR-005**: System MUST deduplicate repeated error patterns and stack traces
- **FR-006**: System MUST cluster similar errors into logical incidents
- **FR-007**: System MUST detect and flag: recurring errors, service restart loops, new unseen exceptions
- **FR-008**: System MUST provide `logpilot watch <session>` command to start capture
- **FR-009**: System MUST provide `logpilot summarize --last <duration>` command for AI-ready context
- **FR-010**: System MUST provide `logpilot ask "<question>"` command for AI-assisted analysis
- **FR-011**: System MUST support multiple concurrent tmux pane captures
- **FR-012**: System MUST handle 10,000 log lines per minute per pane without data loss
- **FR-013**: System MUST export structured summaries via MCP as JSON with predefined schema (incident summary, error clusters, timeline, metadata) for Claude/Codex consumption
- **FR-014**: System MUST implement token-aware summarization to respect LLM context limits

### Key Entities

- **Session**: A tmux session being monitored (identified by name)
- **Pane**: A specific tmux pane within a session (LogPilot captures per pane)
- **LogEntry**: A single line or multi-line log event with metadata (timestamp, severity, service, raw content)
- **Pattern**: A detected recurring error or anomaly signature
- **Incident**: A cluster of related log entries representing a logical issue
- **Alert**: A triggered notification when thresholds or conditions are met
- **Summary**: A condensed, token-aware representation of recent log activity for AI consumption

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Engineers can identify incident root cause 40% faster compared to manual log review
- **SC-002**: System detects anomalies before human notice in at least 60% of production incidents
- **SC-003**: Alert false-positive rate is below 5% (fewer than 1 in 20 alerts is a duplicate or incorrect)
- **SC-004**: Log latency from tmux pane to AI visibility is under 2 seconds in 95% of cases
- **SC-005**: System successfully processes 10,000 log lines per minute per pane without dropping data
- **SC-006**: AI-generated summaries fit within standard LLM context windows (128k tokens) while preserving critical incident details
- **SC-007**: Engineers can query recent log history and receive relevant answers within 5 seconds

## Assumptions

- Users have tmux installed and are familiar with basic tmux operations (sessions, panes)
- Target users are already using Claude Code or Codex CLI in their terminal workflows
- Logs follow common formats (timestamps, severity indicators) that can be parsed heuristically
- Local execution environment has sufficient resources (RAM, CPU) for log processing
- Users prefer CLI-first interaction over GUI for this workflow
- Network connectivity is not required for core functionality (local-first execution)
- tmux pane content is text-based log output (not binary data or interactive TUIs)
- Users will configure alert thresholds based on their specific service characteristics

## Clarifications

### Session 2026-04-10

- **Q**: When tmux disconnects or the buffer reaches capacity, what should LogPilot's behavior be? → **A**: Standby mode - LogPilot keeps running but marks stream as "stale", resumes capture when tmux reconnects. This provides best incident response UX without requiring manual restart during active incidents.
- **Q**: When the rolling buffer is full, what eviction policy should be applied? → **A**: Hybrid with severity retention - Time-based primary eviction, but high-severity events (ERROR/FATAL) persisted to disk for longer retention. This balances performance with incident response needs.
- **Q**: How should LogPilot deliver alert notifications to the engineer? → **A**: CLI visual indicators only - Color/highlight in terminal, status bar updates, no audio or external notifications. Keeps focus in terminal context during incident response.
- **Q**: What structured format should LogPilot expose via MCP for AI consumption? → **A**: Structured JSON with predefined schema - Fixed schema (incident summary, error clusters, timeline, metadata) that Claude/Codex parses consistently. Predictable structure enables reliable AI processing.

## Open Questions

1. ~~MCP vs File vs Stdin~~ → **RESOLVED: MCP (Model Context Protocol)** will be the primary integration method. LogPilot will implement an MCP server that exposes log context to Claude Code, providing structured, real-time context without file I/O overhead or tight coupling to stdin piping.

2. ~~Remote SSH Support~~ → **RESOLVED: Local tmux only for MVP, SSH support as v1.1 enhancement**. Initial release focuses on local tmux sessions to ensure reliability and simplicity. Remote SSH session monitoring will be prioritized based on user feedback post-MVP.
