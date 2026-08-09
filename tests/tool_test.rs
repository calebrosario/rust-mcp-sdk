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
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
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

#[tokio::test]
async fn test_tool_builder_default_schema() {
    let (_, tool, _) = ToolBuilder::new("defaults").build();
    assert_eq!(
        tool.input_schema,
        json!({"type": "object", "properties": {}})
    );
}

#[tokio::test]
async fn test_tool_builder_custom_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "path": {"type": "string"},
            "recursive": {"type": "boolean", "default": false}
        },
        "required": ["path"]
    });
    let (_, tool, _) = ToolBuilder::new("walk")
        .schema(schema.clone())
        .handler(|_| async { Ok(CallToolResult::text("done")) })
        .build();

    assert_eq!(tool.input_schema, schema);
}

#[tokio::test]
async fn test_registry_overwrite_tool() {
    let mut registry = ToolRegistry::new();

    registry.register(
        "tool".into(),
        Tool {
            name: "tool".into(),
            description: Some("v1".into()),
            input_schema: json!({}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("v1")) })),
    );

    registry.register(
        "tool".into(),
        Tool {
            name: "tool".into(),
            description: Some("v2".into()),
            input_schema: json!({}),
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(CallToolResult::text("v2")) })),
    );

    assert_eq!(registry.list().len(), 1);

    let result = registry
        .call(CallToolParams {
            name: "tool".into(),
            arguments: json!({}),
        })
        .await
        .unwrap();
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "v2"),
        _ => panic!("Expected v2"),
    }
}

#[tokio::test]
async fn test_registry_empty() {
    let registry = ToolRegistry::new();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_registry_default() {
    let registry = ToolRegistry::default();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_tool_handler_returns_multiple_content() {
    let (_, _, handler) = ToolBuilder::new("multi")
        .handler(|_| async {
            Ok(CallToolResult {
                content: vec![
                    Content::text("line 1"),
                    Content::text("line 2"),
                    Content::Image {
                        data: "aGVsbG8=".into(),
                        mime_type: "image/png".into(),
                    },
                ],
                is_error: false,
            })
        })
        .build();

    let result = handler(json!({})).await.unwrap();
    assert_eq!(result.content.len(), 3);
    assert!(matches!(result.content[0], Content::Text { .. }));
    assert!(matches!(result.content[1], Content::Text { .. }));
    assert!(matches!(result.content[2], Content::Image { .. }));
}

#[tokio::test]
async fn test_tool_handler_with_complex_args() {
    let (_, _, handler) = ToolBuilder::new("parse")
        .handler(|args| async move {
            let nested = args.get("config").and_then(|v| v.get("debug"));
            let debug = nested.and_then(|v| v.as_bool()).unwrap_or(false);
            Ok(CallToolResult::text(format!("debug={debug}")))
        })
        .build();

    let result = handler(json!({"config": {"debug": true}})).await.unwrap();
    match &result.content[0] {
        Content::Text { text } => assert_eq!(text, "debug=true"),
        _ => panic!("Expected text"),
    }
}

#[tokio::test]
async fn test_tool_handler_returns_image() {
    let (_, _, handler) = ToolBuilder::new("screenshot")
        .handler(|_| async {
            Ok(CallToolResult {
                content: vec![Content::Image {
                    data: "iVBORw0KGgo=".into(),
                    mime_type: "image/png".into(),
                }],
                is_error: false,
            })
        })
        .build();

    let result = handler(json!({})).await.unwrap();
    match &result.content[0] {
        Content::Image { data, mime_type } => {
            assert_eq!(data, "iVBORw0KGgo=");
            assert_eq!(mime_type, "image/png");
        }
        _ => panic!("Expected image"),
    }
}

#[tokio::test]
async fn test_tool_builder_with_string_arg() {
    let name: String = "dynamic".into();
    let (tool_name, _, _) = ToolBuilder::new(name).build();
    assert_eq!(tool_name, "dynamic");
}

#[tokio::test]
async fn test_tool_handler_called_multiple_times() {
    let (_, _, handler) = ToolBuilder::new("counter")
        .handler(|args| async move {
            let n = args.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(CallToolResult::text(format!("n={n}")))
        })
        .build();

    for i in 0..5 {
        let result = handler(json!({"n": i})).await.unwrap();
        match &result.content[0] {
            Content::Text { text } => assert_eq!(text, &format!("n={i}")),
            _ => panic!("Expected text"),
        }
    }
}

#[tokio::test]
async fn test_tool_handler_returns_empty_content() {
    let (_, _, handler) = ToolBuilder::new("empty")
        .handler(|_| async {
            Ok(CallToolResult {
                content: vec![],
                is_error: false,
            })
        })
        .build();

    let result = handler(json!({})).await.unwrap();
    assert!(result.content.is_empty());
    assert!(!result.is_error);
}
