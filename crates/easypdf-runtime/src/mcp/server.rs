//! MCP 服务器 -- stdio 主循环与 JSON-RPC 分发。
//!
//! 从 stdin 逐行读取 JSON-RPC 2.0 请求，将其分发到相应的处理器，
//! 并将响应写入 stdout。所有日志输出到 stderr 以避免破坏 JSON-RPC 流。

use std::io::{BufRead, Write};

use tracing::{debug, info, warn};

use super::error::Result;
use super::protocol::{
    self, JsonRpcRequest, JsonRpcResponse, METHOD_INITIALIZE, METHOD_NOTIFICATION_INITIALIZED,
    METHOD_PING, METHOD_TOOLS_CALL, METHOD_TOOLS_LIST, PROTOCOL_VERSION, SERVER_NAME,
    SERVER_VERSION,
};
use super::tools::{self, ToolResult};

/// easypdf MCP 服务器。
///
/// 通过 stdio 实现 MCP 协议，将 PDF 操作暴露为工具，
/// 供 LLM agent 发现和调用。
pub struct McpServer;

impl McpServer {
    /// 创建一个新的 MCP 服务器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 运行 stdio 主循环。
    ///
    /// 逐行从 stdin 读取 JSON-RPC 请求，进行分发，
    /// 并将响应写入 stdout。stdin 关闭（EOF）时退出。
    ///
    /// # Errors
    ///
    /// 仅在 stdout 写入失败（broken pipe 等）时返回错误。
    /// 单个请求的错误作为 JSON-RPC 错误响应报告。
    #[allow(clippy::unused_self)]
    pub fn run(&self) -> Result<()> {
        info!("MCP server starting on stdio");
        let stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();

        for line_result in stdin.lines() {
            let line = line_result?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let response = Self::handle_line(line);
            if let Some(resp) = response {
                let json = serde_json::to_string(&resp).map_err(|e| {
                    super::error::McpError::internal(format!("JSON serialization: {e}"))
                })?;
                writeln!(stdout, "{json}")?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    /// 处理一行 JSON-RPC 输入并返回响应
    /// （通知和空行返回 `None`）。
    fn handle_line(line: &str) -> Option<JsonRpcResponse> {
        if line.trim().is_empty() {
            return None;
        }
        match serde_json::from_str::<JsonRpcRequest>(line) {
            Ok(request) => Self::handle_request(&request),
            Err(e) => {
                // 解析错误 -- 无法可靠提取 id，使用 null。
                Some(protocol::error_response(
                    None,
                    protocol::ERROR_PARSE,
                    format!("Parse error: {e}"),
                ))
            }
        }
    }

    /// 分发已解析的 JSON-RPC 请求。
    fn handle_request(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // 通知（无 id）不接收响应。
        id.as_ref()?;

        debug!(method = %request.method, "JSON-RPC request");

        match request.method.as_str() {
            METHOD_INITIALIZE => Some(Self::handle_initialize(id)),
            METHOD_NOTIFICATION_INITIALIZED => None,
            METHOD_TOOLS_LIST => Some(Self::handle_tools_list(id)),
            METHOD_TOOLS_CALL => Some(Self::handle_tools_call(id, &request.params)),
            METHOD_PING => Some(Self::handle_ping(id)),
            _ => {
                warn!(method = %request.method, "unknown JSON-RPC method");
                Some(protocol::error_response(
                    id,
                    protocol::ERROR_METHOD_NOT_FOUND,
                    format!("Method not found: {}", truncate(&request.method, 64)),
                ))
            }
        }
    }

    /// 处理 `initialize` -- 返回服务器能力。
    fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
        let result = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            },
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            }
        });
        protocol::success_response(id, result)
    }

    /// 处理 `tools/list` -- 返回工具定义。
    fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
        let defs = tools::tool_definitions();
        let result = serde_json::json!({ "tools": defs });
        protocol::success_response(id, result)
    }

    /// 处理 `tools/call` -- 分发到相应的工具。
    fn handle_tools_call(
        id: Option<serde_json::Value>,
        params: &serde_json::Value,
    ) -> JsonRpcResponse {
        // 从 params 中提取工具名和参数。
        let Some(name) = params["name"].as_str() else {
            warn!("tools/call missing tool name");
            return protocol::error_response(
                id,
                protocol::ERROR_INVALID_PARAMS,
                "Missing tool name in params",
            );
        };

        info!(tool = name, "tool call");
        let args = &params["arguments"];

        match tools::dispatch_tool(name, args) {
            Ok(result) => {
                let result_json = serde_json::to_value(result).unwrap_or_else(|e| {
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!("Serialization error: {e}")}],
                        "isError": true
                    })
                });
                protocol::success_response(id, result_json)
            }
            Err(e) => {
                warn!(tool = name, error = %e, "tool call failed");
                // 工具级别的错误作为成功的 JSON-RPC 响应返回，
                // 其中 ToolResult 的 isError=true，而非 JSON-RPC 错误。
                // 这符合 MCP 惯例。
                let tool_result = ToolResult {
                    content: vec![super::tools::ContentBlock::Text {
                        text: e.to_string(),
                    }],
                    is_error: Some(true),
                };
                let result_json = serde_json::to_value(tool_result).unwrap_or_else(|se| {
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!("Error: {e}; serialization: {se}")}],
                        "isError": true
                    })
                });
                protocol::success_response(id, result_json)
            }
        }
    }

    /// 处理 `ping` -- 返回空结果。
    fn handle_ping(id: Option<serde_json::Value>) -> JsonRpcResponse {
        protocol::success_response(id, serde_json::json!({}))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// 将字符串截断到 `max_len` 个字符，截断时追加 `...`。
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_response() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
    }

    #[test]
    fn tools_list_returns_all_tools() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"pdf_read_text"));
        assert!(names.contains(&"pdf_to_markdown"));
        assert!(names.contains(&"pdf_create_text"));
        assert!(names.contains(&"pdf_merge"));
        assert!(names.contains(&"pdf_split"));
        assert!(names.contains(&"pdf_metadata"));
        assert!(names.contains(&"pdf_page_count"));
    }

    #[test]
    fn tools_call_missing_name() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: serde_json::json!({"arguments": {}}),
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, protocol::ERROR_INVALID_PARAMS);
    }

    #[test]
    fn tools_call_unknown_tool() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(4)),
            method: "tools/call".to_string(),
            params: serde_json::json!({"name": "nope", "arguments": {}}),
        };
        let resp = McpServer::handle_request(&req).unwrap();
        // 未知工具作为 isError=true 的成功响应返回
        // （工具级别错误，非 JSON-RPC 错误）。
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn tools_call_missing_path_returns_error_result() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(5)),
            method: "tools/call".to_string(),
            params: serde_json::json!({"name": "pdf_read_text", "arguments": {}}),
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing"));
    }

    #[test]
    fn ping_returns_empty_result() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(6)),
            method: "ping".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), serde_json::json!({}));
    }

    #[test]
    fn unknown_method_returns_error() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(7)),
            method: "something/else".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = McpServer::handle_request(&req).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, protocol::ERROR_METHOD_NOT_FOUND);
    }

    #[test]
    fn notification_returns_none() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: serde_json::Value::Null,
        };
        assert!(McpServer::handle_request(&req).is_none());
    }

    #[test]
    fn parse_error_returns_error_response() {
        let resp = McpServer::handle_line("not json");
        let resp = resp.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, protocol::ERROR_PARSE);
    }

    #[test]
    fn empty_line_returns_none() {
        assert!(McpServer::handle_line("").is_none());
        assert!(McpServer::handle_line("  ").is_none());
    }

    #[test]
    fn full_stdio_roundtrip() {
        // 1. initialize
        let init_req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::Value::Null,
        })
        .unwrap();
        let resp = McpServer::handle_line(&init_req).unwrap();
        assert_eq!(resp.id, Some(serde_json::json!(1)));
        assert!(resp.error.is_none());

        // 2. tools/list
        let list_req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: serde_json::Value::Null,
        })
        .unwrap();
        let resp = McpServer::handle_line(&list_req).unwrap();
        assert_eq!(resp.id, Some(serde_json::json!(2)));
        assert!(resp.error.is_none());

        // 3. tools/call -- 缺少 path 导致错误结果
        let call_req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: serde_json::json!({"name": "pdf_page_count", "arguments": {}}),
        })
        .unwrap();
        let resp = McpServer::handle_line(&call_req).unwrap();
        assert_eq!(resp.id, Some(serde_json::json!(3)));
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
    }
}
