use mcp_sdk::protocol::*;
use serde_json::json;

#[test]
fn test_jsonrpc_request_serialization() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(1),
        method: "tools/call".into(),
        params: Some(json!({"name": "echo", "arguments": {"msg": "hi"}})),
    };
    let serialized = serde_json::to_string(&req).unwrap();
    assert!(serialized.contains(r#""jsonrpc":"2.0""#));
    assert!(serialized.contains(r#""id":1"#));
    assert!(serialized.contains(r#""method":"tools/call""#));
}

#[test]
fn test_jsonrpc_request_deserialization() {
    let raw = r#"{"jsonrpc":"2.0","id":42,"method":"ping"}"#;
    let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.method, "ping");
    assert_eq!(req.id, RequestId::Number(42));
    assert!(req.params.is_none());
}

#[test]
fn test_jsonrpc_response_success() {
    let resp = JsonRpcResponse::success(RequestId::Number(1), json!({"ok": true}));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    assert_eq!(resp.id, RequestId::Number(1));
}

#[test]
fn test_jsonrpc_response_error() {
    let resp = JsonRpcResponse::error(
        RequestId::String("abc".into()),
        -32601,
        "Method not found".into(),
        Some(json!({"method": "foo"})),
    );
    assert!(resp.result.is_none());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
    assert_eq!(err.message, "Method not found");
}

#[test]
fn test_request_id_number() {
    let id = RequestId::Number(5);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "5");
    let back: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn test_request_id_string() {
    let id = RequestId::String("req-abc".into());
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, r#""req-abc""#);
    let back: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn test_notification_serialization() {
    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "notifications/initialized".into(),
        params: None,
    };
    let json = serde_json::to_string(&notif).unwrap();
    assert!(!json.contains(r#""id""#));
    assert!(json.contains(r#""method":"notifications/initialized""#));
}

#[test]
fn test_server_info_serialization() {
    let info = ServerInfo {
        name: "test-server".into(),
        version: "1.0.0".into(),
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: ServerInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "test-server");
    assert_eq!(back.version, "1.0.0");
}

#[test]
fn test_initialize_result_serialization() {
    let result = InitializeResult {
        protocol_version: "2024-11-05".into(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability {}),
            ..Default::default()
        },
        server_info: ServerInfo {
            name: "srv".into(),
            version: "0.1".into(),
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""protocolVersion":"2024-11-05""#));
    assert!(json.contains(r#""tools":{}"#));
}

#[test]
fn test_tool_serialization() {
    let tool = Tool {
        name: "echo".into(),
        description: Some("Echo a message".into()),
        input_schema: json!({"type": "object", "properties": {}}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains(r#""name":"echo""#));
    assert!(json.contains(r#""inputSchema""#));
    assert!(json.contains(r#""description":"Echo a message""#));
}

#[test]
fn test_tool_optional_description_omitted() {
    let tool = Tool {
        name: "ping".into(),
        description: None,
        input_schema: json!({"type": "object"}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    assert!(!json.contains(r#""description""#));
}

#[test]
fn test_call_tool_result_text() {
    let result = CallToolResult::text("hello world");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "hello world"),
        _ => panic!("Expected Text content"),
    }
}

#[test]
fn test_call_tool_result_error() {
    let result = CallToolResult::error("something broke");
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[test]
fn test_call_tool_result_multiple_content() {
    let result = CallToolResult {
        content: vec![
            Content::text("line 1"),
            Content::text("line 2"),
        ],
        is_error: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: CallToolResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.content.len(), 2);
}

#[test]
fn test_call_tool_params_deserialization() {
    let raw = r#"{"name":"echo","arguments":{"message":"hi"}}"#;
    let params: CallToolParams = serde_json::from_str(raw).unwrap();
    assert_eq!(params.name, "echo");
    assert_eq!(params.arguments["message"], "hi");
}

#[test]
fn test_call_tool_params_default_arguments() {
    let raw = r#"{"name":"ping"}"#;
    let params: CallToolParams = serde_json::from_str(raw).unwrap();
    assert_eq!(params.name, "ping");
    assert!(params.arguments.is_null() || params.arguments == json!({}));
}

#[test]
fn test_list_tools_result() {
    let result = ListToolsResult {
        tools: vec![
            Tool {
                name: "a".into(),
                description: None,
                input_schema: json!({}),
            },
            Tool {
                name: "b".into(),
                description: Some("tool b".into()),
                input_schema: json!({}),
            },
        ],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""name":"a""#));
    assert!(json.contains(r#""name":"b""#));
}

#[test]
fn test_content_text_variant() {
    let content = Content::text("output");
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains(r#""type":"text""#));
    assert!(json.contains(r#""text":"output""#));
}

#[test]
fn test_content_image_variant() {
    let content = Content::Image {
        data: "base64data".into(),
        mime_type: "image/png".into(),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains(r#""type":"image""#));
    assert!(json.contains(r#""mimeType":"image/png""#));
}

#[test]
fn test_protocol_version_as_str() {
    assert_eq!(ProtocolVersion::V20241105.as_str(), "2024-11-05");
    assert_eq!(ProtocolVersion::Latest.as_str(), "2024-11-05");
}
