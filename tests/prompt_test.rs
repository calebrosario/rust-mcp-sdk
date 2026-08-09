use mcp_sdk::{Content, McpError, PromptArgumentBuilder, PromptBuilder, PromptRegistry};
use serde_json::json;

#[tokio::test]
async fn test_prompt_registry_register_and_list() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("greet")
        .description("Greet someone")
        .argument("name")
        .handler(|args| async move {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
            Ok(mcp_sdk::GetPromptResult {
                description: Some("A greeting".into()),
                messages: vec![mcp_sdk::PromptMessage {
                    role: "assistant".into(),
                    content: Content::text(format!("Hello, {name}!")),
                }],
            })
        })
        .build();

    registry.register(name, prompt, handler);

    let prompts = registry.list();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "greet");
    assert_eq!(prompts[0].description, Some("Greet someone".into()));
    assert_eq!(prompts[0].arguments.len(), 1);
    assert_eq!(prompts[0].arguments[0].name, "name");
}

#[tokio::test]
async fn test_prompt_registry_get_existing() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("code_review")
        .description("Review code")
        .handler(|args| async move {
            let lang = args.get("lang").and_then(|v| v.as_str()).unwrap_or("rust");
            Ok(mcp_sdk::GetPromptResult {
                description: Some("Code review".into()),
                messages: vec![mcp_sdk::PromptMessage {
                    role: "user".into(),
                    content: Content::text(format!("Review this {lang} code")),
                }],
            })
        })
        .build();

    registry.register(name, prompt, handler);

    let result = registry
        .get("code_review", json!({"lang": "python"}))
        .await
        .unwrap();
    assert_eq!(result.description, Some("Code review".into()));
    assert_eq!(result.messages.len(), 1);
    match &result.messages[0].content {
        Content::Text { text } => assert_eq!(text, "Review this python code"),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_prompt_registry_get_missing() {
    let registry = PromptRegistry::new();
    let result = registry.get("nonexistent", json!({})).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound(name) => assert_eq!(name, "nonexistent"),
        _ => panic!("Expected PromptNotFound error"),
    }
}

#[tokio::test]
async fn test_prompt_builder_basic() {
    let (name, prompt, _handler) = PromptBuilder::new("summarize")
        .description("Summarize text")
        .argument("text")
        .argument_with("max_length", |a| a.description("Max words").required())
        .handler(|_| async move {
            Ok(mcp_sdk::GetPromptResult {
                description: None,
                messages: vec![],
            })
        })
        .build();

    assert_eq!(name, "summarize");
    assert_eq!(prompt.name, "summarize");
    assert_eq!(prompt.description, Some("Summarize text".into()));
    assert_eq!(prompt.arguments.len(), 2);
    assert_eq!(prompt.arguments[0].name, "text");
    assert!(!prompt.arguments[0].required);
    assert_eq!(prompt.arguments[1].name, "max_length");
    assert_eq!(prompt.arguments[1].description, Some("Max words".into()));
    assert!(prompt.arguments[1].required);
}

#[tokio::test]
async fn test_prompt_builder_without_description() {
    let (name, prompt, _handler) = PromptBuilder::new("bare").build();
    assert_eq!(name, "bare");
    assert!(prompt.description.is_none());
    assert!(prompt.arguments.is_empty());
}

#[tokio::test]
async fn test_prompt_builder_without_handler_uses_default() {
    let (name, _, handler) = PromptBuilder::new("default_prompt").build();
    assert_eq!(name, "default_prompt");

    let result = handler(json!({})).await.unwrap();
    assert!(result.messages.is_empty());
    assert!(result.description.is_none());
}

#[tokio::test]
async fn test_prompt_builder_multiple_arguments() {
    let (_, prompt, _) = PromptBuilder::new("translate")
        .argument("source_lang")
        .argument("target_lang")
        .argument("text")
        .argument_with("formal", |a| a.description("Formal tone").required())
        .build();

    assert_eq!(prompt.arguments.len(), 4);
    assert_eq!(prompt.arguments[0].name, "source_lang");
    assert_eq!(prompt.arguments[1].name, "target_lang");
    assert_eq!(prompt.arguments[2].name, "text");
    assert_eq!(prompt.arguments[3].name, "formal");
    assert!(prompt.arguments[3].required);
}

#[tokio::test]
async fn test_prompt_builder_argument_with_description_only() {
    let (_, prompt, _) = PromptBuilder::new("p")
        .argument_with("arg1", |a| a.description("An argument"))
        .build();

    assert_eq!(prompt.arguments.len(), 1);
    assert_eq!(prompt.arguments[0].description, Some("An argument".into()));
    assert!(!prompt.arguments[0].required);
}

#[tokio::test]
async fn test_prompt_builder_argument_with_required_only() {
    let (_, prompt, _) = PromptBuilder::new("p")
        .argument_with("arg1", |a| a.required())
        .build();

    assert_eq!(prompt.arguments.len(), 1);
    assert!(prompt.arguments[0].description.is_none());
    assert!(prompt.arguments[0].required);
}

#[tokio::test]
async fn test_prompt_builder_argument_with_neither() {
    let (_, prompt, _) = PromptBuilder::new("p").argument_with("arg1", |a| a).build();

    assert_eq!(prompt.arguments.len(), 1);
    assert!(prompt.arguments[0].description.is_none());
    assert!(!prompt.arguments[0].required);
}

#[tokio::test]
async fn test_prompt_argument_builder_build() {
    let arg = PromptArgumentBuilder {
        name: "test".into(),
        description: None,
        required: false,
    }
    .description("A test arg")
    .required()
    .build();

    assert_eq!(arg.name, "test");
    assert_eq!(arg.description, Some("A test arg".into()));
    assert!(arg.required);
}

#[tokio::test]
async fn test_prompt_argument_builder_default() {
    let arg = PromptArgumentBuilder {
        name: "bare".into(),
        description: None,
        required: false,
    }
    .build();

    assert_eq!(arg.name, "bare");
    assert!(arg.description.is_none());
    assert!(!arg.required);
}

#[tokio::test]
async fn test_prompt_serialization() {
    let prompt = mcp_sdk::Prompt {
        name: "review".into(),
        description: Some("Review code".into()),
        arguments: vec![mcp_sdk::PromptArgument {
            name: "language".into(),
            description: Some("Programming language".into()),
            required: true,
        }],
    };
    let json = serde_json::to_string(&prompt).unwrap();
    assert!(json.contains(r#""name":"review""#));
    assert!(json.contains(r#""description":"Review code""#));
    assert!(json.contains(r#""required":true"#));
}

#[tokio::test]
async fn test_prompt_serialization_without_arguments() {
    let prompt = mcp_sdk::Prompt {
        name: "simple".into(),
        description: Some("Simple".into()),
        arguments: vec![],
    };
    let json = serde_json::to_string(&prompt).unwrap();
    assert!(!json.contains("arguments"));
}

#[tokio::test]
async fn test_prompt_serialization_without_description() {
    let prompt = mcp_sdk::Prompt {
        name: "nodesc".into(),
        description: None,
        arguments: vec![],
    };
    let json = serde_json::to_string(&prompt).unwrap();
    assert!(!json.contains("description"));
}

#[tokio::test]
async fn test_prompt_message_serialization() {
    let msg = mcp_sdk::PromptMessage {
        role: "assistant".into(),
        content: Content::text("Hello!"),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""role":"assistant""#));
    assert!(json.contains(r#""type":"text""#));
}

#[tokio::test]
async fn test_prompt_message_with_image() {
    let msg = mcp_sdk::PromptMessage {
        role: "user".into(),
        content: Content::Image {
            data: "aGVsbG8=".into(),
            mime_type: "image/png".into(),
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"image""#));
    assert!(json.contains(r#""mimeType":"image/png""#));
}

#[tokio::test]
async fn test_get_prompt_result_serialization() {
    let result = mcp_sdk::GetPromptResult {
        description: Some("A review prompt".into()),
        messages: vec![
            mcp_sdk::PromptMessage {
                role: "user".into(),
                content: Content::text("Review this code"),
            },
            mcp_sdk::PromptMessage {
                role: "assistant".into(),
                content: Content::text("Sure!"),
            },
        ],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""description":"A review prompt""#));
    assert!(json.contains(r#""messages""#));
}

#[tokio::test]
async fn test_get_prompt_result_without_description() {
    let result = mcp_sdk::GetPromptResult {
        description: None,
        messages: vec![],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("description"));
    assert!(json.contains(r#""messages":[]"#));
}

#[tokio::test]
async fn test_prompt_registry_multiple_prompts() {
    let mut registry = PromptRegistry::new();
    registry.register(
        "a".into(),
        mcp_sdk::Prompt {
            name: "a".into(),
            description: None,
            arguments: vec![],
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(mcp_sdk::GetPromptResult {
                    description: None,
                    messages: vec![],
                })
            })
        }),
    );
    registry.register(
        "b".into(),
        mcp_sdk::Prompt {
            name: "b".into(),
            description: None,
            arguments: vec![],
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(mcp_sdk::GetPromptResult {
                    description: None,
                    messages: vec![],
                })
            })
        }),
    );

    assert_eq!(registry.list().len(), 2);
}

#[tokio::test]
async fn test_prompt_registry_overwrite() {
    let mut registry = PromptRegistry::new();

    registry.register(
        "p".into(),
        mcp_sdk::Prompt {
            name: "p".into(),
            description: Some("v1".into()),
            arguments: vec![],
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(mcp_sdk::GetPromptResult {
                    description: Some("v1".into()),
                    messages: vec![],
                })
            })
        }),
    );

    registry.register(
        "p".into(),
        mcp_sdk::Prompt {
            name: "p".into(),
            description: Some("v2".into()),
            arguments: vec![],
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(mcp_sdk::GetPromptResult {
                    description: Some("v2".into()),
                    messages: vec![],
                })
            })
        }),
    );

    assert_eq!(registry.list().len(), 1);
    assert_eq!(registry.list()[0].description, Some("v2".into()));

    let result = registry.get("p", json!({})).await.unwrap();
    assert_eq!(result.description, Some("v2".into()));
}

#[tokio::test]
async fn test_prompt_registry_empty() {
    let registry = PromptRegistry::new();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_prompt_registry_default() {
    let registry = PromptRegistry::default();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_prompt_handler_receives_null_arguments() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("null_args")
        .handler(|args| async move {
            assert!(args.is_null());
            Ok(mcp_sdk::GetPromptResult {
                description: None,
                messages: vec![],
            })
        })
        .build();

    registry.register(name, prompt, handler);
    registry.get("null_args", json!(null)).await.unwrap();
}

#[tokio::test]
async fn test_prompt_handler_receives_empty_object() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("empty_obj")
        .handler(|args| async move {
            assert!(args.is_object());
            Ok(mcp_sdk::GetPromptResult {
                description: None,
                messages: vec![mcp_sdk::PromptMessage {
                    role: "assistant".into(),
                    content: Content::text("ok"),
                }],
            })
        })
        .build();

    registry.register(name, prompt, handler);
    let result = registry.get("empty_obj", json!({})).await.unwrap();
    assert_eq!(result.messages.len(), 1);
}

#[tokio::test]
async fn test_prompt_handler_returns_error() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("fail")
        .handler(|_| async move { Err(McpError::Internal("prompt error".into())) })
        .build();

    registry.register(name, prompt, handler);

    let result = registry.get("fail", json!({})).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => assert_eq!(msg, "prompt error"),
        _ => panic!("Expected Internal error"),
    }
}

#[tokio::test]
async fn test_prompt_builder_with_string_name() {
    let name: String = "dynamic".into();
    let (built_name, _, _) = PromptBuilder::new(name).build();
    assert_eq!(built_name, "dynamic");
}

#[tokio::test]
async fn test_prompt_handler_returns_multiple_messages() {
    let mut registry = PromptRegistry::new();
    let (name, prompt, handler) = PromptBuilder::new("multi")
        .handler(|_| async move {
            Ok(mcp_sdk::GetPromptResult {
                description: Some("Multi".into()),
                messages: vec![
                    mcp_sdk::PromptMessage {
                        role: "system".into(),
                        content: Content::text("You are helpful"),
                    },
                    mcp_sdk::PromptMessage {
                        role: "user".into(),
                        content: Content::text("Question"),
                    },
                    mcp_sdk::PromptMessage {
                        role: "assistant".into(),
                        content: Content::text("Answer"),
                    },
                ],
            })
        })
        .build();

    registry.register(name, prompt, handler);
    let result = registry.get("multi", json!({})).await.unwrap();
    assert_eq!(result.messages.len(), 3);
}
