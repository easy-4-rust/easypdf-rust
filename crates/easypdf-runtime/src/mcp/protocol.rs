//! JSON-RPC 2.0 类型和 MCP 协议常量。
//!
//! 定义通过 stdio 进行 MCP 通信的线路格式。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 类型
// ---------------------------------------------------------------------------

/// 从客户端（LLM agent）接收的 JSON-RPC 2.0 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// 协议版本 -- 必须为 `"2.0"`。
    pub jsonrpc: String,
    /// 请求标识符（通知时为 null）。
    pub id: Option<serde_json::Value>,
    /// 方法名（例如 `"initialize"`、`"tools/call"`）。
    pub method: String,
    /// 方法参数 -- 缺失时默认为 `null`。
    #[serde(default = "serde_json::Value::default")]
    pub params: serde_json::Value,
}

/// 返回给客户端的 JSON-RPC 2.0 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// 协议版本 -- 始终为 `"2.0"`。
    pub jsonrpc: String,
    /// 回显的请求标识符。
    pub id: Option<serde_json::Value>,
    /// 成功时的结果值。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 错误对象（与 `result` 互斥）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 错误对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// 数字错误码。
    pub code: i32,
    /// 人类可读的错误消息。
    pub message: String,
    /// 可选的附加错误数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// MCP 协议方法名
// ---------------------------------------------------------------------------

/// `initialize` -- 客户端与服务器之间的握手。
pub const METHOD_INITIALIZE: &str = "initialize";

/// `notifications/initialized` -- 客户端确认初始化完成（通知，无 id）。
pub const METHOD_NOTIFICATION_INITIALIZED: &str = "notifications/initialized";

/// `tools/list` -- 返回可用工具列表。
pub const METHOD_TOOLS_LIST: &str = "tools/list";

/// `tools/call` -- 调用特定工具。
pub const METHOD_TOOLS_CALL: &str = "tools/call";

/// `ping` -- 存活检查。
pub const METHOD_PING: &str = "ping";

// ---------------------------------------------------------------------------
// JSON-RPC 标准错误码
// ---------------------------------------------------------------------------

/// 解析错误 -- 无效的 JSON。
pub const ERROR_PARSE: i32 = -32700;

/// 无效请求 -- 不是有效的 JSON-RPC 2.0 请求。
pub const ERROR_INVALID_REQUEST: i32 = -32600;

/// 方法未找到。
pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;

/// 无效的方法参数。
pub const ERROR_INVALID_PARAMS: i32 = -32602;

/// 内部 JSON-RPC 错误。
pub const ERROR_INTERNAL: i32 = -32603;

// ---------------------------------------------------------------------------
// MCP 协议版本
// ---------------------------------------------------------------------------

/// 此服务器实现的 MCP 协议版本。
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// 初始化期间报告的服务器名称。
pub const SERVER_NAME: &str = "easypdf-mcp";

/// 初始化期间报告的服务器版本。
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构建成功的 JSON-RPC 响应。
#[must_use]
pub fn success_response(
    id: Option<serde_json::Value>,
    result: serde_json::Value,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

/// 构建错误的 JSON-RPC 响应。
#[must_use]
pub fn error_response(
    id: Option<serde_json::Value>,
    code: i32,
    message: impl Into<String>,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        let back = serde_json::to_string(&req).unwrap();
        let req2: JsonRpcRequest = serde_json::from_str(&back).unwrap();
        assert_eq!(req2.method, req.method);
    }

    #[test]
    fn request_defaults_params_to_null() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.params, serde_json::Value::Null);
    }

    #[test]
    fn response_with_result() {
        let resp = success_response(Some(serde_json::json!(42)), serde_json::json!({"ok": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn response_with_error() {
        let resp = error_response(
            Some(serde_json::json!(1)),
            ERROR_METHOD_NOT_FOUND,
            "no such method",
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
        assert!(json.contains("-32601"));
    }

    #[test]
    fn error_code_constants() {
        assert_eq!(ERROR_PARSE, -32700);
        assert_eq!(ERROR_INVALID_REQUEST, -32600);
        assert_eq!(ERROR_METHOD_NOT_FOUND, -32601);
        assert_eq!(ERROR_INVALID_PARAMS, -32602);
        assert_eq!(ERROR_INTERNAL, -32603);
    }
}
