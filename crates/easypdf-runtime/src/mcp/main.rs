//! Binary entry point for the easypdf MCP server.
//!
//! Runs the MCP server on stdio. All logging goes to stderr
//! to avoid corrupting the JSON-RPC stream on stdout.

use easypdf_runtime::mcp::McpServer;

fn main() {
    // Initialize tracing subscriber (compact, stderr, RUST_LOG controlled).
    // Ignores error if subscriber is already set.
    easypdf_core::logging::init_logging().ok();

    if let Err(e) = McpServer::new().run() {
        tracing::error!(error = %e, "MCP server error");
        std::process::exit(1);
    }
}
