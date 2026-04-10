//! MCP Server implementation for LogPilot
//!
//! Provides stdio-based JSON-RPC communication for AI context exchange

use crate::mcp::protocol::*;
use crate::mcp::resources::ResourceHandler;
use crate::models::{Alert, Incident, LogEntry, Pattern};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// MCP Server state and request handler
pub struct McpServer {
    /// Session data storage
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

/// Data for a single monitored session
#[derive(Debug, Clone, Default)]
pub struct SessionData {
    pub entries: Vec<LogEntry>,
    pub patterns: Vec<Pattern>,
    pub incidents: Vec<Incident>,
    pub alerts: Vec<Alert>,
    pub window_start: chrono::DateTime<chrono::Utc>,
}

impl McpServer {
    /// Create a new MCP server
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
        };

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            capabilities,
        }
    }

    /// Update session data (called from watch command)
    pub async fn update_session(
        &self,
        name: String,
        entries: Vec<LogEntry>,
        patterns: Vec<Pattern>,
        incidents: Vec<Incident>,
        alerts: Vec<Alert>,
    ) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            name,
            SessionData {
                entries,
                patterns,
                incidents,
                alerts,
                window_start: Utc::now() - chrono::Duration::minutes(30),
            },
        );
    }

    /// Handle a single JSON-RPC request and return response
    pub fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params),
            "resources/list" => self.handle_resources_list(id),
            "resources/read" => self.handle_resources_read(id, request.params),
            "ping" => JsonRpcResponse::success(id, json!({})),
            _ => {
                warn!("Unknown method: {}", request.method);
                JsonRpcResponse::error(id, JsonRpcError::method_not_found(&request.method))
            }
        }
    }

    /// Handle async request (for resources/read that needs session data)
    pub async fn handle_request_async(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "resources/read" => self.handle_resources_read_async(id, request.params).await,
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

        // Get session data
        let sessions = self.sessions.read().await;
        let session_data = match sessions.get(&parsed.session_name) {
            Some(data) => data.clone(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError {
                        code: -32002,
                        message: format!("Session '{}' not found", parsed.session_name),
                        data: None,
                    },
                );
            }
        };
        drop(sessions); // Release lock

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
            "entries" => {
                ResourceHandler::build_entries(&parsed.session_name, &session_data.entries)
            }
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
            let response = self.handle_request_async(request).await;

            // Write response
            Self::write_response(&mut stdout_lock, &response)?;
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
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_update_and_read_session() {
        let server = McpServer::new();

        // Update session data
        server
            .update_session("test-session".to_string(), vec![], vec![], vec![], vec![])
            .await;

        // Read resources
        let request = JsonRpcRequest::new(
            "resources/read",
            Some(json!({ "uri": "logpilot://session/test-session/summary" })),
        );
        let response = server.handle_request_async(request).await;

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }
}
