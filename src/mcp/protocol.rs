//! MCP (Model Context Protocol) JSON-RPC 2.0 message types
//!
//! Based on the MCP specification for resource-based AI context exchange

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    // Note: id should be omitted entirely if None, not sent as null
    // The MCP SDK schema only accepts string or number for id, not null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

/// MCP Server capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub resources: ResourceCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub supported_uris: Vec<String>,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// JSON-encoded resource data
    pub text: String,
}

/// Initialize request params
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(default)]
    pub resources: Option<Value>,
}

/// Initialize response result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Resources/list result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
}

/// Resources/read params
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesReadParams {
    pub uri: String,
}

/// Resources/read result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesReadResult {
    pub contents: Vec<ResourceContent>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Tools/list result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

/// Tools/call params
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallParams {
    pub name: String,
    pub arguments: Option<Value>,
}

/// Tools/call result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool response content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::from(1)),
            method: method.into(),
            params,
        }
    }

    pub fn new_with_id(
        id: impl Into<Value>,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: method.into(),
            params,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let request = JsonRpcRequest::new("initialize", None);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_jsonrpc_response_serialization() {
        let result = serde_json::json!({ "status": "ok" });
        let response = JsonRpcResponse::success(Some(Value::from(1)), result);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_jsonrpc_error_creation() {
        let err = JsonRpcError::method_not_found("test_method");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("test_method"));
    }

    #[test]
    fn test_resource_serialization() {
        let resource = Resource {
            uri: "logpilot://session/test/summary".to_string(),
            name: "Session Summary".to_string(),
            description: Some("Current incident summary".to_string()),
            mime_type: Some("application/json".to_string()),
        };
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("logpilot://session/test/summary"));
    }

    // Additional MCP Protocol Compliance Tests

    #[test]
    fn test_jsonrpc_request_with_null_id() {
        // Server may send notifications with null id
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "ping".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"ping\""));
    }

    #[test]
    fn test_jsonrpc_request_with_string_id() {
        // Some clients use string IDs
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::from("req-123")),
            method: "resources/list".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"id\":\"req-123\""));
    }

    #[test]
    fn test_jsonrpc_request_with_numeric_id() {
        // Standard numeric ID
        let request = JsonRpcRequest::new_with_id(42, "resources/read", None);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"id\":42"));
    }

    #[test]
    fn test_jsonrpc_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "initialize");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_jsonrpc_response_deserialization() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05"},"error":null}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_with_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null,"error":{"code":-32601,"message":"Method not found","data":null}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_jsonrpc_error_codes() {
        // Standard JSON-RPC 2.0 error codes
        let invalid_request = JsonRpcError::invalid_request("Missing jsonrpc field");
        assert_eq!(invalid_request.code, -32600);

        let invalid_params = JsonRpcError::invalid_params("Expected string");
        assert_eq!(invalid_params.code, -32602);

        let internal_error = JsonRpcError::internal_error("Database connection failed");
        assert_eq!(internal_error.code, -32603);
    }

    #[test]
    fn test_jsonrpc_response_null_result() {
        // Response with null result (valid case for some methods)
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::from(1)),
            result: Some(Value::Null),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"result\":null"));
    }

    #[test]
    fn test_resource_optional_fields() {
        // Resource with minimal fields (no description, no mime_type)
        let resource = Resource {
            uri: "logpilot://session/test/entries".to_string(),
            name: "Entries".to_string(),
            description: None,
            mime_type: None,
        };
        let json = serde_json::to_string(&resource).unwrap();
        assert!(!json.contains("description")); // Should be skipped
        assert!(!json.contains("mimeType")); // Should be skipped
    }

    #[test]
    fn test_initialize_result_serialization() {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                resources: ResourceCapabilities {
                    supported_uris: vec!["logpilot://session/{name}/summary".to_string()],
                },
            },
            server_info: ServerInfo {
                name: "logpilot".to_string(),
                version: "0.1.0".to_string(),
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("2024-11-05"));
        assert!(json.contains("serverInfo"));
        assert!(json.contains("logpilot"));
    }

    #[test]
    fn test_resource_content_serialization() {
        let content = ResourceContent {
            uri: "logpilot://session/test/summary".to_string(),
            mime_type: Some("application/json".to_string()),
            text: r#"{"total_entries":100}"#.to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"text\":\"{\\\"total_entries\\\":100}\""));
    }

    #[test]
    fn test_resources_read_params_deserialization() {
        let json = r#"{"uri":"logpilot://session/test/summary"}"#;
        let params: ResourcesReadParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.uri, "logpilot://session/test/summary");
    }

    #[test]
    fn test_resources_list_result_serialization() {
        let result = ResourcesListResult {
            resources: vec![Resource {
                uri: "logpilot://session/test/summary".to_string(),
                name: "Summary".to_string(),
                description: None,
                mime_type: None,
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("resources"));
    }
}
