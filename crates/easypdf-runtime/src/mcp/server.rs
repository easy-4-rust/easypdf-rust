//! MCP server -- stdio main loop and JSON-RPC dispatch.
//!
//! Reads JSON-RPC 2.0 requests from stdin (one per line), dispatches them
//! to the appropriate handler, and writes responses to stdout. All logging
//! goes to stderr to avoid corrupting the JSON-RPC stream.

use std::io::{BufRead, Write};

use tracing::{debug, info, warn};

use super::error::Result;
use super::protocol::{
    self, JsonRpcRequest, JsonRpcResponse, METHOD_INITIALIZE, METHOD_NOTIFICATION_INITIALIZED,
    METHOD_PING, METHOD_TOOLS_CALL, METHOD_TOOLS_LIST, PROTOCOL_VERSION, SERVER_NAME,
    SERVER_VERSION,
};
use super::tools::{self, ToolResult};

/// The easypdf MCP server.
///
/// Implements the MCP protocol over stdio, exposing PDF operations as
/// tools that LLM agents can discover and invoke.
pub struct McpServer;

impl McpServer {
    /// Create a new MCP server.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Run the main stdio loop.
    ///
    /// Reads JSON-RPC requests from stdin line by line, dispatches them,
    /// and writes responses to stdout. Exits when stdin is closed (EOF).
    ///
    /// # Errors
    ///
    /// Returns an error only if stdout writes fail (broken pipe, etc.).
    /// Individual request errors are reported as JSON-RPC error responses.
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

    /// Process a single line of JSON-RPC input and return a response
    /// (or `None` for notifications and empty lines).
    fn handle_line(line: &str) -> Option<JsonRpcResponse> {
        if line.trim().is_empty() {
            return None;
        }
        match serde_json::from_str::<JsonRpcRequest>(line) {
            Ok(request) => Self::handle_request(&request),
            Err(e) => {
                // Parse error -- cannot reliably extract id, use null.
                Some(protocol::error_response(
                    None,
                    protocol::ERROR_PARSE,
                    format!("Parse error: {e}"),
                ))
            }
        }
    }

    /// Dispatch a parsed JSON-RPC request.
    fn handle_request(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // Notifications (no id) do not receive a response.
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

    /// Handle `initialize` -- return server capabilities.
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

    /// Handle `tools/list` -- return tool definitions.
    fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
        let defs = tools::tool_definitions();
        let result = serde_json::json!({ "tools": defs });
        protocol::success_response(id, result)
    }

    /// Handle `tools/call` -- dispatch to the appropriate tool.
    fn handle_tools_call(
        id: Option<serde_json::Value>,
        params: &serde_json::Value,
    ) -> JsonRpcResponse {
        // Extract tool name and arguments from params.
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
                // Tool-level errors are returned as a successful JSON-RPC
                // response with isError=true in the ToolResult, NOT as a
                // JSON-RPC error. This matches MCP convention.
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

    /// Handle `ping` -- return empty result.
    fn handle_ping(id: Option<serde_json::Value>) -> JsonRpcResponse {
        protocol::success_response(id, serde_json::json!({}))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate a string to `max_len` characters, appending `...` if truncated.
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
        // Unknown tool returns as a successful response with isError=true
        // (tool-level error, not JSON-RPC error).
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
        // handle_line is called with trimmed non-empty lines in run(),
        // but test the trim path anyway.
        assert!(McpServer::handle_line("").is_none());
        assert!(McpServer::handle_line("  ").is_none());
    }

    #[test]
    fn full_stdio_roundtrip() {
        // Simulate a full initialize -> tools/list -> tools/call flow
        // by calling handle_line with serialized requests.

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

        // 3. tools/call with missing path -> error result
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
