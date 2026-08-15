//! easypdf 的 MCP（Model Context Protocol）服务器。
//!
//! 通过标准 MCP 协议（基于 stdio）向 LLM agent 暴露 PDF 操作。

pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;

pub use error::{McpError, Result};
pub use server::McpServer;
pub use tools::{ContentBlock, ToolDefinition, ToolResult};
