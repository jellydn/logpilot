# Data Model: tmux Log Copilot

**Feature**: tmux Log Copilot  
**Date**: 2026-04-10  
**Phase**: 1 — Design

---

## Entity Overview

```
┌─────────┐       ┌─────────┐       ┌───────────┐
│ Session │───────│  Pane   │───────│ LogEntry  │
│         │  1:N  │         │  1:N   │           │
└─────────┘       └─────────┘       └─────┬─────┘
                                          │
                                          │ N:1
                                          ▼
                                    ┌───────────┐
                                    │  Pattern  │
                                    │  (via     │
                                    │ signature)│
                                    └─────┬─────┘
                                          │
                                          │ N:M
                                          ▼
                                    ┌───────────┐
                                    │  Incident │
                                    └─────┬─────┘
                                          │
                                          │ 1:N
                                          ▼
                                    ┌───────────┐
                                    │   Alert   │
                                    └───────────┘
```

---

## Entity: Session

Represents a tmux session being monitored by LogPilot.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `name` | String | tmux session name (e.g., "api-prod") |
| `tmux_socket` | String | Path to tmux socket (default: tmux default) |
| `status` | Enum | `Active`, `Stale`, `Disconnected` |
| `created_at` | DateTime | When monitoring started |
| `last_seen` | DateTime | Last successful capture timestamp |
| `pane_ids` | Vec<UUID> | References to monitored panes |

**Constraints**:
- `name` must be unique per LogPilot instance
- `status` transitions: Active ↔ Stale (on disconnect) ↔ Disconnected (timeout)

---

## Entity: Pane

Represents a specific tmux pane within a session.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `session_id` | UUID | Parent session reference |
| `tmux_id` | String | tmux pane ID (e.g., "api-prod:1.0") |
| `capture_process` | Option<Process> | Handle to pipe-pane process |
| `buffer` | RingBuffer | In-memory log entry storage |
| `status` | Enum | `Capturing`, `Paused`, `Error` |

**Constraints**:
- One capture process per pane maximum
- Buffer size configurable (default: 30 min retention)

---

## Entity: LogEntry

Single log line or multi-line event with parsed metadata.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `pane_id` | UUID | Source pane reference |
| `sequence` | u64 | Monotonic sequence number (per pane) |
| `timestamp` | DateTime | Parsed or received timestamp |
| `severity` | Enum | `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL`, `UNKNOWN` |
| `service` | Option<String> | Parsed service name (e.g., "checkout-service") |
| `raw_content` | String | Original log line(s) |
| `parsed_fields` | Map<String, String> | Extracted structured fields |
| `received_at` | DateTime | When LogPilot received this entry |

**Constraints**:
- `sequence` is per-pane monotonic (for ordering)
- `severity` defaults to `UNKNOWN` if not parsable
- Multi-line entries (stack traces) collapsed into single `raw_content`

---

## Entity: Pattern

Detected recurring error or log signature.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `signature` | String | Hash of normalized content (SimHash or FNV) |
| `regex` | Option<String> | Matching pattern (if user-defined) |
| `severity` | Severity | Highest severity seen for this pattern |
| `first_seen` | DateTime | First occurrence timestamp |
| `last_seen` | DateTime | Most recent occurrence |
| `occurrence_count` | u64 | Total occurrences (sliding window) |
| `window_count` | u32 | Occurrences in current 60s window |
| `window_start` | DateTime | Start of current counting window |
| `sample_entry` | UUID | Representative LogEntry ID |

**Constraints**:
- `signature` is unique per LogPilot instance
- `window_count` decays after 60 seconds (sliding window)
- `occurrence_count` >= `window_count` always

**State Transitions**:
```
New → Active (window_count >= threshold)
Active → Recurring (persisted for 5+ min)
Recurring → Resolved (no occurrences for 10 min)
```

---

## Entity: Incident

Logical cluster of related patterns representing an issue.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `title` | String | Auto-generated or user-defined description |
| `severity` | Severity | Highest severity of constituent patterns |
| `status` | Enum | `Active`, `Mitigating`, `Resolved` |
| `started_at` | DateTime | When incident was created |
| `resolved_at` | Option<DateTime> | When marked resolved |
| `pattern_ids` | Vec<UUID> | Related patterns |
| `affected_services` | Vec<String> | Services mentioned in related logs |
| `entry_count` | u64 | Total log entries associated |

**Constraints**:
- Incidents auto-created when related patterns spike
- Manual or automatic resolution
- Entry count approximate (may include unclustered entries)

---

## Entity: Alert

Triggered notification when thresholds or conditions met.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Internal unique identifier |
| `type` | Enum | `RecurringError`, `RestartLoop`, `NewException`, `ErrorRate` |
| `incident_id` | Option<UUID> | Associated incident (if any) |
| `pattern_id` | Option<UUID> | Associated pattern (if any) |
| `threshold` | Option<f64> | Configured threshold value |
| `current_value` | f64 | Value at trigger time |
| `triggered_at` | DateTime | When alert fired |
| `acknowledged_at` | Option<DateTime> | When engineer acknowledged |
| `status` | Enum | `Active`, `Acknowledged`, `Resolved` |
| `message` | String | Human-readable alert description |

**Constraints**:
- One alert per (type, incident) pair (deduplicated)
- Auto-resolved when underlying condition clears (for some types)

---

## Entity: Summary (MCP Resource)

Condensed, token-aware representation for AI consumption.

| Field | Type | Description |
|-------|------|-------------|
| `session_name` | String | Source session identifier |
| `generated_at` | DateTime | Summary generation timestamp |
| `window_start` | DateTime | Start of summarized period |
| `window_end` | DateTime | End of summarized period |
| `total_entries` | u64 | Count of entries in window |
| `entries_by_severity` | Map<Severity, u64> | Distribution of log levels |
| `active_incidents` | Vec<IncidentSummary> | Active incidents with key details |
| `top_patterns` | Vec<PatternSummary> | Most frequent patterns |
| `active_alerts` | Vec<AlertSummary> | Currently firing alerts |
| `services_affected` | Vec<String> | Unique services in window |

**Constraints**:
- Token budget enforced (default: 4000 tokens for Claude context)
- Prioritizes: active incidents > patterns > recent entries
- Never includes raw log lines (summaries only)

---

## Validation Rules

### LogEntry
1. `timestamp` must not be in the future (allow 5s clock skew)
2. `sequence` must be > previous entry for same pane
3. `raw_content` maximum 10KB (truncate with marker)

### Pattern
1. `signature` must be non-empty and unique
2. `window_count` reset when `now - window_start > 60s`
3. `occurrence_count` never decreases

### Incident
1. Must have at least one associated pattern OR 10+ unpatterned ERROR entries
2. Auto-resolved if no new patterns added for 10 minutes

### Alert
1. Deduplication: max one active alert per (type, incident_id) pair
2. `ErrorRate` alerts require threshold configuration

---

## State Machines

### Session Status
```
        ┌──────────┐
        │  Active  │◄─────────────────┐
        └────┬─────┘                  │
             │ disconnect detected     │ reconnect
             ▼                         │
        ┌──────────┐                   │
        │  Stale   │───────────────────┘
        └────┬─────┘  (auto-retry every 5s)
             │ 5 retries failed
             ▼
        ┌──────────────┐
        │ Disconnected │
        └──────────────┘
```

### Pattern Lifecycle
```
        ┌─────────┐
        │   New   │ (first occurrence)
        └────┬────┘
             │ window_count >= 5
             ▼
        ┌──────────┐
        │  Active  │ (recurring)
        └────┬─────┘
             │ persists 5+ min
             ▼
        ┌────────────┐
        │  Recurring │ (stable pattern)
        └────────────┘
```

---

## Query Patterns

### Real-time Pipeline Queries
- "All entries for pane X since timestamp Y" → RingBuffer scan
- "Patterns with window_count >= threshold" → Pattern index scan
- "Active incidents affecting service Z" → Incident filter by services

### MCP Resource Queries
- "Summary for session X, last 10 minutes" → Aggregate entries, patterns, incidents
- "Entries for pattern Y" → LogEntry filter by pattern_id
- "Active alerts" → Alert filter by status

### Historical Queries (SQLite)
- "All ERROR entries between T1 and T2" → severe_logs table range scan
- "Patterns that correlate with incident X" → Join severe_logs to patterns
