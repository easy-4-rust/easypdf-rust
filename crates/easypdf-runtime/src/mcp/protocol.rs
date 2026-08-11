//! JSON-RPC 2.0 types and MCP protocol constants.
//!
//! Defines the wire format for MCP communication over stdio.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 types
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 request received from the client (LLM agent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version -- must be `"2.0"`.
    pub jsonrpc: String,
    /// Request identifier (null for notifications).
    pub id: Option<serde_json::Value>,
    /// The method name (e.g. `"initialize"`, `"tools/call"`).
    pub method: String,
    /// Method parameters -- defaults to `null` when absent.
    #[serde(default = "serde_json::Value::default")]
    pub params: serde_json::Value,
}

/// A JSON-RPC 2.0 response sent back to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version -- always `"2.0"`.
    pub jsonrpc: String,
    /// Echoed request identifier.
    pub id: Option<serde_json::Value>,
    /// Successful result value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error object (mutually exclusive with `result`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// MCP protocol method names
// ---------------------------------------------------------------------------

/// `initialize` -- handshake between client and server.
pub const METHOD_INITIALIZE: &str = "initialize";

/// `notifications/initialized` -- client confirms initialization (notification, no id).
pub const METHOD_NOTIFICATION_INITIALIZED: &str = "notifications/initialized";

/// `tools/list` -- returns the list of available tools.
pub const METHOD_TOOLS_LIST: &str = "tools/list";

/// `tools/call` -- invokes a specific tool.
pub const METHOD_TOOLS_CALL: &str = "tools/call";

/// `ping` -- liveness check.
pub const METHOD_PING: &str = "ping";

// ---------------------------------------------------------------------------
// JSON-RPC standard error codes
// ---------------------------------------------------------------------------

/// Parse error -- invalid JSON.
pub const ERROR_PARSE: i32 = -32700;

/// Invalid Request -- not a valid JSON-RPC 2.0 request.
pub const ERROR_INVALID_REQUEST: i32 = -32600;

/// Method not found.
pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;

/// Invalid method parameters.
pub const ERROR_INVALID_PARAMS: i32 = -32602;

/// Internal JSON-RPC error.
pub const ERROR_INTERNAL: i32 = -32603;

// ---------------------------------------------------------------------------
// MCP protocol version
// ---------------------------------------------------------------------------

/// The MCP protocol version this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name reported during initialization.
pub const SERVER_NAME: &str = "easypdf-mcp";

/// Server version reported during initialization.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a successful JSON-RPC response.
#[must_use]
pub fn success_response(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

/// Build an error JSON-RPC response.
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
        let resp = error_response(Some(serde_json::json!(1)), ERROR_METHOD_NOT_FOUND, "no such method");
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
