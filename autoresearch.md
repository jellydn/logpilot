# Autoresearch: Improve MCP Resources and Tools

## Objective
Enhance LogPilot's MCP server with additional resources and tools for better AI integration. Focus on:
1. Adding new resource types (filters, query parameters)
2. Implementing MCP tools (not just resources)
3. Improving resource content with better metadata
4. Adding resource templates with proper URI template syntax

## Metrics
- **Primary**: resource_coverage_score (%, higher is better) - percentage of useful log analysis features exposed via MCP
- **Secondary**: tool_count (higher is better), avg_response_size_bytes (lower is better for performance)

## How to Run
`./autoresearch.sh` - runs tests and validates MCP resource/tool coverage

## Files in Scope
- `src/mcp/resources.rs` - Resource definitions and handlers
- `src/mcp/server.rs` - MCP server request handlers
- `src/mcp/protocol.rs` - Protocol types (may need Tool types)
- `tests/test_mcp_protocol.rs` - Tests for MCP functionality

## Off Limits
- Core log capture (tmux, parsing)
- CLI commands other than mcp-server
- Database schema changes

## Constraints
- Must maintain backward compatibility with existing resources
- Must follow MCP 2024-11-05 protocol spec
- All new features must have tests
- Response sizes should be reasonable (<100KB default)

## Current Resources
1. `logpilot://session/{name}/summary` - Session overview
2. `logpilot://session/{name}/entries` - Log entries
3. `logpilot://session/{name}/patterns` - Error patterns
4. `logpilot://session/{name}/incidents` - Active incidents
5. `logpilot://session/{name}/alerts` - Active alerts

## Proposed Additions
- Query parameters for filtering (severity, time range, service)
- Pagination support for large entry lists
- Statistics/analytics resource
- Search tool for complex queries
- Export resource for formatted output

## What's Been Tried

### Current State
- 5 basic resources implemented (summary, entries, patterns, incidents, alerts)
- 2 MCP tools implemented: `search` and `stats`
- Query parameter parsing with filtering support:
  - `severity` filter for entries resource
  - `service` filter for entries resource
  - `since` and `until` time range filters for entries resource
- Pagination support with `limit` (max 1000) and `offset` parameters
- Pagination metadata shows filtered total count
- Tests for tools/list and tools/call methods

### Completed
1. ✅ Added `severity` query param to entries resource
2. ✅ Added `since` and `until` time filters to entries resource
3. ✅ Implemented `search` tool with text pattern and optional severity filter
4. ✅ Implemented `stats` tool for session statistics
5. ✅ Added pagination with `limit` and `offset`
6. ✅ URI in responses preserves original query parameters
