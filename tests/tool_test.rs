use mcp_sdk::error::McpError;
use mcp_sdk::protocol::*;
use mcp_sdk::tool::{ToolBuilder, ToolRegistry};
use serde_json::json;

#[tokio::test]
async fn test_registry_register_and_list() {
    let mut registry = ToolRegistry::new();

    registry.register(
        "echo".into(),
        Tool {
            name: "echo".into(),
            description: Some("Echo".into()),
            input_schema: json!({"type": "object"}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("ok")) })),
    );

    let tools = registry.list();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
}

#[tokio::test]
async fn test_registry_multiple_tools() {
    let mut registry = ToolRegistry::new();

    registry.register(
        "a".into(),
        Tool {
            name: "a".into(),
            description: None,
            input_schema: json!({}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("a")) })),
    );
    registry.register(
        "b".into(),
        Tool {
            name: "b".into(),
            description: None,
            input_schema: json!({}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("b")) })),
    );

    let tools = registry.list();
    assert_eq!(tools.len(), 2);
}

#[tokio::test]
async fn test_registry_call_existing_tool() {
    let mut registry = ToolRegistry::new();

    registry.register(
        "greet".into(),
        Tool {
            name: "greet".into(),
            description: None,
            input_schema: json!({"type": "object"}),
        },
        std::sync::Arc::new(|args| {
            Box::pin(async move {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                Ok(CallToolResult::text(format!("Hello, {name}!")))
            })
        }),
    );

    let result = registry
        .call(CallToolParams {
            name: "greet".into(),
            arguments: json!({"name": "Rust"}),
        })
        .await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(!tool_result.is_error);
    match &tool_result.content[0] {
        Content::Text { text } => assert_eq!(text, "Hello, Rust!"),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_registry_call_missing_tool() {
    let registry = ToolRegistry::new();

    let result = registry
        .call(CallToolParams {
            name: "nonexistent".into(),
            arguments: json!({}),
        })
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound(name) => assert_eq!(name, "nonexistent"),
        _ => panic!("Expected ToolNotFound error"),
    }
}

#[tokio::test]
async fn test_registry_call_tool_no_arguments() {
    let mut registry = ToolRegistry::new();

    registry.register(
        "ping".into(),
        Tool {
            name: "ping".into(),
            description: None,
            input_schema: json!({"type": "object"}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("pong")) })),
    );

    let result = registry
        .call(CallToolParams {
            name: "ping".into(),
            arguments: json!({}),
        })
        .await;

    assert!(result.is_ok());
    match &result.unwrap().content[0] {
        Content::Text { text } => assert_eq!(text, "pong"),
        _ => panic!("Expected pong"),
    }
}

#[tokio::test]
async fn test_tool_builder_basic() {
    let (name, tool, _handler) = ToolBuilder::new("echo")
        .description("Echo a message")
        .schema(json!({"type": "object", "properties": {"msg": {"type": "string"}}}))
        .handler(|args| async move {
            let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("");
            Ok(CallToolResult::text(msg))
        })
        .build();

    assert_eq!(name, "echo");
    assert_eq!(tool.name, "echo");
    assert!(tool.description.is_some());
}

#[tokio::test]
async fn test_tool_builder_without_description() {
    let (name, tool, _) = ToolBuilder::new("ping")
        .schema(json!({"type": "object"}))
        .handler(|_| async { Ok(CallToolResult::text("pong")) })
        .build();

    assert_eq!(name, "ping");
    assert!(tool.description.is_none());
}

#[tokio::test]
async fn test_tool_builder_without_handler_defaults_to_error() {
    let (name, tool, handler) = ToolBuilder::new("noop").build();
    assert_eq!(name, "noop");
    assert!(tool.description.is_none());

    let result = handler(json!({})).await.unwrap();
    assert!(result.is_error);
}

#[tokio::test]
async fn test_tool_builder_handler_returns_error() {
    let (_, _, handler) = ToolBuilder::new("fail")
        .handler(|_| async { Ok(CallToolResult::error("intentional failure")) })
        .build();

    let result = handler(json!({})).await.unwrap();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "intentional failure"),
        _ => panic!("Expected text"),
    }
}
