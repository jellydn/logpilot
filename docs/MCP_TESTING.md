# MCP Server Testing Guide

This guide explains how to test the LogPilot MCP server manually and verify it's working correctly.

## Quick Start

### 1. Start the MCP Server

```bash
./target/release/logpilot mcp-server
```

You should see:
```
[LogPilot] MCP server starting...
[LogPilot] Protocol: Model Context Protocol 2024-11-05
[LogPilot] Version: 0.1.0
[LogPilot] MCP server ready - waiting for connections
```

The server is now running and waiting for JSON-RPC requests on stdin.

### 2. Test with Manual JSON-RPC Requests

The MCP server uses JSON-RPC 2.0 over stdio. You can test it by sending JSON messages:

#### Test: Initialize

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}' | ./target/release/logpilot mcp-server
```

Expected response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocol_version": "2024-11-05",
    "capabilities": {
      "resources": {
        "supported_uris": [
          "logpilot://session/{name}/summary",
          "logpilot://session/{name}/entries",
          "logpilot://session/{name}/patterns",
          "logpilot://session/{name}/incidents",
          "logpilot://session/{name}/alerts"
        ]
      }
    },
    "server_info": {
      "name": "logpilot",
      "version": "0.1.0"
    }
  }
}
```

#### Test: Ping

```bash
echo '{"jsonrpc": "2.0", "id": 2, "method": "ping"}' | ./target/release/logpilot mcp-server
```

Expected response:
```json
{"jsonrpc": "2.0", "id": 2, "result": {}}
```

#### Test: List Resources

```bash
echo '{"jsonrpc": "2.0", "id": 3, "method": "resources/list"}' | ./target/release/logpilot mcp-server
```

Expected response:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "resources": [
      {
        "uri": "logpilot://session/{name}/summary",
        "name": "Session Summary",
        "description": "Current incident summary for session"
      },
      {
        "uri": "logpilot://session/{name}/entries",
        "name": "Log Entries",
        "description": "Log entries within time range"
      },
      {
        "uri": "logpilot://session/{name}/patterns",
        "name": "Detected Patterns",
        "description": "Detected patterns for session"
      },
      {
        "uri": "logpilot://session/{name}/incidents",
        "name": "Active Incidents",
        "description": "Active incidents for session"
      },
      {
        "uri": "logpilot://session/{name}/alerts",
        "name": "Active Alerts",
        "description": "Active alerts for session"
      }
    ]
  }
}
```

### 3. Test with Verbose Mode

For more detailed logging during testing:

```bash
./target/release/logpilot mcp-server --verbose
```

This will show:
- Transport details
- Available resource URIs
- Debug output for each request

## Testing with a Session

To test with actual session data, you need to:

1. First, start watching a tmux session:

```bash
./target/release/logpilot watch my-session &
```

2. Then, in another terminal, start the MCP server with a wrapper that provides session data (this requires integration with the capture system - not yet fully implemented).

## Automated Testing Script

Create a test script `test_mcp.sh`:

```bash
#!/bin/bash
set -e

MCP_SERVER="./target/release/logpilot mcp-server"

echo "Testing MCP Server..."

# Test 1: Initialize
echo -n "Test 1: initialize... "
result=$(echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize"}' | $MCP_SERVER 2>/dev/null)
if echo "$result" | grep -q '"protocol_version":"2024-11-05"'; then
    echo "✓ PASS"
else
    echo "✗ FAIL"
    exit 1
fi

# Test 2: Ping
echo -n "Test 2: ping... "
result=$(echo '{"jsonrpc": "2.0", "id": 2, "method": "ping"}' | $MCP_SERVER 2>/dev/null)
if echo "$result" | grep -q '"result":{}'; then
    echo "✓ PASS"
else
    echo "✗ FAIL"
    exit 1
fi

# Test 3: Resources/List
echo -n "Test 3: resources/list... "
result=$(echo '{"jsonrpc": "2.0", "id": 3, "method": "resources/list"}' | $MCP_SERVER 2>/dev/null)
if echo "$result" | grep -q '"resources"'; then
    echo "✓ PASS"
else
    echo "✗ FAIL"
    exit 1
fi

echo "All tests passed!"
```

Make it executable and run:

```bash
chmod +x test_mcp.sh
./test_mcp.sh
```

## Testing with Claude Code

To test the MCP server with Claude Code:

1. Add to your Claude Code configuration (e.g., `~/.claude/config.json`):

```json
{
  "mcp_servers": [
    {
      "name": "logpilot",
      "command": "/path/to/logpilot/target/release/logpilot",
      "args": ["mcp-server"],
      "env": {}
    }
  ]
}
```

2. Start Claude Code and ask:
   - "What resources are available from logpilot?"
   - "Get the summary for session my-session"

## Debugging Tips

### Server Not Starting
- Check that the binary exists: `ls -la target/release/logpilot`
- Check permissions: `chmod +x target/release/logpilot`

### No Response to Requests
- Ensure you're sending valid JSON-RPC 2.0 format
- Check that the request includes a valid `method` field
- Use `--verbose` flag for debug output

### Invalid JSON
- Check JSON syntax with `jq`:
  ```bash
  echo '{"jsonrpc": "2.0", "id": 1, "method": "ping"}' | jq .
  ```

### Connection Issues
- The MCP server uses stdio (stdin/stdout), not TCP ports
- Ensure no other process is intercepting stdin/stdout
- For testing, redirect stderr to see logs: `2>&1`

## Expected Behavior

1. **Startup**: Server prints `[LogPilot] MCP server ready` and waits
2. **Requests**: Server processes one JSON-RPC request per line
3. **Responses**: Server outputs one JSON-RPC response per line
4. **Shutdown**: Server exits on EOF (Ctrl+D) or SIGTERM

## Resource URIs

The MCP server exposes these resource URIs:

| URI | Description |
|-----|-------------|
| `logpilot://session/{name}/summary` | Session summary with incidents, patterns, alerts |
| `logpilot://session/{name}/entries` | Raw log entries |
| `logpilot://session/{name}/patterns` | Detected patterns |
| `logpilot://session/{name}/incidents` | Active incidents |
| `logpilot://session/{name}/alerts` | Active alerts |

Replace `{name}` with your tmux session name.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "MCP server ready" not appearing | Check stderr output, not stdout |
| Empty responses | Ensure session exists and has data |
| "Session not found" | The session must be actively watched first |
| JSON parse errors | Validate your JSON with `jq` before sending |

## Next Steps

For full integration testing:
1. Start a tmux session with logs: `tmux new-session -d -s test-session "echo 'test log'"`
2. Run LogPilot watch: `logpilot watch test-session`
3. Query via MCP: `echo '{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"logpilot://session/test-session/summary"}}' | logpilot mcp-server`
