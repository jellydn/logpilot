# Quickstart: LogPilot for tmux

Get LogPilot running and integrated with Claude Code in under 5 minutes.

---

## Installation

### Prerequisites

- tmux installed and running
- Rust toolchain (1.75+) — [install via rustup](https://rustup.rs/)
- macOS or Linux

### Install from crates.io (when published)

```bash
cargo install logpilot
```

### Install from source

```bash
git clone https://github.com/jellydn/logpilot
cd logpilot
cargo build --release
# Binary at: target/release/logpilot
```

---

## First Watch Session

### 1. Start tmux with a log-producing session

```bash
# Create a tmux session with some logs
tmux new-session -d -s api-prod -n logs
tmux send-keys -t api-pros:logs "kubectl logs -f deployment/api-prod" Enter
```

### 2. Attach LogPilot

```bash
logpilot watch api-prod
```

**Expected output**:
```
[LogPilot] Attaching to session: api-prod
[LogPilot] Monitoring pane: api-prod:logs.0
[LogPilot] Buffer: 30min rolling, persistence: ERROR/FATAL
[LogPilot] MCP server ready on stdio
[LogPilot] Press Ctrl+C to stop
```

### 3. Verify capture

In another terminal, check LogPilot is seeing logs:

```bash
# Summary of last 5 minutes
logpilot summarize --last 5m
```

---

## Claude Code Integration

### Option 1: MCP Server Mode (Recommended)

LogPilot exposes an MCP server that Claude Code can connect to.

#### Add to Claude Code configuration

Edit your Claude Code configuration (typically `~/.claude/config.json`):

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

#### Query logs from Claude Code

Once connected, ask Claude:

```
What errors have occurred in the last 10 minutes?
```

Claude will fetch the `logpilot://session/{name}/summary` resource and provide analysis.

### Option 2: Manual Context Copy

For quick ad-hoc analysis:

```bash
# Generate summary and copy to clipboard
logpilot summarize --last 10m | pbcopy  # macOS
logpilot summarize --last 10m | xclip -selection clipboard  # Linux
```

Then paste into Claude Code chat.

---

## Common Commands

### Watch a session

```bash
logpilot watch <session-name>
```

Options:
- `--pane <pane-id>` — Watch specific pane (default: active pane)
- `--buffer <minutes>` — Rolling buffer duration (default: 30)
- `--persist-path <path>` — Where to store high-severity logs

### Summarize recent activity

```bash
logpilot summarize --last <duration>
```

Duration formats: `10m`, `1h`, `30s`

### Ask AI-assisted questions

```bash
logpilot ask "Why are checkout requests failing?"
```

This formats recent logs + your question for Claude/Codex.

### List monitored sessions

```bash
logpilot status
```

### Stop watching

Ctrl+C in the watch terminal, or:

```bash
logpilot stop <session-name>
```

---

## Configuration File

Create `~/.config/logpilot/config.toml`:

```toml
[buffer]
duration_minutes = 30
max_memory_mb = 100
persist_severity = ["ERROR", "FATAL"]
persist_path = "~/.local/share/logpilot"

[patterns]
# Custom regex patterns for your log format
custom_patterns = [
  "^(?P<timestamp>\\d{4}-\\d{2}-\\d{2}T[^ ]+) (?P<level>\\w+) (?P<service>\\w+): (?P<message>.*)$"
]

[alerts]
# Alert thresholds
recurring_error_window_seconds = 60
recurring_error_threshold = 5
restart_loop_window_seconds = 30
error_rate_threshold_per_minute = 10

[mcp]
# MCP server settings
enabled = true
transport = "stdio"
```

---

## Troubleshooting

### "Session not found"

```bash
# List tmux sessions
tmux list-sessions

# Verify exact name
logpilot watch "exact-session-name"
```

### "Permission denied" on tmux socket

```bash
# Check tmux socket path
echo $TMUX

# Run with same user as tmux session
# Or: tmux -S /path/to/socket list-sessions
```

### High memory usage

Reduce buffer duration:

```bash
logpilot watch api-prod --buffer 10  # 10 minutes instead of 30
```

Or in config:

```toml
[buffer]
duration_minutes = 10
max_memory_mb = 50
```

### MCP not connecting

1. Verify LogPilot is running: `logpilot status`
2. Check Claude Code MCP config syntax
3. Try manual mode: `logpilot mcp-server --verbose`

---

## Next Steps

- **Configure alert thresholds** in `config.toml`
- **Add custom patterns** for your log formats
- **Integrate with Slack** (future enhancement)
- **Read full documentation**: See `docs/` directory

---

## Keyboard Shortcuts (During Watch)

| Key | Action |
|-----|--------|
| `Ctrl+C` | Stop watching |
| `s` | Print summary to stdout |
| `a` | Acknowledge all alerts |
| `r` | Force pattern re-analysis |
| `?` | Show help |

---

**Version**: MVP 1.0 | **Platform**: macOS, Linux | **Requires**: tmux, Rust 1.75+
