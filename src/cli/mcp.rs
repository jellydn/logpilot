//! MCP server command - start Model Context Protocol server
//!
//! Usage: logpilot mcp-server

use crate::mcp::run_mcp_server;
use clap::Args;

/// Run LogPilot as an MCP server
#[derive(Args, Clone)]
pub struct McpArgs {
    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Use legacy hand-rolled MCP implementation (deprecated)
    #[arg(long, hide = true)]
    pub legacy: bool,
}

/// Handle the mcp-server command
pub async fn handle(args: McpArgs) -> anyhow::Result<()> {
    if args.legacy {
        // Legacy implementation
        eprintln!("[LogPilot] MCP server starting (legacy mode)...");
        eprintln!("[LogPilot] Protocol: Model Context Protocol 2024-11-05");
        eprintln!("[LogPilot] Version: {}", env!("CARGO_PKG_VERSION"));
        eprintln!("[LogPilot] Transport: stdio");
        eprintln!("[LogPilot] Resources: logpilot://session/{{name}}/summary, entries, patterns, incidents, alerts");

        let server = crate::mcp::McpServer::new();
        eprintln!("[LogPilot] MCP server ready - waiting for connections");
        server.run_stdio().await?;
    } else {
        // Official rmcp SDK implementation
        // Note: rmcp handles its own initialization logging
        run_mcp_server().await?;
    }

    Ok(())
}
