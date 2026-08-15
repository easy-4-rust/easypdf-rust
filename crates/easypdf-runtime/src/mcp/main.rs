//! easypdf MCP 服务器的二进制入口。
//!
//! 通过标准输入输出运行 MCP 服务器。所有日志输出到标准错误，
//! 以避免污染标准输出上的 JSON-RPC 流。

use easypdf_runtime::mcp::McpServer;

fn main() {
    // 初始化 tracing 订阅者（紧凑格式、输出到 stderr、由 RUST_LOG 控制）。
    // 若订阅者已设置则忽略错误。
    easypdf_core::logging::init_logging().ok();

    if let Err(e) = McpServer::new().run() {
        tracing::error!(error = %e, "MCP server error");
        std::process::exit(1);
    }
}
