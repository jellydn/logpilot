//! MCP server command - start Model Context Protocol server
//!
//! Usage: logpilot mcp-server

use crate::mcp::McpServer;
use clap::Args;

/// Run LogPilot as an MCP server
#[derive(Args, Clone)]
pub struct McpArgs {
    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Handle the mcp-server command
pub async fn handle(args: McpArgs) -> anyhow::Result<()> {
    // Always print startup message so users know the server is starting
    eprintln!("[LogPilot] MCP server starting...");
    eprintln!("[LogPilot] Protocol: Model Context Protocol 2024-11-05");
    eprintln!("[LogPilot] Version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!("[LogPilot] Transport: stdio");
    eprintln!("[LogPilot] Resources: logpilot://session/{{name}}/summary, entries, patterns, incidents, alerts");

    // Create and run MCP server
    let server = McpServer::new();

    // In a real implementation, this would also:
    // - Connect to the capture system to get live data
    // - Spawn a background task to update session data
    // - Handle graceful shutdown

    // Print ready message - this is the key signal that the server is up
    eprintln!("[LogPilot] MCP server ready - waiting for connections");

    // Run stdio server
    server.run_stdio().await?;

    Ok(())
}
