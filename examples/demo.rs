use mcp_sdk::{
    Content, McpServer, PromptBuilder, PromptMessage, ResourceBuilder, ResourceContents,
    StdioTransport, ToolBuilder,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("mcp_sdk=debug,info")
        .init();

    let server = Arc::new(McpServer::new("demo-server", "0.2.0"));

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

    server
        .register_resource(
            ResourceBuilder::new("config://app", "App Config")
                .description("Application configuration")
                .mime_type("application/json")
                .handler(|_uri| async move {
                    Ok(vec![ResourceContents::Text {
                        uri: "config://app".into(),
                        mime_type: Some("application/json".into()),
                        text: r#"{"name":"demo","version":"0.2.0","debug":true}"#.into(),
                    }])
                }),
        )
        .await;

    server
        .register_resource(
            ResourceBuilder::new("file:///readme", "Readme")
                .description("Project readme")
                .mime_type("text/markdown")
                .handler(|_uri| async move {
                    Ok(vec![ResourceContents::Text {
                        uri: "file:///readme".into(),
                        mime_type: Some("text/markdown".into()),
                        text:
                            "# Demo Server\n\nA demo MCP server with tools, resources, and prompts."
                                .into(),
                    }])
                }),
        )
        .await;

    server
        .register_prompt(
            PromptBuilder::new("code_review")
                .description("Generate a code review prompt")
                .argument_with("language", |a| {
                    a.description("Programming language").required()
                })
                .argument("focus")
                .handler(|args| async move {
                    let lang = args
                        .get("language")
                        .and_then(|v| v.as_str())
                        .unwrap_or("rust");
                    let focus = args
                        .get("focus")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general quality");

                    Ok(mcp_sdk::GetPromptResult {
                        description: Some(format!("Review {lang} code — focus: {focus}")),
                        messages: vec![
                            PromptMessage {
                                role: "user".into(),
                                content: Content::text(format!(
                                    "Please review this {lang} code. Focus on {focus}."
                                )),
                            },
                            PromptMessage {
                                role: "assistant".into(),
                                content: Content::text(
                                    "I'll review the code for {focus}. Please share the code."
                                        .replace("{focus}", focus),
                                ),
                            },
                        ],
                    })
                }),
        )
        .await;

    server
        .register_prompt(
            PromptBuilder::new("summarize")
                .description("Summarize text")
                .argument_with("text", |a| a.description("Text to summarize").required())
                .handler(|args| async move {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    Ok(mcp_sdk::GetPromptResult {
                        description: Some("Text summarization".into()),
                        messages: vec![PromptMessage {
                            role: "user".into(),
                            content: Content::text(format!("Summarize: {text}")),
                        }],
                    })
                }),
        )
        .await;

    tracing::info!("Starting demo server: 2 tools, 2 resources, 2 prompts");

    if let Err(e) = StdioTransport::serve(server).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
