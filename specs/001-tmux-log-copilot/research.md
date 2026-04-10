# Research & Technology Decisions: tmux Log Copilot

**Feature**: tmux Log Copilot  
**Date**: 2026-04-10  
**Phase**: 0 — Research & Decisions

---

## 1. tmux Capture Mechanisms

### Options Evaluated

| Mechanism | Latency | Complexity | Reliability |
|-----------|---------|------------|-------------|
| `capture-pane` polling | ~100-500ms | Low | High |
| `pipe-pane` streaming | <50ms | Medium | Medium |
| Direct tmux socket IPC | <10ms | High | Low (internal API) |

### Decision: **pipe-pane streaming**

**Rationale**: 
- Lowest latency meets <2s requirement with margin
- Native stdout streaming without polling overhead
- Standard tmux feature (stable API)
- Simpler error handling than socket IPC

**Tradeoff**: Slightly more complex process management than polling, but acceptable for latency gains.

**Implementation**: Spawn `tmux pipe-pane -t <target> "cat >> /tmp/logpilot-fifo"` per pane, read from named pipes in tokio async tasks.

---

## 2. MCP Protocol Implementation

### Options Evaluated

| Approach | Maturity | Effort | Integration |
|----------|----------|--------|-------------|
| `mcp-sdk` crate (if exists) | Unknown | Low | Native |
| Manual JSON-RPC | Stable | Medium | Full control |
| Claude Desktop protocol | Evolving | Low | Desktop-only |

### Decision: **Manual JSON-RPC with MCP spec compliance**

**Rationale**:
- No mature Rust MCP SDK found (as of research date)
- MCP is simple JSON-RPC 2.0 with resource primitives
- Manual implementation gives full control over resource schemas
- Future SDK adoption possible without breaking changes

**Protocol Stack**:
- Transport: stdio (Claude Code launches LogPilot as MCP server)
- Messages: JSON-RPC 2.0
- Primitives: `resources/list`, `resources/read` for log data

---

## 3. Log Parsing Strategy

### Options Evaluated

| Parser | Speed | Flexibility | Development |
|--------|-------|-------------|-------------|
| Regex | Fast enough | High (any format) | Quick |
| nom (parser combinator) | Very fast | Medium (structured) | Medium |
| Hand-rolled | Fastest | Low | Slow |

### Decision: **Hybrid: regex primary, optional structured parsers**

**Rationale**:
- Regex handles arbitrary log formats (heuristic matching)
- 10k lines/min = ~167 lines/sec — regex easily handles this
- Pre-built parsers for common formats (JSON, logfmt) for optimization
- Extensible: users can add custom patterns

**Implementation**:
- Default: regex-based timestamp/severity/service extraction
- Known formats: `{"timestamp":"...", "level":"..."}` (JSON), `level=info msg=...` (logfmt)
- Custom: User-provided regex patterns in config

---

## 4. Pattern Detection Algorithms

### Recurring Error Detection

**Algorithm**: Sliding window frequency count with decay

```rust
// Pseudocode
struct PatternTracker {
    signature: String,      // hash of normalized log content
    count: AtomicU32,
    window_start: Instant,  // 60-second sliding window
    first_seen: Instant,
}
```

**Threshold**: 5+ occurrences in 60-second window triggers "recurring error" flag

### Deduplication (Stack Traces)

**Algorithm**: SimHash for fuzzy matching

- Hash stack trace lines with locality-sensitive hashing
- Within Hamming distance 3 = same error
- Reduces noise while catching variations

### Service Restart Loop Detection

**Algorithm**: State machine with timing

```rust
enum RestartState {
    Idle,
    Starting,      // saw "starting" message
    Running,       // normal operation
    Stopping,      // saw "stopping" message
}

// Loop detected: Starting → Running → Stopping → Starting within 30s
```

---

## 5. Rust Async Architecture

### Pattern: Producer-Consumer with Tokio Channels

```
tmux pipe-pane (per pane)
       │
       ▼
[Capture Task] ──► mpsc::channel ──► [Parser Task]
                                        │
                                        ▼
                                   [Dedup Task]
                                        │
                                        ▼
                                   [Analyzer Task]
                                        │
                                        ▼
                                   [Buffer + MCP Server]
```

**Channels**:
- `mpsc::unbounded_channel<LogEntry>` — capture → parser
- `mpsc::channel<LogEntry>` (bounded, 1000) — parser → dedup (backpressure)
- `broadcast::channel<Incident>` — analyzer → MCP (pub/sub for alerts)

**Rationale**:
- Natural backpressure handling with bounded channels
- Independent task failure (per pane) doesn't crash whole system
- MCP server can query buffer without blocking pipeline

---

## 6. Storage Strategy

### In-Memory Ring Buffer

**Structure**: `Arc<RwLock<RingBuffer<LogEntry>>>` per pane

- Capacity: 30 minutes at 10k lines/min = 300k entries
- Entry size: ~256 bytes average = 75MB per pane (acceptable)
- Eviction: FIFO for INFO/DEBUG, persist ERROR/FATAL to SQLite

### SQLite Persistence (High-Severity Only)

**Schema**:
```sql
CREATE TABLE severe_logs (
    id INTEGER PRIMARY KEY,
    session TEXT NOT NULL,
    pane TEXT NOT NULL,
    timestamp INTEGER NOT NULL,  -- Unix epoch ms
    severity TEXT NOT NULL,
    service TEXT,
    content TEXT NOT NULL,
    pattern_hash TEXT,
    incident_id INTEGER
);
CREATE INDEX idx_timestamp ON severe_logs(timestamp);
CREATE INDEX idx_pattern ON severe_logs(pattern_hash);
```

**Rationale**:
- In-memory for speed (real-time pipeline)
- Disk only for ERROR/FATAL (rare, but must not be lost)
- SQLite for queryability (time-range scans, pattern correlation)

---

## 7. Summary of Decisions

| Area | Decision | Rationale |
|------|----------|-----------|
| tmux capture | pipe-pane streaming | Lowest latency, standard API |
| MCP | Manual JSON-RPC | No mature SDK, simple protocol |
| Parsing | Regex + optional structured | Flexibility + speed balance |
| Pattern detection | Sliding window + SimHash | Accurate recurring detection, fuzzy dedup |
| Async | Tokio producer-consumer | Backpressure, fault isolation |
| Storage | In-memory ring + SQLite overflow | Speed for real-time, durability for errors |

---

## Open Questions (for Phase 1)

1. Should we support Windows (tmux on WSL only) or macOS/Linux only?
2. Claude Code MCP integration: stdio transport confirmed?
3. Log format auto-detection: attempt to infer or require explicit config?

**Answers** (documented here):
1. **macOS/Linux only** — tmux is primarily Unix; WSL adds complexity for MVP
2. **Yes, stdio** — Claude Code MCP uses stdio transport
3. **Auto-detect common formats** (JSON, logfmt) with regex fallback
