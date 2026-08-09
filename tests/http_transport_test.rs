use mcp_sdk::{McpServer, ToolBuilder};
use serde_json::json;
use std::sync::Arc;

async fn start_test_server() -> String {
    let server = Arc::new(McpServer::new("test-http", "1.0.0"));
    server
        .register_tool(
            ToolBuilder::new("echo")
                .description("Echo message")
                .handler(|args| async move {
                    let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(mcp_sdk::CallToolResult::text(msg))
                }),
        )
        .await;

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
        .json(&json!({"jsonrpc": "2.0", "id": 0, "method": "initialize"}))
        .send()
        .await;
    let _ = client
        .post(&url)
        .json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .send()
        .await;

    addr
}

fn get_free_port() -> u16 {
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_http_initialize() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "2024-11-05"}
    });

    let response = send_http_request(&addr, &body).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["protocolVersion"].is_string());
    assert!(response["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn test_http_ping() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });

    let response = send_http_request(&addr, &body).await;

    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
}

#[tokio::test]
async fn test_http_tools_list() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    });

    let response = send_http_request(&addr, &body).await;

    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo");
}

#[tokio::test]
async fn test_http_tools_call() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {"msg": "hello over http"}
        }
    });

    let response = send_http_request(&addr, &body).await;

    let content = response["result"]["content"].as_array().unwrap();
    assert_eq!(content[0]["text"], "hello over http");
    assert_eq!(response["result"]["isError"], false);
}

#[tokio::test]
async fn test_http_unknown_method() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "unknown/method"
    });

    let response = send_http_request(&addr, &body).await;

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601);
}

#[tokio::test]
async fn test_http_notification() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let client = reqwest::Client::new();
    let url = format!("http://{addr}/mcp");

    let response = client.post(&url).json(&body).send().await.unwrap();

    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.unwrap();
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn test_http_tool_not_found() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {"name": "nonexistent"}
    });

    let response = send_http_request(&addr, &body).await;

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601);
}

#[tokio::test]
async fn test_http_string_request_id() {
    let addr = start_test_server().await;

    let body = json!({
        "jsonrpc": "2.0",
        "id": "custom-string-id",
        "method": "ping"
    });

    let response = send_http_request(&addr, &body).await;

    assert_eq!(response["id"], "custom-string-id");
}

async fn send_http_request(addr: &str, body: &serde_json::Value) -> serde_json::Value {
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/mcp");
    let response = client.post(&url).json(body).send().await.unwrap();
    assert!(response.status().is_success());
    response.json().await.unwrap()
}
