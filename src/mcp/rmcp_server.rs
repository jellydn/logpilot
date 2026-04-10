//! MCP Server implementation using the official rmcp SDK
//!
//! This replaces the hand-rolled protocol implementation with the official
//! Model Context Protocol Rust SDK.

use crate::mcp::data_store::get_or_init_global_store;
use crate::models::Severity;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ErrorData, Implementation, ServerCapabilities, ServerInfo},
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::json,
    tool, tool_router, ServerHandler,
};

/// Search tool parameters
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct SearchParams {
    /// Session name to search
    pub session: String,
    /// Text pattern to search for
    pub pattern: String,
    /// Optional severity filter (ERROR, WARN, INFO, DEBUG)
    pub severity: Option<String>,
}

/// Stats tool parameters
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct StatsParams {
    /// Session name
    pub session: String,
}

/// LogPilot MCP Server using rmcp SDK
#[derive(Debug, Clone)]
pub struct LogPilotMcpServer {
    data_store: crate::mcp::data_store::SessionDataStore,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl LogPilotMcpServer {
    /// Create a new MCP server with the global data store
    pub fn new() -> Self {
        Self {
            data_store: get_or_init_global_store(),
            tool_router: Self::tool_router(),
        }
    }

    /// Search tool - searches log entries by text pattern
    #[tool(description = "Search log entries by text pattern")]
    async fn search(&self, params: Parameters<SearchParams>) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        tracing::info!("Search tool called for session: {}", params.session);

        if params.session.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "Missing required parameter: session",
                Some(json!({ "field": "session" })),
            ));
        }

        if params.pattern.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "Missing required parameter: pattern",
                Some(json!({ "field": "pattern" })),
            ));
        }

        // Get session data from store
        let session_data = self.data_store.get_session(&params.session).await;

        let results = if let Some(data) = session_data {
            let matches: Vec<String> = data
                .entries
                .iter()
                .filter(|e| {
                    // Pattern match
                    let pattern_match = e.raw_content.contains(&params.pattern);

                    // Severity filter (if specified)
                    let severity_match = if let Some(ref sev) = params.severity {
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
                "No matches found (Source: live data)".to_string()
            } else {
                format!(
                    "{} matches (Source: live data)\n{}",
                    matches.len(),
                    matches.join("\n")
                )
            }
        } else {
            // Session not found - try to capture snapshot from tmux
            match self
                .capture_and_search(&params.session, &params.pattern, params.severity.as_deref())
                .await
            {
                Ok(result) => result,
                Err(e) => format!("Session '{}' not found: {}", params.session, e),
            }
        };

        Ok(CallToolResult::success(vec![Content::text(results)]))
    }

    /// Stats tool - get session statistics
    #[tool(description = "Get session statistics")]
    async fn stats(&self, params: Parameters<StatsParams>) -> Result<CallToolResult, ErrorData> {
        let session = params.0.session;

        tracing::info!("Stats tool called for session: {}", session);

        if session.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "Missing required parameter: session",
                Some(json!({ "field": "session" })),
            ));
        }

        // Try data store first
        let stats = if let Some(data) = self.data_store.get_session(&session).await {
            let error_count = data
                .entries
                .iter()
                .filter(|e| matches!(e.severity, Severity::Error | Severity::Fatal))
                .count();

            format!(
                "Session: {}\nTotal entries: {}\nErrors/Fatal: {}\nPatterns: {}\nIncidents: {}\nAlerts: {}\n(Source: live data)",
                session,
                data.entries.len(),
                error_count,
                data.patterns.len(),
                data.incidents.len(),
                data.alerts.len()
            )
        } else {
            // Session not found - try to capture snapshot
            match self.capture_stats(&session).await {
                Ok(result) => result,
                Err(e) => format!("Session '{}' not found: {}", session, e),
            }
        };

        Ok(CallToolResult::success(vec![Content::text(stats)]))
    }

    /// Capture and search from tmux (fallback when session not in data store)
    async fn capture_and_search(
        &self,
        session_name: &str,
        pattern: &str,
        severity: Option<&str>,
    ) -> anyhow::Result<String> {
        use crate::capture::tmux::TmuxCommand;
        use crate::models::LogEntry;
        use crate::pipeline::parser::LogParser;
        use tokio::process::Command;

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
            return Ok(format!(
                "No matches found (Source: snapshot capture)\nNote: Run 'logpilot watch {}' for live monitoring",
                session_name
            ));
        }

        Ok(format!(
            "{} matches (Source: snapshot capture)\n{}\n\nNote: Run 'logpilot watch {}' for live monitoring",
            matches.len(),
            matches.join("\n"),
            session_name
        ))
    }

    /// Capture stats from tmux (fallback when session not in data store)
    async fn capture_stats(&self, session_name: &str) -> anyhow::Result<String> {
        use crate::capture::tmux::TmuxCommand;
        use crate::models::LogEntry;
        use crate::pipeline::parser::LogParser;
        use tokio::process::Command;

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

                if matches!(entry.severity, Severity::Error | Severity::Fatal) {
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
}

impl ServerHandler for LogPilotMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("logpilot", env!("CARGO_PKG_VERSION")))
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<rmcp::model::ListResourcesResult, rmcp::model::ErrorData>,
    > + Send
           + '_ {
        async move {
            use rmcp::model::{ListResourcesResult, RawResource, Resource};

            tracing::debug!("list_resources called");

            let resources = vec![
                Resource::new(
                    RawResource {
                        uri: "logpilot://session/{name}/summary".to_string(),
                        name: "Session Summary".to_string(),
                        title: None,
                        description: Some("Current incident summary for the session".to_string()),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                ),
                Resource::new(
                    RawResource {
                        uri: "logpilot://session/{name}/entries".to_string(),
                        name: "Log Entries".to_string(),
                        title: None,
                        description: Some("Log entries within a time range".to_string()),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                ),
                Resource::new(
                    RawResource {
                        uri: "logpilot://session/{name}/patterns".to_string(),
                        name: "Detected Patterns".to_string(),
                        title: None,
                        description: Some("Detected error patterns for the session".to_string()),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                ),
                Resource::new(
                    RawResource {
                        uri: "logpilot://session/{name}/incidents".to_string(),
                        name: "Active Incidents".to_string(),
                        title: None,
                        description: Some("Currently active incidents".to_string()),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                ),
                Resource::new(
                    RawResource {
                        uri: "logpilot://session/{name}/alerts".to_string(),
                        name: "Active Alerts".to_string(),
                        title: None,
                        description: Some("Currently firing alerts".to_string()),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                ),
            ];

            Ok(ListResourcesResult::with_all_items(resources))
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<
        Output = Result<rmcp::model::ListToolsResult, rmcp::model::ErrorData>,
    > + Send
           + '_ {
        async move {
            use rmcp::model::{ListToolsResult, Tool};
            use std::sync::Arc;

            tracing::debug!("list_tools called");

            let search_schema: Arc<rmcp::model::JsonObject> = Arc::new(
                serde_json::from_value(serde_json::json!({
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
                }))
                .unwrap(),
            );

            let stats_schema: Arc<rmcp::model::JsonObject> = Arc::new(
                serde_json::from_value(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "session": {
                            "type": "string",
                            "description": "Session name"
                        }
                    },
                    "required": ["session"]
                }))
                .unwrap(),
            );

            let tools = vec![
                Tool::new(
                    "search",
                    "Search log entries by text pattern",
                    search_schema,
                ),
                Tool::new("stats", "Get session statistics", stats_schema),
            ];

            Ok(ListToolsResult::with_all_items(tools))
        }
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::CallToolResult, rmcp::model::ErrorData>>
           + Send
           + '_ {
        async move {
            use rmcp::model::ErrorData as McpError;

            let name = request.name.as_ref();
            tracing::debug!("call_tool: {}", name);

            if name == "search" {
                let session = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("session"))
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing required parameter: session", None)
                    })?;

                let pattern = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("pattern"))
                    .and_then(|p| p.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing required parameter: pattern", None)
                    })?;

                let severity = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("severity"))
                    .and_then(|s| s.as_str());

                // Use the tool router instead
                let search_params = SearchParams {
                    session: session.to_string(),
                    pattern: pattern.to_string(),
                    severity: severity.map(|s| s.to_string()),
                };
                self.search(rmcp::handler::server::wrapper::Parameters(search_params))
                    .await
            } else if name == "stats" {
                let session = request
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("session"))
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing required parameter: session", None)
                    })?;

                // Use the tool router instead
                let stats_params = StatsParams {
                    session: session.to_string(),
                };
                self.stats(rmcp::handler::server::wrapper::Parameters(stats_params))
                    .await
            } else {
                Err(McpError::method_not_found::<
                    rmcp::model::CallToolRequestMethod,
                >())
            }
        }
    }
}

impl Default for LogPilotMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the MCP server using the official rmcp SDK
pub async fn run_mcp_server() -> anyhow::Result<()> {
    use rmcp::service::serve_server;
    use rmcp::transport::stdio;

    tracing::info!("Starting LogPilot MCP server");
    tracing::info!("Protocol: Model Context Protocol 2025-06-18");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Transport: stdio");

    let server = LogPilotMcpServer::new();
    tracing::info!("Server initialized, waiting for connections");

    let service = serve_server(server, stdio()).await?;
    tracing::info!("MCP server started successfully");

    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = LogPilotMcpServer::new();
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some());
    }
}
