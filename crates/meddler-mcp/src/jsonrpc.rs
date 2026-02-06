use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
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
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes.
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": null
        }"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, serde_json::json!(1));
    }

    #[test]
    fn success_response() {
        let resp = JsonRpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"tools": []}),
        );
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());

        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("error"));
    }

    #[test]
    fn error_response() {
        let resp = JsonRpcResponse::error(
            serde_json::json!(1),
            METHOD_NOT_FOUND,
            "Method not found",
        );
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());

        let err = resp.error.unwrap();
        assert_eq!(err.code, METHOD_NOT_FOUND);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn request_with_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "abc",
            "method": "tools/call",
            "params": {
                "name": "send_message",
                "arguments": {
                    "to": "researcher",
                    "content": "hello"
                }
            }
        }"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/call");

        let params = req.params.unwrap();
        assert_eq!(params["name"], "send_message");
        assert_eq!(params["arguments"]["to"], "researcher");
    }
}
