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
- 5 basic resources implemented
- Simple query parameter parsing exists
- No pagination (returns max 100 entries)
- No tools (only resources)
- No filtering by severity/time in entries resource

### Ideas for Improvement
1. Add `severity` query param to entries resource
2. Add `since` and `until` time filters
3. Implement `tools/search` for text search
4. Add pagination with `limit` and `offset`
5. Create `stats` resource with aggregated metrics
