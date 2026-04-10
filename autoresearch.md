# Autoresearch: MCP Standard Compliance

## Objective
Make LogPilot's MCP implementation fully compliant with the Model Context Protocol standard and verify it using the MCP inspector tool. Focus on protocol correctness, error handling, and resource discovery.

## Metrics
- **Primary**: mcp_inspector_pass_rate (%, higher is better) — percentage of inspector tests passing
- **Secondary**: protocol_errors_count (lower is better), resource_discovery_time_ms (lower is better)

## How to Run
`./autoresearch.sh` — installs MCP inspector if needed, runs it against the server, outputs `METRIC mcp_inspector_pass_rate=X`.

## Files in Scope
- `src/mcp/protocol.rs` — JSON-RPC 2.0 message types, request/response handling
- `src/mcp/server.rs` — MCP server implementation, request handlers
- `src/mcp/resources.rs` — Resource handler for session data
- `src/mcp/data_store.rs` — Shared data store for live session data
- `src/cli/mcp.rs` — MCP server command entry point
- `Cargo.toml` — may need to add MCP SDK dependency

## Off Limits
- Core log capture logic (tmux interaction, parsing)
- CLI commands other than mcp-server
- Buffer management and persistence

## Constraints
- Must maintain backward compatibility with existing MCP clients
- Must use stdio transport (as per MCP spec)
- Must support all required MCP methods: initialize, resources/list, resources/read, ping
- Protocol version must be "2024-11-05"
- Tests must pass (`cargo test --all-features`)

## What's Been Tried

### Current State
- Basic MCP server structure exists with stdio transport
- Implements: initialize, resources/list, resources/read, ping
- Protocol version: 2024-11-05
- JSON-RPC 2.0 message types implemented
- Resource URIs: logpilot://session/{name}/summary, entries, patterns, incidents, alerts

### Known Issues
1. No proper MCP SDK integration - hand-rolled protocol implementation
2. No validation against official MCP inspector
3. Error handling may not match MCP spec exactly
4. No support for notifications (client → server)
5. Resource templates may not follow MCP URI template spec

### Potential Improvements
1. Add `rmcp` (Rust MCP SDK) for standard compliance
2. Use MCP inspector for automated testing
3. Add proper logging/tracing for debugging
4. Implement notification support if needed
5. Add resource subscription support
