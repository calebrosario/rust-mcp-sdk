use mcp_sdk::error::McpError;
use mcp_sdk::protocol::*;
use mcp_sdk::{McpServer, ToolBuilder};
use serde_json::json;
use std::sync::Arc;

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
async fn test_security_large_but_valid_payload() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(ToolBuilder::new("echo").handler(|args| async move {
            let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("");
            Ok(mcp_sdk::CallToolResult::text(msg))
        }))
        .await;

    let large_string = "x".repeat(100_000);
    let request = make_request(
        1,
        "tools/call",
        Some(json!({"name": "echo", "arguments": {"msg": large_string}})),
    );

    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_security_deeply_nested_json() {
    let server = McpServer::new("test", "1.0.0");

    let mut nested = json!({"name": "a"});
    for _ in 0..100 {
        nested = json!({"nested": nested});
    }

    let request = make_request(1, "tools/call", Some(nested));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_extremely_long_method_name() {
    let server = McpServer::new("test", "1.0.0");

    let long_method = "x".repeat(10_000);
    let request = make_request(1, &long_method, None);

    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MethodNotFound(m) => assert_eq!(m.len(), 10_000),
        _ => panic!("Expected MethodNotFound"),
    }
}

#[tokio::test]
async fn test_security_handler_returning_error_doesnt_crash() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("fail")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::error("handler error")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "fail"})));
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.is_error);
}

#[tokio::test]
async fn test_security_handler_returning_mcp_error_propagates() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("internal_err")
                .handler(|_| async { Err(McpError::Internal("something broke".into())) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "internal_err"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => assert_eq!(msg, "something broke"),
        _ => panic!("Expected Internal error"),
    }
}

#[tokio::test]
async fn test_security_resource_handler_error_propagates() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///err", "Err")
                .handler(|_| async { Err(McpError::Internal("read failed".into())) }),
        )
        .await;

    let request = make_request(1, "resources/read", Some(json!({"uri": "file:///err"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_prompt_handler_error_propagates() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("fail")
                .handler(|_| async { Err(McpError::Internal("prompt failed".into())) }),
        )
        .await;

    let request = make_request(1, "prompts/get", Some(json!({"name": "fail"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_null_bytes_in_method_name() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call\x00inject", None);
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MethodNotFound(m) => assert!(m.contains('\0')),
        _ => panic!("Expected MethodNotFound"),
    }
}

#[tokio::test]
async fn test_security_control_chars_in_tool_name() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("legit")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "legit\x00evil"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound(name) => assert!(name.contains('\0')),
        _ => panic!("Expected ToolNotFound"),
    }
}

#[tokio::test]
async fn test_security_sql_injection_in_tool_name() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("query")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("results")) }),
        )
        .await;

    let request = make_request(
        1,
        "tools/call",
        Some(json!({"name": "query'; DROP TABLE tools; --"})),
    );
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound(_) => {}
        _ => panic!("Expected ToolNotFound for SQL injection attempt"),
    }
}

#[tokio::test]
async fn test_security_path_traversal_in_resource_uri() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(
        1,
        "resources/read",
        Some(json!({"uri": "../../../etc/passwd"})),
    );
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound(uri) => assert_eq!(uri, "../../../etc/passwd"),
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[tokio::test]
async fn test_security_unicode_in_resource_uri() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(
        1,
        "resources/read",
        Some(json!({"uri": "file:///测试/路径"})),
    );
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound(uri) => assert!(uri.contains("测试")),
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[tokio::test]
async fn test_security_tool_name_with_markup_injection() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("safe").handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    let request = make_request(
        1,
        "tools/call",
        Some(json!({"name": "<script>alert('xss')</script>"})),
    );
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_missing_required_fields_in_request() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!({"arguments": {}})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_empty_string_tool_name() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("real").handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": ""})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound(name) => assert!(name.is_empty()),
        _ => panic!("Expected ToolNotFound"),
    }
}

#[tokio::test]
async fn test_security_empty_string_resource_uri() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "resources/read", Some(json!({"uri": ""})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound(uri) => assert!(uri.is_empty()),
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[tokio::test]
async fn test_security_empty_string_prompt_name() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "prompts/get", Some(json!({"name": ""})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound(name) => assert!(name.is_empty()),
        _ => panic!("Expected PromptNotFound"),
    }
}

#[tokio::test]
async fn test_security_null_arguments_value() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(ToolBuilder::new("accept_null").handler(|args| async move {
            assert!(args.is_null());
            Ok(mcp_sdk::CallToolResult::text("accepted null"))
        }))
        .await;

    let request = make_request(
        1,
        "tools/call",
        Some(json!({"name": "accept_null", "arguments": null})),
    );
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!result.is_error);
}

#[tokio::test]
async fn test_security_array_instead_of_object_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!([1, 2, 3])));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_number_instead_of_object_params() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!(42)));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_boolean_instead_of_string_name() {
    let server = McpServer::new("test", "1.0.0");

    let request = make_request(1, "tools/call", Some(json!({"name": true})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_max_request_id() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(i64::MAX),
        method: "ping".into(),
        params: None,
    };

    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    match response.id {
        RequestId::Number(n) => assert_eq!(n, i64::MAX),
        _ => panic!("Expected Number id"),
    }
}

#[tokio::test]
async fn test_security_min_request_id() {
    let server = McpServer::new("test", "1.0.0");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::Number(i64::MIN),
        method: "ping".into(),
        params: None,
    };

    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    match response.id {
        RequestId::Number(n) => assert_eq!(n, i64::MIN),
        _ => panic!("Expected Number id"),
    }
}

#[tokio::test]
async fn test_security_very_long_string_request_id() {
    let server = McpServer::new("test", "1.0.0");

    let long_id = "x".repeat(10_000);
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: RequestId::String(long_id.clone()),
        method: "ping".into(),
        params: None,
    };

    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    match response.id {
        RequestId::String(s) => assert_eq!(s.len(), 10_000),
        _ => panic!("Expected String id"),
    }
}

#[tokio::test]
async fn test_security_concurrent_tool_calls_dont_corrupt_state() {
    let server = Arc::new(McpServer::new("test", "1.0.0"));
    server
        .register_tool(
            mcp_sdk::ToolBuilder::new("counter").handler(|args| async move {
                let n = args.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(mcp_sdk::CallToolResult::text(format!("{}", n * 2)))
            }),
        )
        .await;

    let mut handles = vec![];
    for i in 0..50 {
        let server = server.clone();
        handles.push(tokio::spawn(async move {
            let request = make_request(
                i,
                "tools/call",
                Some(json!({"name": "counter", "arguments": {"n": i}})),
            );
            init_server(&server).await;
            let response = server.handle_request(&request).await.unwrap();
            let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
            match &result.content[0] {
                Content::Text { text } => assert_eq!(text, &format!("{}", i * 2)),
                _ => panic!("Expected text"),
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_security_concurrent_register_and_call() {
    let server = Arc::new(McpServer::new("test", "1.0.0"));

    let mut handles = vec![];

    for i in 0..10 {
        let server = server.clone();
        handles.push(tokio::spawn(async move {
            server
                .register_tool(mcp_sdk::ToolBuilder::new(format!("tool_{i}")).handler(
                    move |_| async move { Ok(mcp_sdk::CallToolResult::text(format!("tool_{i}"))) },
                ))
                .await;
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let request = make_request(1, "tools/list", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.tools.len(), 10);
}

#[tokio::test]
async fn test_server_remains_usable_after_error() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("ok").handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    let bad_request = make_request(1, "tools/call", None);
    init_server(&server).await;
    assert!(server.handle_request(&bad_request).await.is_err());

    let good_request = make_request(2, "tools/call", Some(json!({"name": "ok"})));
    let response = server.handle_request(&good_request).await.unwrap();
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_security_error_does_not_expose_tool_list() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("secret_admin_tool")
                .handler(|_| async { Ok(mcp_sdk::CallToolResult::text("secret")) }),
        )
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "wrong_name"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(!error_msg.contains("secret_admin_tool"));
}

#[tokio::test]
async fn test_security_error_does_not_expose_resource_list() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("secret://internal/data", "Secret")
                .handler(|_| async move { Ok(vec![]) }),
        )
        .await;

    let request = make_request(1, "resources/read", Some(json!({"uri": "wrong://uri"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(!error_msg.contains("secret://internal"));
}

#[cfg(feature = "http")]
mod http_security {
    use mcp_sdk::McpServer;
    use serde_json::json;
    use std::sync::Arc;

    fn get_free_port() -> u16 {
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        socket.local_addr().unwrap().port()
    }

    async fn start_test_server() -> String {
        let server = Arc::new(McpServer::new("test-http", "1.0.0"));
        let port = get_free_port();
        let addr = format!("127.0.0.1:{port}");

        let server_clone = server.clone();
        tokio::spawn(async move {
            let _ = mcp_sdk::HttpTransport::serve(server_clone, &format!("127.0.0.1:{port}")).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");
        let _ = client
            .post(&url)
            .json(&serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize"}))
            .send()
            .await;
        let _ = client
            .post(&url)
            .json(&serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .send()
            .await;

        addr
    }

    #[tokio::test]
    async fn test_http_no_auth_required() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        });

        let response = client.post(&url).json(&body).send().await.unwrap();
        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn test_http_handles_empty_body() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let response = client.post(&url).body("").send().await.unwrap();
        assert!(!response.status().is_success());
    }

    #[tokio::test]
    async fn test_http_handles_non_json_body() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body("not json at all")
            .send()
            .await
            .unwrap();

        assert!(!response.status().is_success());
    }

    #[tokio::test]
    async fn test_http_unknown_method_returns_error_not_crash() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "admin/shutdown"
        });

        let response = client.post(&url).json(&body).send().await.unwrap();
        assert!(response.status().is_success());

        let result: serde_json::Value = response.json().await.unwrap();
        assert!(result["error"].is_object());
    }

    #[tokio::test]
    async fn test_http_concurrent_requests() {
        let addr = start_test_server().await;
        let url = format!("http://{addr}/mcp");

        let client = reqwest::Client::new();
        let mut handles = vec![];

        for i in 0..20 {
            let client = client.clone();
            let url = url.clone();
            handles.push(tokio::spawn(async move {
                let body = json!({
                    "jsonrpc": "2.0",
                    "id": i,
                    "method": "ping"
                });
                let response = client.post(&url).json(&body).send().await.unwrap();
                assert!(response.status().is_success());
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_http_wrong_content_type() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let response = client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .send()
            .await
            .unwrap();

        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_http_get_method_rejected() {
        let addr = start_test_server().await;

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/mcp");

        let response = client.get(&url).send().await.unwrap();
        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_http_server_survives_bad_requests() {
        let addr = start_test_server().await;
        let url = format!("http://{addr}/mcp");

        let client = reqwest::Client::new();

        for _ in 0..5 {
            let _ = client
                .post(&url)
                .header("Content-Type", "application/json")
                .body("garbage")
                .send()
                .await;
        }

        let body = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
        let response = client.post(&url).json(&body).send().await.unwrap();
        assert!(response.status().is_success());
    }
}

#[tokio::test]
async fn test_security_resource_handler_receives_unmodified_uri() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("safe://resource", "Safe").handler(|uri| {
                let uri = uri.to_string();
                async move {
                    Ok(vec![mcp_sdk::ResourceContents::Text {
                        uri: uri.clone(),
                        mime_type: None,
                        text: uri,
                    }])
                }
            }),
        )
        .await;

    let request = make_request(1, "resources/read", Some(json!({"uri": "safe://resource"})));
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: ReadResourceResult = serde_json::from_value(response.result.unwrap()).unwrap();
    match &result.contents[0] {
        mcp_sdk::ResourceContents::Text { text, .. } => {
            assert_eq!(text, "safe://resource");
        }
        _ => panic!("Expected text"),
    }
}

#[tokio::test]
async fn test_security_prompt_handler_receives_unmodified_args() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("echo_args").handler(|args| async move {
                Ok(mcp_sdk::GetPromptResult {
                    description: Some(args.to_string()),
                    messages: vec![],
                })
            }),
        )
        .await;

    let request = make_request(
        1,
        "prompts/get",
        Some(json!({"name": "echo_args", "arguments": {"user_input": "test'; DROP TABLE--"}})),
    );
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let result: GetPromptResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.description.unwrap().contains("DROP TABLE"));
}

#[tokio::test]
async fn test_security_server_name_with_special_chars() {
    let server = McpServer::new("test\x00name", "1.0.0");

    let request = make_request(1, "initialize", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let init: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(init.server_info.name.contains('\0'));
}

#[tokio::test]
async fn test_security_server_name_with_markup() {
    let name = "<script>alert('xss')</script>";
    let server = McpServer::new(name, "1.0.0");

    let request = make_request(1, "initialize", None);
    init_server(&server).await;
    let response = server.handle_request(&request).await.unwrap();
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains(name));
}

#[tokio::test]
async fn test_security_tool_handler_panic_is_caught() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(ToolBuilder::new("panicker").handler(|_| async move {
            panic!("intentional panic in tool handler");
        }))
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "panicker"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => {
            assert!(msg.contains("Handler execution failed"), "got: {}", msg)
        }
        _ => panic!("Expected Internal error from panicked handler"),
    }
}

#[tokio::test]
async fn test_security_server_usable_after_tool_panic() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(ToolBuilder::new("panicker").handler(|_| async move { panic!("boom") }))
        .await;
    server
        .register_tool(
            ToolBuilder::new("safe").handler(|_| async { Ok(mcp_sdk::CallToolResult::text("ok")) }),
        )
        .await;

    let panic_request = make_request(1, "tools/call", Some(json!({"name": "panicker"})));
    init_server(&server).await;
    assert!(server.handle_request(&panic_request).await.is_err());

    let safe_request = make_request(2, "tools/call", Some(json!({"name": "safe"})));
    let response = server.handle_request(&safe_request).await.unwrap();
    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "ok"),
        _ => panic!("Expected ok"),
    }
}

#[tokio::test]
async fn test_security_resource_handler_panic_is_caught() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_resource(
            mcp_sdk::ResourceBuilder::new("file:///panic", "Panic")
                .handler(|_| async move { panic!("resource panic") }),
        )
        .await;

    let request = make_request(1, "resources/read", Some(json!({"uri": "file:///panic"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => {
            assert!(msg.contains("Handler execution failed"), "got: {}", msg)
        }
        _ => panic!("Expected Internal error from panicked handler"),
    }
}

#[tokio::test]
async fn test_security_prompt_handler_panic_is_caught() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_prompt(
            mcp_sdk::PromptBuilder::new("panicker")
                .handler(|_| async move { panic!("prompt panic") }),
        )
        .await;

    let request = make_request(1, "prompts/get", Some(json!({"name": "panicker"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => {
            assert!(msg.contains("Handler execution failed"), "got: {}", msg)
        }
        _ => panic!("Expected Internal error from panicked handler"),
    }
}

#[tokio::test]
async fn test_security_tool_panic_error_code_is_internal() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(ToolBuilder::new("panic").handler(|_| async move { panic!("crash") }))
        .await;

    let request = make_request(1, "tools/call", Some(json!({"name": "panic"})));
    init_server(&server).await;
    let result = server.handle_request(&request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), -32603);
}

#[tokio::test]
async fn test_security_multiple_panics_dont_crash() {
    let server = McpServer::new("test", "1.0.0");
    server
        .register_tool(
            ToolBuilder::new("panicker").handler(|_| async move { panic!("crash {}", 1) }),
        )
        .await;

    for i in 0..5 {
        let request = make_request(i, "tools/call", Some(json!({"name": "panicker"})));
        init_server(&server).await;
        assert!(server.handle_request(&request).await.is_err());
    }

    let ping = make_request(99, "ping", None);
    let response = server.handle_request(&ping).await.unwrap();
    assert!(response.result.is_some());
}
