use mcp_sdk::{McpServer, StdioTransport, ToolBuilder};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("mcp_sdk=debug,info")
        .init();

    let server = Arc::new(McpServer::new("echo-server", "0.1.0"));

    server
        .register_tool(
            ToolBuilder::new("echo")
                .description("Echo back the provided message")
                .schema(json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to echo back"
                        }
                    },
                    "required": ["message"]
                }))
                .handler(|args| async move {
                    let msg = args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("no message provided");
                    Ok(mcp_sdk::CallToolResult::text(msg))
                }),
        )
        .await;

    server
        .register_tool(
            ToolBuilder::new("add")
                .description("Add two numbers")
                .schema(json!({
                    "type": "object",
                    "properties": {
                        "a": { "type": "number" },
                        "b": { "type": "number" }
                    },
                    "required": ["a", "b"]
                }))
                .handler(|args| async move {
                    let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    Ok(mcp_sdk::CallToolResult::text(format!("{}", a + b)))
                }),
        )
        .await;

    tracing::info!("Starting echo server with 2 tools");

    if let Err(e) = StdioTransport::serve(server).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
