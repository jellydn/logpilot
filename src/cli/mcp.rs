//! MCP server command - start Model Context Protocol server
//!
//! Usage: logpilot mcp-server

use clap::Args;

/// Run LogPilot as an MCP server
#[derive(Args, Clone)]
pub struct McpArgs {
    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Handle the mcp-server command
pub async fn handle(_args: McpArgs) -> anyhow::Result<()> {
    // Legacy implementation (rmcp disabled due to Rust 1.86 compatibility)
    eprintln!("[LogPilot] MCP server starting...");
    eprintln!("[LogPilot] Protocol: Model Context Protocol 2024-11-05");
    eprintln!("[LogPilot] Version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!("[LogPilot] Transport: stdio");
    eprintln!("[LogPilot] Resources: logpilot://session/{{name}}/summary, entries, patterns, incidents, alerts");

    let server = crate::mcp::McpServer::new();
    eprintln!("[LogPilot] MCP server ready - waiting for connections");
    server.run_stdio().await?;

    Ok(())
}
