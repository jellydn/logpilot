//! MCP Protocol Compliance Tests
//!
//! Tests LogPilot's MCP server against the Model Context Protocol specification

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Test: MCP server responds to initialize request
#[test]
fn test_mcp_initialize() {
    // Start the MCP server
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let mut writer = stdin;
    writeln!(writer, "{}", init_request.to_string()).expect("Failed to write request");
    writer.flush().expect("Failed to flush");
    // Keep stdin open so the server doesn't see connection closed
    std::mem::forget(writer);

    // Read response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value =
        serde_json::from_str(&response_line).expect("Failed to parse response as JSON");

    // Validate response structure
    assert_eq!(response["jsonrpc"], "2.0", "JSON-RPC version must be 2.0");
    assert_eq!(response["id"], 1, "Response ID must match request ID");
    assert!(
        response["result"].is_object(),
        "Response must have result object"
    );

    // Validate result fields
    let result = response["result"].as_object().unwrap();
    assert!(
        result.contains_key("protocolVersion"),
        "Result must have protocolVersion"
    );
    assert!(
        result.contains_key("capabilities"),
        "Result must have capabilities"
    );
    assert!(
        result.contains_key("serverInfo"),
        "Result must have serverInfo"
    );

    // Validate protocol version
    assert_eq!(
        result["protocolVersion"], "2025-06-18",
        "Protocol version must be 2025-06-18"
    );

    // Validate server info
    let server_info = result["serverInfo"].as_object().unwrap();
    assert!(
        server_info.contains_key("name"),
        "Server info must have name"
    );
    assert!(
        server_info.contains_key("version"),
        "Server info must have version"
    );
    assert_eq!(
        server_info["name"], "logpilot",
        "Server name must be logpilot"
    );

    // Cleanup
    let _ = child.kill();
}

/// Test: MCP server responds to ping
#[test]
fn test_mcp_ping() {
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // Send ping request
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "ping"
    });

    let mut writer = stdin;
    writeln!(writer, "{}", ping_request.to_string()).expect("Failed to write request");
    writer.flush().expect("Failed to flush");
    // Keep stdin open so the server doesn't see connection closed
    std::mem::forget(writer);

    // Read response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value = serde_json::from_str(&response_line).expect("Failed to parse response");

    // Validate ping response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 42);
    assert!(
        response["result"].is_object() || response["result"].is_null(),
        "Ping response should have empty result or null"
    );
    assert!(response["error"].is_null(), "Ping should not return error");

    let _ = child.kill();
}

/// Test: MCP server handles unknown method
#[test]
fn test_mcp_unknown_method() {
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // First send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let mut writer = stdin;
    writeln!(writer, "{}", init_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Read initialize response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let _init_response = lines
        .next()
        .expect("No init response")
        .expect("Failed to read");

    // Send initialized notification
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(writer, "{}", initialized).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Send unknown method request
    let unknown_request = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "unknown/method"
    });
    writeln!(writer, "{}", unknown_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Keep stdin open
    std::mem::forget(writer);

    // Read response for unknown method
    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value = serde_json::from_str(&response_line).expect("Failed to parse response");

    // Validate error response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 99);
    assert!(
        response["result"].is_null(),
        "Error response should have null result"
    );
    assert!(
        response["error"].is_object(),
        "Error response must have error object"
    );

    let error = response["error"].as_object().unwrap();
    assert_eq!(
        error["code"], -32601,
        "Error code should be Method not found (-32601)"
    );
    assert!(
        error["message"].as_str().unwrap().contains("unknown"),
        "Error message should indicate unknown method"
    );

    let _ = child.kill();
}

/// Test: MCP server handles resources/list
#[test]
fn test_mcp_resources_list() {
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // First send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let mut writer = stdin;
    writeln!(writer, "{}", init_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Read initialize response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let _init_response = lines
        .next()
        .expect("No init response")
        .expect("Failed to read");

    // Send initialized notification
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(writer, "{}", initialized).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Send resources/list request
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list"
    });
    writeln!(writer, "{}", list_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");
    // Keep stdin open so the server doesn't see connection closed
    std::mem::forget(writer);

    // Read response

    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value = serde_json::from_str(&response_line).expect("Failed to parse response");

    // Validate response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object(), "Response must have result");

    let result = response["result"].as_object().unwrap();
    assert!(
        result.contains_key("resources"),
        "Result must contain resources array"
    );
    assert!(result["resources"].is_array(), "Resources must be an array");

    let _ = child.kill();
}

/// Test: MCP server handles tools/list
#[test]
fn test_mcp_tools_list() {
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // First send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let mut writer = stdin;
    writeln!(writer, "{}", init_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Read initialize response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let _init_response = lines
        .next()
        .expect("No init response")
        .expect("Failed to read");

    // Send initialized notification
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(writer, "{}", initialized).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Send tools/list request
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/list"
    });
    writeln!(writer, "{}", list_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");
    // Keep stdin open so the server doesn't see connection closed
    std::mem::forget(writer);

    // Read response
    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value = serde_json::from_str(&response_line).expect("Failed to parse response");

    // Validate response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);
    assert!(response["result"].is_object(), "Response must have result");

    let result = response["result"].as_object().unwrap();
    assert!(
        result.contains_key("tools"),
        "Result must contain tools array"
    );
    assert!(result["tools"].is_array(), "Tools must be an array");

    // The rmcp-based server now returns tools via list_tools
    let tools = result["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Tools array should not be empty");

    // Verify we have search and stats tools
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(tool_names.contains(&"search"), "Should have 'search' tool");
    assert!(tool_names.contains(&"stats"), "Should have 'stats' tool");

    let _ = child.kill();
}

/// Test: MCP server handles tools/call with unknown tool
#[test]
fn test_mcp_tools_call_unknown_tool() {
    let mut child = Command::new("./target/release/logpilot")
        .args(["mcp-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start MCP server");

    // Give the server time to start up
    thread::sleep(Duration::from_millis(100));

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // First send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let mut writer = stdin;
    writeln!(writer, "{}", init_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Read initialize response
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let _init_response = lines
        .next()
        .expect("No init response")
        .expect("Failed to read");

    // Send initialized notification
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(writer, "{}", initialized).expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Send tools/call request with unknown tool
    let call_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "unknown_tool",
            "arguments": {}
        }
    });
    writeln!(writer, "{}", call_request.to_string()).expect("Failed to write");
    writer.flush().expect("Failed to flush");
    // Keep stdin open so the server doesn't see connection closed
    std::mem::forget(writer);

    // Read response
    let response_line = lines
        .next()
        .expect("No response received")
        .expect("Failed to read line");

    let response: Value = serde_json::from_str(&response_line).expect("Failed to parse response");

    // Validate error response - rmcp returns method_not_found for unknown tools
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 5);
    assert!(
        response["error"].is_object(),
        "Error response must have error object"
    );

    let error = response["error"].as_object().unwrap();
    // Note: rmcp returns -32601 (Method not found) for unknown tool names
    assert_eq!(
        error["code"], -32601,
        "Error code should be Method not found (-32601) for unknown tool"
    );
    assert!(
        error["message"].as_str().unwrap().contains("tools/call"),
        "Error message should indicate unknown tool"
    );

    let _ = child.kill();
}
