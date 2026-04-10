//! MCP Server implementation for LogPilot
//!
//! Provides stdio-based JSON-RPC communication for AI context exchange

use crate::mcp::data_store::{get_or_init_global_store, SessionDataStore};
use crate::mcp::protocol::*;
use crate::mcp::resources::ResourceHandler;
use chrono::Utc;
use serde_json::json;
use std::io::{self, BufRead, Write};
use tracing::{debug, error, info, warn};

/// MCP Server state and request handler
pub struct McpServer {
    /// Shared session data store for live data access
    data_store: SessionDataStore,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

impl McpServer {
    /// Create a new MCP server with access to the global data store
    pub fn new() -> Self {
        let capabilities = ServerCapabilities {
            resources: ResourceCapabilities {
                supported_uris: vec![
                    "logpilot://session/{name}/summary".to_string(),
                    "logpilot://session/{name}/entries".to_string(),
                    "logpilot://session/{name}/patterns".to_string(),
                    "logpilot://session/{name}/incidents".to_string(),
                    "logpilot://session/{name}/alerts".to_string(),
                ],
            },
            tools: Some(crate::mcp::protocol::ToolsCapabilities {
                list_changed: false,
            }),
        };

        // Get or initialize the global data store for live data access
        let data_store = get_or_init_global_store();

        Self {
            data_store,
            capabilities,
        }
    }

    /// Create a new MCP server with a specific data store (for testing)
    #[cfg(test)]
    fn with_data_store(data_store: SessionDataStore) -> Self {
        let capabilities = ServerCapabilities {
            resources: ResourceCapabilities {
                supported_uris: vec![
                    "logpilot://session/{name}/summary".to_string(),
                    "logpilot://session/{name}/entries".to_string(),
                    "logpilot://session/{name}/patterns".to_string(),
                    "logpilot://session/{name}/incidents".to_string(),
                    "logpilot://session/{name}/alerts".to_string(),
                ],
            },
            tools: Some(crate::mcp::protocol::ToolsCapabilities {
                list_changed: false,
            }),
        };

        Self {
            data_store,
            capabilities,
        }
    }

    /// Handle a single JSON-RPC request and return response
    pub fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params),
            "resources/list" => self.handle_resources_list(id),
            "resources/read" => self.handle_resources_read(id, request.params),
            "tools/list" => self.handle_tools_list(id),
            "ping" => JsonRpcResponse::success(id, json!({})),
            _ => {
                warn!("Unknown method: {}", request.method);
                JsonRpcResponse::error(id, JsonRpcError::method_not_found(&request.method))
            }
        }
    }

    /// Handle async request (for resources/read and tools/call that need session data)
    pub async fn handle_request_async(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "resources/read" => self.handle_resources_read_async(id, request.params).await,
            "tools/call" => self.handle_tools_call_async(id, request.params).await,
            _ => self.handle_request(request), // Fall back to sync handler
        }
    }

    fn handle_initialize(
        &self,
        id: Option<serde_json::Value>,
        _params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            server_info: ServerInfo {
                name: "logpilot".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        match serde_json::to_value(result) {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize: {}", e)),
            ),
        }
    }

    fn handle_resources_list(&self, id: Option<serde_json::Value>) -> JsonRpcResponse {
        let result = ResourceHandler::list_resources();
        match serde_json::to_value(result) {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize: {}", e)),
            ),
        }
    }

    fn handle_tools_list(&self, id: Option<serde_json::Value>) -> JsonRpcResponse {
        let tools = vec![
            Tool {
                name: "search".to_string(),
                description: "Search log entries by text pattern".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "session": {
                            "type": "string",
                            "description": "Session name to search"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Text pattern to search for"
                        },
                        "severity": {
                            "type": "string",
                            "description": "Optional severity filter (ERROR, WARN, etc.)"
                        }
                    },
                    "required": ["session", "pattern"]
                }),
            },
            Tool {
                name: "stats".to_string(),
                description: "Get session statistics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "session": {
                            "type": "string",
                            "description": "Session name"
                        }
                    },
                    "required": ["session"]
                }),
            },
        ];
        let result = ToolsListResult { tools };
        match serde_json::to_value(result) {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize: {}", e)),
            ),
        }
    }

    async fn handle_tools_call_async(
        &self,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        use crate::mcp::protocol::{ToolContent, ToolsCallParams, ToolsCallResult};

        let params = match params {
            Some(p) => match serde_json::from_value::<ToolsCallParams>(p) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                    );
                }
            },
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing params"));
            }
        };

        let result = match params.name.as_str() {
            "search" => {
                let session = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("session"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let pattern = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("pattern"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                let severity = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("severity"))
                    .and_then(|s| s.as_str());

                // Get search results
                let results = match self.search_session(session, pattern, severity).await {
                    Ok(r) => r,
                    Err(e) => format!("Error searching '{}': {}", session, e),
                };

                ToolsCallResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Search results for '{}' in {}:\n{}",
                            pattern, session, results
                        ),
                    }],
                    is_error: None,
                }
            }
            "stats" => {
                let session = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("session"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                let (stats, is_error) = match self.get_session_stats(session).await {
                    Ok(s) => (s, false),
                    Err(e) => (
                        format!("Error getting stats for '{}': {}", session, e),
                        true,
                    ),
                };

                ToolsCallResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: stats,
                    }],
                    is_error: Some(is_error),
                }
            }
            _ => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("Unknown tool: {}", params.name)),
                );
            }
        };

        match serde_json::to_value(result) {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize: {}", e)),
            ),
        }
    }

    fn handle_resources_read(
        &self,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        // Parse params
        let params = match params {
            Some(p) => match serde_json::from_value::<ResourcesReadParams>(p) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                    );
                }
            },
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing params"));
            }
        };

        // Parse URI
        let parsed = match ResourceHandler::parse_uri(&params.uri) {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("Invalid URI: {}", params.uri)),
                );
            }
        };

        // Return not found error - async handler will have actual data
        JsonRpcResponse::error(
            id,
            JsonRpcError {
                code: -32002,
                message: format!(
                    "Session '{}' not found (use async handler)",
                    parsed.session_name
                ),
                data: None,
            },
        )
    }

    async fn handle_resources_read_async(
        &self,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        // Parse params
        let params = match params {
            Some(p) => match serde_json::from_value::<ResourcesReadParams>(p) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                    );
                }
            },
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing params"));
            }
        };

        // Parse URI
        let parsed = match ResourceHandler::parse_uri(&params.uri) {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("Invalid URI: {}", params.uri)),
                );
            }
        };

        // Get session data from the shared store (live data from watch command)
        let session_data = match self.data_store.get_session(&parsed.session_name).await {
            Some(data) => data,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError {
                        code: -32002,
                        message: format!(
                            "Session '{}' not found. Is the watch command running?",
                            parsed.session_name
                        ),
                        data: None,
                    },
                );
            }
        };

        // Build resource content based on type
        let content = match parsed.resource_type.as_str() {
            "summary" => ResourceHandler::build_summary(
                &parsed.session_name,
                &session_data.entries,
                &session_data.patterns,
                &session_data.incidents,
                &session_data.alerts,
                session_data.window_start,
                Utc::now(),
            ),
            "entries" => ResourceHandler::build_entries(
                &parsed.session_name,
                &session_data.entries,
                &parsed.query_params,
            ),
            "patterns" => {
                ResourceHandler::build_patterns(&parsed.session_name, &session_data.patterns)
            }
            "incidents" => {
                ResourceHandler::build_incidents(&parsed.session_name, &session_data.incidents)
            }
            "alerts" => ResourceHandler::build_alerts(&parsed.session_name, &session_data.alerts),
            _ => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!(
                        "Unknown resource type: {}",
                        parsed.resource_type
                    )),
                );
            }
        };

        let result = ResourcesReadResult {
            contents: vec![content],
        };

        match serde_json::to_value(result) {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize: {}", e)),
            ),
        }
    }

    /// Run the MCP server on stdio
    pub async fn run_stdio(&self) -> io::Result<()> {
        info!("MCP server starting on stdio");

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line?;
            debug!("Received: {}", line);

            // Parse request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let response = JsonRpcResponse::error(
                        None,
                        JsonRpcError::invalid_request(format!("Parse error: {}", e)),
                    );
                    Self::write_response(&mut stdout_lock, &response)?;
                    continue;
                }
            };

            // Handle request
            let response = self.handle_request_async(request.clone()).await;

            // Only send response if request has an id (not a notification)
            // Per MCP spec, notifications have no id and don't get a response
            if request.id.is_some() {
                Self::write_response(&mut stdout_lock, &response)?;
            } else {
                debug!("Notification processed (no response sent)");
            }
        }

        info!("MCP server shutting down");
        Ok(())
    }

    fn write_response(writer: &mut dyn Write, response: &JsonRpcResponse) -> io::Result<()> {
        let json = serde_json::to_string(response)?;
        debug!("Sending: {}", json);
        writeln!(writer, "{}", json)?;
        writer.flush()
    }

    /// Get stats for a session, capturing from tmux if not in data store
    async fn get_session_stats(&self, session_name: &str) -> anyhow::Result<String> {
        use crate::capture::tmux::TmuxCommand;
        use crate::models::LogEntry;
        use crate::pipeline::parser::LogParser;
        use tokio::process::Command;

        // First try data store (if watch is running)
        if let Some(data) = self.data_store.get_session(session_name).await {
            let error_count = data
                .entries
                .iter()
                .filter(|e| {
                    matches!(
                        e.severity,
                        crate::models::Severity::Error | crate::models::Severity::Fatal
                    )
                })
                .count();
            return Ok(format!(
                "Session: {}\nTotal entries: {}\nErrors/Fatal: {}\nPatterns: {}\nIncidents: {}\nAlerts: {}\n(Source: live data)",
                session_name,
                data.entries.len(),
                error_count,
                data.patterns.len(),
                data.incidents.len(),
                data.alerts.len()
            ));
        }

        // Otherwise, capture a snapshot from tmux
        // Verify session exists
        if !TmuxCommand::session_exists(session_name).await? {
            return Ok(format!("Session '{}' not found", session_name));
        }

        // Get panes
        let panes = TmuxCommand::list_panes(session_name).await?;
        if panes.is_empty() {
            return Ok(format!("No panes found in session '{}'", session_name));
        }

        // Capture from each pane
        let mut total_entries = 0;
        let mut error_count = 0;
        let parser = LogParser::new();

        for pane in panes {
            let output = Command::new("tmux")
                .args(["capture-pane", "-p", "-t", &pane, "-S", "-100"])
                .output()
                .await?;

            if !output.status.success() {
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let mut entry = LogEntry::new(
                    uuid::Uuid::nil(),
                    total_entries as u64,
                    chrono::Utc::now(),
                    trimmed.to_string(),
                );
                parser.parse(&mut entry);
                total_entries += 1;

                if matches!(
                    entry.severity,
                    crate::models::Severity::Error | crate::models::Severity::Fatal
                ) {
                    error_count += 1;
                }
            }
        }

        Ok(format!(
            "Session: {}\nTotal entries: {}\nErrors/Fatal: {}\n(Source: snapshot capture)\nNote: Run 'logpilot watch {}' for live monitoring",
            session_name,
            total_entries,
            error_count,
            session_name
        ))
    }

    /// Search a session for pattern, capturing from tmux if not in data store
    async fn search_session(
        &self,
        session_name: &str,
        pattern: &str,
        severity: Option<&str>,
    ) -> anyhow::Result<String> {
        use crate::capture::tmux::TmuxCommand;
        use crate::models::LogEntry;
        use crate::pipeline::parser::LogParser;
        use tokio::process::Command;

        // First try data store (if watch is running)
        if let Some(data) = self.data_store.get_session(session_name).await {
            let matches: Vec<String> = data
                .entries
                .iter()
                .filter(|e| {
                    // Pattern match
                    let pattern_match = e.raw_content.contains(pattern);
                    // Severity filter (if specified)
                    let severity_match = if let Some(sev) = severity {
                        let entry_sev = format!("{:?}", e.severity).to_uppercase();
                        entry_sev == sev.to_uppercase()
                    } else {
                        true
                    };
                    pattern_match && severity_match
                })
                .take(50)
                .map(|e| format!("[{}] {}", e.timestamp, e.raw_content))
                .collect();

            if matches.is_empty() {
                return Ok("No matches found (Source: live data)".to_string());
            }
            return Ok(format!(
                "{} matches (Source: live data)\n{}",
                matches.len(),
                matches.join("\n")
            ));
        }

        // Otherwise, capture a snapshot from tmux
        // Verify session exists
        if !TmuxCommand::session_exists(session_name).await? {
            return Ok(format!("Session '{}' not found", session_name));
        }

        // Get panes
        let panes = TmuxCommand::list_panes(session_name).await?;
        if panes.is_empty() {
            return Ok(format!("No panes found in session '{}'", session_name));
        }

        // Capture from each pane
        let mut matches: Vec<String> = Vec::new();
        let parser = LogParser::new();

        for pane in panes {
            let output = Command::new("tmux")
                .args(["capture-pane", "-p", "-t", &pane, "-S", "-100"])
                .output()
                .await?;

            if !output.status.success() {
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Check pattern match first (cheaper than parsing)
                if !trimmed.contains(pattern) {
                    continue;
                }

                let mut entry = LogEntry::new(
                    uuid::Uuid::nil(),
                    0,
                    chrono::Utc::now(),
                    trimmed.to_string(),
                );
                parser.parse(&mut entry);

                // Severity filter (if specified)
                let severity_match = if let Some(sev) = severity {
                    let entry_sev = format!("{:?}", entry.severity).to_uppercase();
                    entry_sev == sev.to_uppercase()
                } else {
                    true
                };

                if severity_match {
                    matches.push(format!("[{}] {}", entry.timestamp, trimmed));
                    if matches.len() >= 50 {
                        break;
                    }
                }
            }

            if matches.len() >= 50 {
                break;
            }
        }

        if matches.is_empty() {
            return Ok("No matches found (Source: snapshot capture)\nNote: Run 'logpilot watch {}' for live monitoring".to_string());
        }

        Ok(format!(
            "{} matches (Source: snapshot capture)\n{}\n\nNote: Run 'logpilot watch {}' for live monitoring",
            matches.len(),
            matches.join("\n"),
            session_name
        ))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::data_store::SessionDataStore;
    use serde_json::json;

    #[test]
    fn test_handle_initialize() {
        let server = McpServer::new();
        let request = JsonRpcRequest::new("initialize", None);
        let response = server.handle_request(request);

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_handle_ping() {
        let server = McpServer::new();
        let request = JsonRpcRequest::new("ping", None);
        let response = server.handle_request(request);

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_handle_unknown_method() {
        let server = McpServer::new();
        let request = JsonRpcRequest::new("unknown_method", None);
        let response = server.handle_request(request);

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn test_handle_resources_list() {
        let server = McpServer::new();
        let request = JsonRpcRequest::new("resources/list", None);
        let response = server.handle_request(request);

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result.get("resources").is_some());
    }

    #[tokio::test]
    async fn test_read_session_from_data_store() {
        // Create a data store with test data
        let data_store = SessionDataStore::new();
        data_store.create_session("test-session").await;

        // Create server with the data store
        let server = McpServer::with_data_store(data_store);

        // Read resources
        let request = JsonRpcRequest::new(
            "resources/read",
            Some(json!({ "uri": "logpilot://session/test-session/summary" })),
        );
        let response = server.handle_request_async(request).await;

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_read_missing_session() {
        let server = McpServer::new();

        // Try to read a session that doesn't exist
        let request = JsonRpcRequest::new(
            "resources/read",
            Some(json!({ "uri": "logpilot://session/nonexistent-session/summary" })),
        );
        let response = server.handle_request_async(request).await;

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32002);
    }
}
