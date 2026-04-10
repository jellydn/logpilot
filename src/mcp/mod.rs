//! MCP (Model Context Protocol) module
//!
//! Provides AI context bridge for Claude Code / Codex integration
#![allow(dead_code)] // Infrastructure not yet fully implemented

pub mod data_store;
pub mod protocol;
pub mod resources;
pub mod rmcp_server;
pub mod server;

pub use rmcp_server::run_mcp_server;
pub use server::McpServer;
