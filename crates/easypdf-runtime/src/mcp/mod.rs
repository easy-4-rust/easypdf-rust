//! MCP (Model Context Protocol) server for easypdf.
//!
//! Exposes PDF operations to LLM agents via the standard MCP protocol over stdio.

pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;

pub use error::{McpError, Result};
pub use server::McpServer;
pub use tools::{ContentBlock, ToolDefinition, ToolResult};
