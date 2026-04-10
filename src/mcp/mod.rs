//! MCP (Model Context Protocol) module
//!
//! Provides AI context bridge for Claude Code / Codex integration
#![allow(dead_code)] // Infrastructure not yet fully implemented

pub mod data_store;
pub mod protocol;
pub mod resources;
pub mod server;

pub use server::McpServer;
