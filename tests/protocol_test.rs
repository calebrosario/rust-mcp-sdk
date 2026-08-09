use mcp_sdk::protocol::*;
use serde_json::json;

#[test]
fn test_jsonrpc_request_serialization() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(1),
        method: "test".into(),
        params: Some(json!({"key": "value"})),
    };
    let json_str = serde_json::to_string(&request).unwrap();
    assert!(json_str.contains(r#""jsonrpc":"2.0""#));
    assert!(json_str.contains(r#""method":"test""#));
    assert!(json_str.contains(r#""params":{"key":"value"}"#));
}

#[test]
fn test_jsonrpc_request_serialization_without_params() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(1),
        method: "test".into(),
        params: None,
    };
    let json_str = serde_json::to_string(&request).unwrap();
    assert!(!json_str.contains("params"));
}

#[test]
fn test_jsonrpc_request_deserialization() {
    let json_str = r#"{"jsonrpc":"2.0","id":42,"method":"ping","params":{"x":1}}"#;
    let request: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
    assert_eq!(request.method, "ping");
    match request.id {
        RequestId::Number(n) => assert_eq!(n, 42),
        _ => panic!("Expected Number id"),
    }
    assert!(request.params.is_some());
}

#[test]
fn test_jsonrpc_request_deserialization_without_params() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let request: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
    assert!(request.params.is_none());
}

#[test]
fn test_jsonrpc_request_string_id_deserialization() {
    let json_str = r#"{"jsonrpc":"2.0","id":"abc","method":"test"}"#;
    let request: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
    match request.id {
        RequestId::String(s) => assert_eq!(s, "abc"),
        _ => panic!("Expected String id"),
    }
}

#[test]
fn test_jsonrpc_notification_serialization() {
    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "progress".into(),
        params: Some(json!({"percent": 50})),
    };
    let json_str = serde_json::to_string(&notif).unwrap();
    assert!(json_str.contains(r#""method":"progress""#));
    assert!(json_str.contains(r#""params":{"percent":50}"#));
}

#[test]
fn test_jsonrpc_notification_serialization_without_params() {
    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "initialized".into(),
        params: None,
    };
    let json_str = serde_json::to_string(&notif).unwrap();
    assert!(!json_str.contains("params"));
}

#[test]
fn test_jsonrpc_notification_deserialization() {
    let json_str = r#"{"jsonrpc":"2.0","method":"initialized","params":null}"#;
    let notif: JsonRpcNotification = serde_json::from_str(json_str).unwrap();
    assert_eq!(notif.method, "initialized");
}

#[test]
fn test_jsonrpc_response_success() {
    let response = JsonRpcResponse::success(RequestId::Number(1), json!({"ok": true}));
    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains(r#""result":{"ok":true}"#));
    assert!(!json_str.contains("error"));
}

#[test]
fn test_jsonrpc_response_error() {
    let response = JsonRpcResponse::error(
        RequestId::Number(2),
        -32601,
        "Method not found".into(),
        None,
    );
    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains(r#""code":-32601"#));
    assert!(json_str.contains(r#""message":"Method not found""#));
    assert!(!json_str.contains(r#""result""#));
}

#[test]
fn test_jsonrpc_response_error_with_data() {
    let response = JsonRpcResponse::error(
        RequestId::Number(3),
        -32602,
        "Invalid params".into(),
        Some(json!({"field": "name"})),
    );
    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains(r#""data":{"field":"name"}"#));
}

#[test]
fn test_jsonrpc_response_deserialization_success() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"result":{"value":42}}"#;
    let response: JsonRpcResponse = serde_json::from_str(json_str).unwrap();
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_jsonrpc_response_deserialization_error() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#;
    let response: JsonRpcResponse = serde_json::from_str(json_str).unwrap();
    assert!(response.result.is_none());
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32700);
}

#[test]
fn test_jsonrpc_response_both_none_serialization() {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(1),
        result: None,
        error: None,
    };
    let json_str = serde_json::to_string(&response).unwrap();
    assert!(!json_str.contains("result"));
    assert!(!json_str.contains("error"));
}

#[test]
fn test_jsonrpc_error_serialization() {
    let error = JsonRpcError {
        code: -32601,
        message: "Method not found".into(),
        data: Some(json!({"method": "foo"})),
    };
    let json_str = serde_json::to_string(&error).unwrap();
    assert!(json_str.contains(r#""code":-32601"#));
    assert!(json_str.contains(r#""data":{"method":"foo"}"#));
}

#[test]
fn test_jsonrpc_error_serialization_without_data() {
    let error = JsonRpcError {
        code: -32700,
        message: "Parse error".into(),
        data: None,
    };
    let json_str = serde_json::to_string(&error).unwrap();
    assert!(!json_str.contains("data"));
}

#[test]
fn test_jsonrpc_error_deserialization() {
    let json_str = r#"{"code":-32602,"message":"Invalid params","data":{"key":"val"}}"#;
    let error: JsonRpcError = serde_json::from_str(json_str).unwrap();
    assert_eq!(error.code, -32602);
    assert!(error.data.is_some());
}

#[test]
fn test_request_id_number() {
    let id = RequestId::Number(42);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "42");

    let deserialized: RequestId = serde_json::from_str("42").unwrap();
    match deserialized {
        RequestId::Number(n) => assert_eq!(n, 42),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn test_request_id_string() {
    let id = RequestId::String("abc-123".into());
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, r#""abc-123""#);

    let deserialized: RequestId = serde_json::from_str(r#""abc-123""#).unwrap();
    match deserialized {
        RequestId::String(s) => assert_eq!(s, "abc-123"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_request_id_null() {
    let id = RequestId::null();
    match id {
        RequestId::Number(0) => {}
        _ => panic!("Expected Number(0)"),
    }
}

#[test]
fn test_request_id_from_i64() {
    let id: RequestId = 99i64.into();
    match id {
        RequestId::Number(n) => assert_eq!(n, 99),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn test_request_id_from_str() {
    let id: RequestId = "req-1".into();
    match id {
        RequestId::String(s) => assert_eq!(s, "req-1"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_request_id_partial_eq() {
    assert_eq!(RequestId::Number(1), RequestId::Number(1));
    assert_ne!(RequestId::Number(1), RequestId::Number(2));
    assert_eq!(RequestId::String("a".into()), RequestId::String("a".into()));
    assert_ne!(RequestId::String("a".into()), RequestId::String("b".into()));
    assert_ne!(RequestId::Number(1), RequestId::String("1".into()));
}

#[test]
fn test_server_info_serialization() {
    let info = ServerInfo {
        name: "test-server".into(),
        version: "1.2.3".into(),
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains(r#""name":"test-server""#));
    assert!(json.contains(r#""version":"1.2.3""#));
}

#[test]
fn test_server_capabilities_default() {
    let caps = ServerCapabilities::default();
    assert!(caps.tools.is_none());
    assert!(caps.resources.is_none());
    assert!(caps.prompts.is_none());
    assert!(caps.logging.is_none());

    let json = serde_json::to_string(&caps).unwrap();
    assert_eq!(json, "{}");
}

#[test]
fn test_server_capabilities_with_tools() {
    let caps = ServerCapabilities {
        tools: Some(ToolsCapability { list_changed: true }),
        ..Default::default()
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains(r#""tools":{"listChanged":true}"#));
    assert!(!json.contains("resources"));
}

#[test]
fn test_server_capabilities_with_resources() {
    let caps = ServerCapabilities {
        resources: Some(ResourcesCapability {
            subscribe: true,
            list_changed: true,
        }),
        ..Default::default()
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains(r#""subscribe":true"#));
    assert!(json.contains(r#""listChanged":true"#));
}

#[test]
fn test_server_capabilities_with_prompts() {
    let caps = ServerCapabilities {
        prompts: Some(PromptsCapability { list_changed: true }),
        ..Default::default()
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains(r#""prompts":{"listChanged":true}"#));
}

#[test]
fn test_server_capabilities_with_logging() {
    let caps = ServerCapabilities {
        logging: Some(LoggingCapability {}),
        ..Default::default()
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains(r#""logging":{}"#));
}

#[test]
fn test_server_capabilities_all() {
    let caps = ServerCapabilities {
        tools: Some(ToolsCapability::default()),
        resources: Some(ResourcesCapability::default()),
        prompts: Some(PromptsCapability::default()),
        logging: Some(LoggingCapability {}),
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("tools"));
    assert!(json.contains("resources"));
    assert!(json.contains("prompts"));
    assert!(json.contains("logging"));
}

#[test]
fn test_tools_capability_default() {
    let cap = ToolsCapability::default();
    assert!(!cap.list_changed);
}

#[test]
fn test_tools_capability_list_changed_true() {
    let cap = ToolsCapability { list_changed: true };
    let json = serde_json::to_string(&cap).unwrap();
    assert!(json.contains(r#""listChanged":true"#));
}

#[test]
fn test_tools_capability_deserialization_with_list_changed() {
    let json = json!({"listChanged": true});
    let cap: ToolsCapability = serde_json::from_value(json).unwrap();
    assert!(cap.list_changed);
}

#[test]
fn test_tools_capability_deserialization_without_list_changed() {
    let json = json!({});
    let cap: ToolsCapability = serde_json::from_value(json).unwrap();
    assert!(!cap.list_changed);
}

#[test]
fn test_resources_capability_default() {
    let cap = ResourcesCapability::default();
    assert!(!cap.subscribe);
    assert!(!cap.list_changed);
}

#[test]
fn test_resources_capability_subscribe() {
    let cap = ResourcesCapability {
        subscribe: true,
        list_changed: false,
    };
    let json = serde_json::to_string(&cap).unwrap();
    assert!(json.contains(r#""subscribe":true"#));
    assert!(json.contains(r#""listChanged":false"#));
}

#[test]
fn test_prompts_capability_default() {
    let cap = PromptsCapability::default();
    assert!(!cap.list_changed);
}

#[test]
fn test_logging_capability_serialization() {
    let cap = LoggingCapability {};
    let json = serde_json::to_string(&cap).unwrap();
    assert_eq!(json, "{}");
}

#[test]
fn test_initialize_result_serialization() {
    let result = InitializeResult {
        protocol_version: "2024-11-05".into(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability::default()),
            ..Default::default()
        },
        server_info: ServerInfo {
            name: "srv".into(),
            version: "0.1".into(),
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""protocolVersion":"2024-11-05""#));
    assert!(json.contains(r#""tools":{"listChanged":false}"#));
    assert!(json.contains(r#""serverInfo":{"name":"srv""#));
}

#[test]
fn test_initialize_result_deserialization() {
    let json = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "srv", "version": "1.0"}
    });
    let result: InitializeResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.protocol_version, "2024-11-05");
    assert_eq!(result.server_info.name, "srv");
}

#[test]
fn test_tool_serialization() {
    let tool = Tool {
        name: "echo".into(),
        description: Some("Echo a message".into()),
        input_schema: json!({"type": "object"}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains(r#""name":"echo""#));
    assert!(json.contains(r#""description":"Echo a message""#));
    assert!(json.contains(r#""inputSchema":{"type":"object"}"#));
}

#[test]
fn test_tool_optional_description_omitted() {
    let tool = Tool {
        name: "bare".into(),
        description: None,
        input_schema: json!({}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    assert!(!json.contains("description"));
}

#[test]
fn test_tool_deserialization() {
    let json = json!({
        "name": "test",
        "description": "A test tool",
        "inputSchema": {"type": "object"}
    });
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.name, "test");
    assert_eq!(tool.description, Some("A test tool".into()));
}

#[test]
fn test_list_tools_result_without_cursor() {
    let result = ListToolsResult {
        tools: vec![Tool {
            name: "a".into(),
            description: None,
            input_schema: json!({}),
        }],
        next_cursor: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("nextCursor"));
}

#[test]
fn test_list_tools_result_with_cursor() {
    let result = ListToolsResult {
        tools: vec![],
        next_cursor: Some("page2".into()),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""nextCursor":"page2""#));
}

#[test]
fn test_call_tool_params_serialization() {
    let params = CallToolParams {
        name: "echo".into(),
        arguments: json!({"msg": "hi"}),
    };
    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains(r#""name":"echo""#));
    assert!(json.contains(r#""arguments":{"msg":"hi"}"#));
}

#[test]
fn test_call_tool_params_deserialization_with_arguments() {
    let json = json!({"name": "test", "arguments": {"x": 1}});
    let params: CallToolParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.name, "test");
    assert_eq!(params.arguments["x"], 1);
}

#[test]
fn test_call_tool_params_deserialization_default_arguments() {
    let json = json!({"name": "test"});
    let params: CallToolParams = serde_json::from_value(json).unwrap();
    assert!(params.arguments.is_null() || params.arguments == json!({}));
}

#[test]
fn test_call_tool_params_deserialization_null_arguments() {
    let json = json!({"name": "test", "arguments": null});
    let params: CallToolParams = serde_json::from_value(json).unwrap();
    assert!(params.arguments.is_null());
}

#[test]
fn test_call_tool_result_text() {
    let result = CallToolResult::text("hello");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "hello"),
        _ => panic!("Expected text"),
    }
}

#[test]
fn test_call_tool_result_error() {
    let result = CallToolResult::error("failed");
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "failed"),
        _ => panic!("Expected error text"),
    }
}

#[test]
fn test_call_tool_result_text_from_string() {
    let msg: String = "from string".into();
    let result = CallToolResult::text(msg);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "from string"),
        _ => panic!("Expected text"),
    }
}

#[test]
fn test_call_tool_result_multiple_content() {
    let result = CallToolResult {
        content: vec![Content::text("line 1"), Content::text("line 2")],
        is_error: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""isError":false"#));
}

#[test]
fn test_call_tool_result_is_error_serialization() {
    let result = CallToolResult {
        content: vec![],
        is_error: true,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""isError":true"#));
}

#[test]
fn test_call_tool_result_deserialization() {
    let json = json!({
        "content": [{"type": "text", "text": "hello"}],
        "isError": false
    });
    let result: CallToolResult = serde_json::from_value(json).unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[test]
fn test_call_tool_result_empty_content() {
    let result = CallToolResult {
        content: vec![],
        is_error: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""content":[]"#));
}

#[test]
fn test_content_text_serialization() {
    let content = Content::Text {
        text: "hello".into(),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains(r#""type":"text""#));
    assert!(json.contains(r#""text":"hello""#));
}

#[test]
fn test_content_text_deserialization() {
    let json = json!({"type": "text", "text": "world"});
    let content: Content = serde_json::from_value(json).unwrap();
    match content {
        Content::Text { text } => assert_eq!(text, "world"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_content_image_serialization() {
    let content = Content::Image {
        data: "aGVsbG8=".into(),
        mime_type: "image/png".into(),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains(r#""type":"image""#));
    assert!(json.contains(r#""data":"aGVsbG8=""#));
    assert!(json.contains(r#""mimeType":"image/png""#));
}

#[test]
fn test_content_image_deserialization() {
    let json = json!({"type": "image", "data": "AAAA", "mimeType": "image/jpeg"});
    let content: Content = serde_json::from_value(json).unwrap();
    match content {
        Content::Image { data, mime_type } => {
            assert_eq!(data, "AAAA");
            assert_eq!(mime_type, "image/jpeg");
        }
        _ => panic!("Expected Image"),
    }
}

#[test]
fn test_content_resource_link_serialization() {
    let content = Content::ResourceLink {
        resource: ResourceLink {
            uri: "file:///test.txt".into(),
            mime_type: Some("text/plain".into()),
        },
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains(r#""type":"resource""#));
    assert!(json.contains(r#""uri":"file:///test.txt""#));
}

#[test]
fn test_content_resource_link_deserialization() {
    let json = json!({
        "type": "resource",
        "resource": {"uri": "file:///x", "mimeType": "text/plain"}
    });
    let content: Content = serde_json::from_value(json).unwrap();
    match content {
        Content::ResourceLink { resource } => {
            assert_eq!(resource.uri, "file:///x");
        }
        _ => panic!("Expected ResourceLink"),
    }
}

#[test]
fn test_content_text_constructor_from_str() {
    let content = Content::text("from &str");
    match content {
        Content::Text { text } => assert_eq!(text, "from &str"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_content_text_constructor_from_string() {
    let s: String = "from String".into();
    let content = Content::text(s);
    match content {
        Content::Text { text } => assert_eq!(text, "from String"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_resource_link_with_mime_type() {
    let link = ResourceLink {
        uri: "file:///doc.md".into(),
        mime_type: Some("text/markdown".into()),
    };
    let json = serde_json::to_string(&link).unwrap();
    assert!(json.contains(r#""mimeType":"text/markdown""#));
}

#[test]
fn test_resource_link_without_mime_type() {
    let link = ResourceLink {
        uri: "file:///bare".into(),
        mime_type: None,
    };
    let json = serde_json::to_string(&link).unwrap();
    assert!(!json.contains("mimeType"));
}

#[test]
fn test_resource_link_deserialization() {
    let json = json!({"uri": "file:///x.txt", "mimeType": "text/plain"});
    let link: ResourceLink = serde_json::from_value(json).unwrap();
    assert_eq!(link.uri, "file:///x.txt");
    assert_eq!(link.mime_type, Some("text/plain".into()));
}

#[test]
fn test_list_params_with_cursor() {
    let params = ListParams {
        cursor: Some("abc".into()),
    };
    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains(r#""cursor":"abc""#));
}

#[test]
fn test_list_params_without_cursor() {
    let params = ListParams { cursor: None };
    let json = serde_json::to_string(&params).unwrap();
    assert!(!json.contains("cursor"));
}

#[test]
fn test_list_params_deserialization() {
    let json = json!({"cursor": "page2"});
    let params: ListParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.cursor, Some("page2".into()));
}

#[test]
fn test_protocol_version_v20241105() {
    assert_eq!(ProtocolVersion::V20241105.as_str(), "2024-11-05");
}

#[test]
fn test_protocol_version_latest() {
    assert_eq!(ProtocolVersion::Latest.as_str(), "2024-11-05");
}

#[test]
fn test_protocol_version_default() {
    let version = ProtocolVersion::default();
    match version {
        ProtocolVersion::Latest => {}
        _ => panic!("Expected Latest as default"),
    }
}

#[test]
fn test_protocol_version_copy() {
    let v1 = ProtocolVersion::V20241105;
    let v2 = v1;
    assert_eq!(v1.as_str(), v2.as_str());
}
