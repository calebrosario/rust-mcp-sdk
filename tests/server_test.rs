use mcp_sdk::error::McpError;
use mcp_sdk::protocol::*;
use mcp_sdk::McpServer;
use serde_json::json;

#[tokio::test]
async fn test_server_initialize() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(1),
        method: "initialize".into(),
        params: Some(json!({"protocolVersion": "2024-11-05"})),
    };

    let response = server.handle_request(&request).await.unwrap();
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    let init: InitializeResult = serde_json::from_value(result).unwrap();
    assert_eq!(init.server_info.name, "test");
    assert_eq!(init.server_info.version, "1.0.0");
    assert_eq!(init.protocol_version, "2024-11-05");
    assert!(init.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_server_ping() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(2),
        method: "ping".into(),
        params: None,
    };

    let response = server.handle_request(&request).await.unwrap();
    assert!(response.result.is_some());
    assert_eq!(response.result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_server_unknown_method() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(3),
        method: "foo/bar".into(),
        params: None,
    };

    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MethodNotFound(method) => assert_eq!(method, "foo/bar"),
        _ => panic!("Expected MethodNotFound"),
    }
}

#[tokio::test]
async fn test_server_tools_list_empty() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(4),
        method: "tools/list".into(),
        params: None,
    };

    let response = server.handle_request(&request).await.unwrap();
    let result: ListToolsResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.tools.is_empty());
}

#[tokio::test]
async fn test_server_tools_list_with_tools() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("echo")
                .description("echo tool")
                .schema(json!({"type": "object"}))
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("ping")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("pong")) }),
        )
        .await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(5),
        method: "tools/list".into(),
        params: None,
    };

    let response = server.handle_request(&request).await.unwrap();
    let result: ListToolsResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.tools.len(), 2);
}

#[tokio::test]
async fn test_server_tools_call_success() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("greet")
                .schema(json!({"type": "object", "properties": {"name": {"type": "string"}}}))
                .handler(|args| async move {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
                    Ok(mcp_sdk::CallToolResult::text(format!("Hello, {name}!")))
                }),
        )
        .await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(6),
        method: "tools/call".into(),
        params: Some(json!({"name": "greet", "arguments": {"name": "Rust"}})),
    };

    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "Hello, Rust!"),
        _ => panic!("Expected text"),
    }
}

#[tokio::test]
async fn test_server_tools_call_unknown_tool() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(7),
        method: "tools/call".into(),
        params: Some(json!({"name": "nonexistent", "arguments": {}})),
    };

    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound(name) => assert_eq!(name, "nonexistent"),
        _ => panic!("Expected ToolNotFound"),
    }
}

#[tokio::test]
async fn test_server_tools_call_missing_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(8),
        method: "tools/call".into(),
        params: None,
    };

    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidParams(_) => {}
        _ => panic!("Expected InvalidParams"),
    }
}

#[tokio::test]
async fn test_server_tools_call_error_response() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("fail")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::error("boom")) }),
        )
        .await;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(9),
        method: "tools/call".into(),
        params: Some(json!({"name": "fail", "arguments": {}})),
    };

    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.is_error);
}

#[tokio::test]
async fn test_server_handles_multiple_requests() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("add")
                .handler(|args| async move {
                    let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    Ok(mcp_sdk::CallToolResult::text(format!("{}", a + b)))
                }),
        )
        .await;

    for i in 0..5 {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: RequestId::Number(i),
            method: "tools/call".into(),
            params: Some(json!({"name": "add", "arguments": {"a": i, "b": i}})),
        };

        let response = server.handle_request(&request).await.unwrap();
        let result: CallToolResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        match &result.content[0] {
            Content::Text { text } => assert_eq!(text, &format!("{}", i * 2)),
            _ => panic!("Expected text"),
        }
    }
}

#[tokio::test]
async fn test_server_response_id_preserved() {
    let server = McpServer::new("test", "1.0.0");

    let string_id = RequestId::String("req-xyz".into());
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: string_id.clone(),
        method: "ping".into(),
        params: None,
    };

    let response = server.handle_request(&request).await.unwrap();
    assert_eq!(response.id, string_id);
}

#[tokio::test]
async fn test_server_notification_doesnt_panic() {
    let server = McpServer::new("test", "1.0.0");

    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "notifications/initialized".into(),
        params: None,
    };

    server.handle_notification(&notif);
}
