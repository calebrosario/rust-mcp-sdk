use mcp_sdk::error::McpError;
use mcp_sdk::protocol::*;
use mcp_sdk::McpServer;
use serde_json::json;

fn make_request(id: i64, method: &str, params: Option<serde_json::Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(id),
        method: method.into(),
        params,
    }
}

async fn init_server(server: &McpServer) {
    let _ = server
        .handle_request(&make_request(0, "initialize", None))
        .await;
    server
        .handle_notification(&JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "notifications/initialized".into(),
            params: None,
        })
        .await;
}

#[tokio::test]
async fn test_server_initialize() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(
        1,
        "initialize",
        Some(json!({"protocolVersion": "2024-11-05"})),
    );

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
async fn test_server_initialize_with_none_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "initialize", None);

    let response = server.handle_request(&request).await.unwrap();
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_server_initialize_with_string_name() {
    let name: String = "string-server".into();
    let version: String = "2.0.0".into();
    let server = McpServer::new(name, version);

    let request = make_request(1, "initialize", None);
    let response = server.handle_request(&request).await.unwrap();
    let init: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(init.server_info.name, "string-server");
    assert_eq!(init.server_info.version, "2.0.0");
}

#[tokio::test]
async fn test_server_initialize_advertises_all_capabilities() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "initialize", None);
    let response = server.handle_request(&request).await.unwrap();
    let init: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(init.capabilities.tools.is_some());
    assert!(init.capabilities.resources.is_some());
    assert!(init.capabilities.prompts.is_some());
}

#[tokio::test]
async fn test_server_custom_protocol_version() {
    let server = McpServer::new("test", "1.0.0").protocol_version(ProtocolVersion::V20241105);

    let request = make_request(1, "initialize", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let init: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(init.protocol_version, "2024-11-05");
}

#[tokio::test]
async fn test_server_server_info_getter() {
    let server = McpServer::new("my-server", "3.1.4");
    let info = server.server_info();
    assert_eq!(info.name, "my-server");
    assert_eq!(info.version, "3.1.4");
}

#[tokio::test]
async fn test_server_ping() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "ping", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_server_unknown_method() {
    let server = McpServer::new("test", "1.0.0");
    init_server(&server).await;

    let request = make_request(1, "unknown/method", None);
    let result = server.handle_request(&request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MethodNotFound(method) => assert_eq!(method, "unknown/method"),
        _ => panic!("Expected MethodNotFound"),
    }
}

#[tokio::test]
async fn test_server_tools_list_empty() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.tools.is_empty());
    assert!(result.next_cursor.is_none());
}

#[tokio::test]
async fn test_server_tools_list_with_tools() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("echo")
                .description("Echo")
                .handler(|args| async move {
                    let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(mcp_sdk::CallToolResult::text(msg))
                }),
        )
        .await;
    server
        .register_tool(mcp_sdk::ToolBuilder::new("add").handler(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(mcp_sdk::CallToolResult::text(format!("{}", a + b)))
        }))
        .await;

    let request = make_request(1, "tools/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.tools.len(), 2);
}

#[tokio::test]
async fn test_server_tools_call_success() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("greet").handler(|args| async move {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
                Ok(mcp_sdk::CallToolResult::text(format!("Hello, {name}!")))
            }),
        )
        .await;

    let request = make_request(
        1,
        "tools/call",
        Some(json!({"name": "greet", "arguments": {"name": "Rust"}})),
    );
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "Hello, Rust!"),
        _ => panic!("Expected text"),
    }
}

#[tokio::test]
async fn test_server_tools_call_unknown_tool() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!({"name": "nonexistent"})));
    init_server(&server).await;
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

    let request = make_request(1, "tools/call", None);
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidParams(msg) => assert!(msg.contains("missing params")),
        _ => panic!("Expected InvalidParams"),
    }
}

#[tokio::test]
async fn test_server_tools_call_invalid_params_not_object() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!("not an object")));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_server_tools_call_with_no_arguments_field() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("ping")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("pong")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "ping"})));
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "pong"),
        _ => panic!("Expected pong"),
    }
}

#[tokio::test]
async fn test_server_tools_call_error_response() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("fail")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::error("tool failed")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "fail"})));
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "tool failed"),
        _ => panic!("Expected error text"),
    }
}

#[tokio::test]
async fn test_server_handles_multiple_requests() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(mcp_sdk::ToolBuilder::new("add").handler(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(mcp_sdk::CallToolResult::text(format!("{}", a + b)))
        }))
        .await;

    for i in 0..5 {
        let request = make_request(
            i,
            "tools/call",
            Some(json!({"name": "add", "arguments": {"a": i, "b": i}})),
        );
        init_server(&server).await;
        let response = server.handle_request(&request).await.unwrap();
        let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
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

    init_server(&server).await;
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

    server.handle_notification(&notif).await;

    assert!(server.is_initialized().await);
}

#[tokio::test]
async fn test_server_is_initialized_false_before_notification() {
    let server = McpServer::new("test", "1.0.0");
    assert!(!server.is_initialized().await);
}

#[tokio::test]
async fn test_server_is_initialized_true_after_notification() {
    let server = McpServer::new("test", "1.0.0");

    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "notifications/initialized".into(),
        params: None,
    };

    server.handle_notification(&notif).await;
    assert!(server.is_initialized().await);
}

#[tokio::test]
async fn test_server_notification_non_initialized_method() {
    let server = McpServer::new("test", "1.0.0");

    let notif = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "some/other/notification".into(),
        params: None,
    };

    server.handle_notification(&notif).await;
    assert!(!server.is_initialized().await);
}

#[tokio::test]
async fn test_server_resources_list_empty() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "resources/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.resources.is_empty());
    assert!(result.next_cursor.is_none());
}

#[tokio::test]
async fn test_server_resources_list_with_resources() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///readme.md", "Readme")
                .description("Project readme")
                .handler(|_| async move {
                    Ok(vec![mcp_sdk::ResourceContents::Text {
                        uri: "file:///readme.md".into(),
                        mime_type: None,
                        text: "# Hello".into(),
                    }])
                }),
        )
        .await;

    let request = make_request(1, "resources/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.resources.len(), 1);
    assert_eq!(result.resources[0].uri, "file:///readme.md");
}

#[tokio::test]
async fn test_server_resources_list_multiple() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///a", "A").handler(|_| async move { Ok(vec![]) }),
        )
        .await;
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///b", "B").handler(|_| async move { Ok(vec![]) }),
        )
        .await;

    let request = make_request(1, "resources/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.resources.len(), 2);
}

#[tokio::test]
async fn test_server_resources_read_success() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///config.json", "Config").handler(
                |_| async move {
                    Ok(vec![mcp_sdk::ResourceContents::Text {
                        uri: "file:///config.json".into(),
                        mime_type: Some("application/json".into()),
                        text: r#"{"version": "1.0"}"#.into(),
                    }])
                },
            ),
        )
        .await;

    let request = make_request(
        1,
        "resources/read",
        Some(json!({"uri": "file:///config.json"})),
    );
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ReadResourceResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        mcp_sdk::ResourceContents::Text { text, .. } => {
            assert_eq!(text, r#"{"version": "1.0"}"#)
        }
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_server_resources_read_unknown_uri() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(
        1,
        "resources/read",
        Some(json!({"uri": "file:///nonexistent"})),
    );
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound(uri) => assert_eq!(uri, "file:///nonexistent"),
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[tokio::test]
async fn test_server_resources_read_missing_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "resources/read", None);
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidParams(msg) => assert!(msg.contains("missing params")),
        _ => panic!("Expected InvalidParams"),
    }
}

#[tokio::test]
async fn test_server_resources_read_invalid_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "resources/read", Some(json!("not an object")));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_server_prompts_list_empty() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "prompts/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListPromptsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.prompts.is_empty());
}

#[tokio::test]
async fn test_server_prompts_list_with_prompts() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("explain")
                .description("Explain code")
                .argument("language")
                .handler(|_| async move {
                    Ok(mcp_sdk::GetPromptResult {
                        description: None,
                        messages: vec![],
                    })
                }),
        )
        .await;

    let request = make_request(1, "prompts/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListPromptsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.prompts.len(), 1);
    assert_eq!(result.prompts[0].name, "explain");
    assert_eq!(result.prompts[0].arguments.len(), 1);
}

#[tokio::test]
async fn test_server_prompts_list_multiple() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(mcp_sdk::PromptBuilder::new("a").handler(|_| async move {
            Ok(mcp_sdk::GetPromptResult {
                description: None,
                messages: vec![],
            })
        }))
        .await;
    server
        .register_prompt(mcp_sdk::PromptBuilder::new("b").handler(|_| async move {
            Ok(mcp_sdk::GetPromptResult {
                description: None,
                messages: vec![],
            })
        }))
        .await;

    let request = make_request(1, "prompts/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListPromptsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.prompts.len(), 2);
}

#[tokio::test]
async fn test_server_prompts_get_success() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("greet")
                .description("Greet someone")
                .handler(|args| async move {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
                    Ok(mcp_sdk::GetPromptResult {
                        description: Some("Greeting prompt".into()),
                        messages: vec![mcp_sdk::PromptMessage {
                            role: "assistant".into(),
                            content: mcp_sdk::Content::text(format!("Hello, {name}!")),
                        }],
                    })
                }),
        )
        .await;

    let request = make_request(
        1,
        "prompts/get",
        Some(json!({"name": "greet", "arguments": {"name": "Rust"}})),
    );
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: GetPromptResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.description, Some("Greeting prompt".into()));
    assert_eq!(result.messages.len(), 1);
    match &result.messages[0].content {
        mcp_sdk::Content::Text { text } => assert_eq!(text, "Hello, Rust!"),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_server_prompts_get_without_arguments() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("simple").handler(|_| async move {
                Ok(mcp_sdk::GetPromptResult {
                    description: None,
                    messages: vec![mcp_sdk::PromptMessage {
                        role: "user".into(),
                        content: mcp_sdk::Content::text("Hello"),
                    }],
                })
            }),
        )
        .await;

    let request = make_request(1, "prompts/get", Some(json!({"name": "simple"})));
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: GetPromptResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.messages.len(), 1);
}

#[tokio::test]
async fn test_server_prompts_get_with_null_arguments() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("null_test").handler(|args| async move {
                assert!(args.is_null());
                Ok(mcp_sdk::GetPromptResult {
                    description: None,
                    messages: vec![],
                })
            }),
        )
        .await;

    let request = make_request(
        1,
        "prompts/get",
        Some(json!({"name": "null_test", "arguments": null})),
    );
    init_server(&server).await;
    server.handle_request(&request).await.unwrap();
}

#[tokio::test]
async fn test_server_prompts_get_unknown_name() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "prompts/get", Some(json!({"name": "nonexistent"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound(name) => assert_eq!(name, "nonexistent"),
        _ => panic!("Expected PromptNotFound"),
    }
}

#[tokio::test]
async fn test_server_prompts_get_missing_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "prompts/get", None);
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidParams(msg) => assert!(msg.contains("missing params")),
        _ => panic!("Expected InvalidParams"),
    }
}

#[tokio::test]
async fn test_server_prompts_get_invalid_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "prompts/get", Some(json!(42)));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_server_registry_getter() {
    let server = McpServer::new("test", "1.0.0");
    let registry = server.registry();
    let tools = registry.lock().await.list();
    assert!(tools.is_empty());
}

#[tokio::test]
async fn test_server_concurrent_requests() {
    let server = std::sync::Arc::new(McpServer::new("test", "1.0.0"));
    server
        .register_tool(mcp_sdk::ToolBuilder::new("add").handler(|args| async move {
            let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(mcp_sdk::CallToolResult::text(format!("{}", a + b)))
        }))
        .await;

    let mut handles = vec![];
    for i in 0..10 {
        let server = server.clone();
        handles.push(tokio::spawn(async move {
            let request = make_request(
                i,
                "tools/call",
                Some(json!({"name": "add", "arguments": {"a": i, "b": 1}})),
            );
            init_server(&server).await;
            let response = server.handle_request(&request).await.unwrap();
            let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
            match &result.content[0] {
                Content::Text { text } => assert_eq!(text, &format!("{}", i + 1)),
                _ => panic!("Expected text"),
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
