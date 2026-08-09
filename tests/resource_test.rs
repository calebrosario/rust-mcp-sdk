use mcp_sdk::{McpError, ResourceBuilder, ResourceContents, ResourceRegistry};
use serde_json::json;

#[tokio::test]
async fn test_resource_registry_register_and_list() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///test.txt", "Test File")
        .description("A test resource")
        .mime_type("text/plain")
        .handler(|_uri| async move {
            Ok(vec![ResourceContents::Text {
                uri: "file:///test.txt".into(),
                mime_type: Some("text/plain".into()),
                text: "hello world".into(),
            }])
        })
        .build();

    registry.register(uri, resource, handler);

    let resources = registry.list();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "file:///test.txt");
    assert_eq!(resources[0].name, "Test File");
    assert_eq!(resources[0].description, Some("A test resource".into()));
    assert_eq!(resources[0].mime_type, Some("text/plain".into()));
}

#[tokio::test]
async fn test_resource_registry_read_existing() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///data.json", "Data")
        .handler(|_| async move {
            Ok(vec![ResourceContents::Text {
                uri: "file:///data.json".into(),
                mime_type: Some("application/json".into()),
                text: r#"{"key": "value"}"#.into(),
            }])
        })
        .build();

    registry.register(uri, resource, handler);

    let contents = registry.read("file:///data.json").await.unwrap();
    assert_eq!(contents.len(), 1);
    match &contents[0] {
        ResourceContents::Text { text, .. } => assert_eq!(text, r#"{"key": "value"}"#),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_resource_registry_read_missing() {
    let registry = ResourceRegistry::new();
    let result = registry.read("file:///nonexistent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound(uri) => assert_eq!(uri, "file:///nonexistent"),
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_resource_builder_basic() {
    let (uri, resource, _handler) = ResourceBuilder::new("file:///log.txt", "Log File")
        .description("Application log")
        .mime_type("text/plain")
        .handler(|_| async move { Ok(vec![]) })
        .build();

    assert_eq!(uri, "file:///log.txt");
    assert_eq!(resource.uri, "file:///log.txt");
    assert_eq!(resource.name, "Log File");
    assert_eq!(resource.description, Some("Application log".into()));
    assert_eq!(resource.mime_type, Some("text/plain".into()));
}

#[test]
fn test_resource_builder_without_description() {
    let (uri, resource, _handler) = ResourceBuilder::new("file:///bare.txt", "Bare").build();
    assert_eq!(uri, "file:///bare.txt");
    assert!(resource.description.is_none());
    assert!(resource.mime_type.is_none());
}

#[test]
fn test_resource_builder_without_mime_type() {
    let (_, resource, _) = ResourceBuilder::new("file:///doc.md", "Doc")
        .description("A doc")
        .build();
    assert_eq!(resource.description, Some("A doc".into()));
    assert!(resource.mime_type.is_none());
}

#[test]
fn test_resource_builder_without_handler_uses_default() {
    let (uri, _resource, handler) = ResourceBuilder::new("file:///default", "Default").build();
    assert_eq!(uri, "file:///default");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let contents = rt.block_on(handler("file:///default")).unwrap();
    assert_eq!(contents.len(), 1);
    match &contents[0] {
        ResourceContents::Text { uri, text, .. } => {
            assert_eq!(uri, "file:///default");
            assert!(text.is_empty());
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_resource_serialization() {
    let resource = mcp_sdk::Resource {
        uri: "file:///test.txt".into(),
        name: "Test".into(),
        description: Some("A test file".into()),
        mime_type: Some("text/plain".into()),
    };
    let json = serde_json::to_string(&resource).unwrap();
    assert!(json.contains(r#""uri":"file:///test.txt""#));
    assert!(json.contains(r#""name":"Test""#));
    assert!(json.contains(r#""description":"A test file""#));
    assert!(json.contains(r#""mimeType":"text/plain""#));
}

#[test]
fn test_resource_serialization_without_optional_fields() {
    let resource = mcp_sdk::Resource {
        uri: "file:///bare".into(),
        name: "Bare".into(),
        description: None,
        mime_type: None,
    };
    let json = serde_json::to_string(&resource).unwrap();
    assert!(!json.contains("description"));
    assert!(!json.contains("mimeType"));
}

#[test]
fn test_resource_deserialization() {
    let json = json!({
        "uri": "file:///test.txt",
        "name": "Test",
        "description": "A file",
        "mimeType": "text/plain"
    });
    let resource: mcp_sdk::Resource = serde_json::from_value(json).unwrap();
    assert_eq!(resource.uri, "file:///test.txt");
    assert_eq!(resource.name, "Test");
}

#[test]
fn test_resource_deserialization_minimal() {
    let json = json!({"uri": "x", "name": "y"});
    let resource: mcp_sdk::Resource = serde_json::from_value(json).unwrap();
    assert_eq!(resource.uri, "x");
    assert_eq!(resource.name, "y");
    assert!(resource.description.is_none());
    assert!(resource.mime_type.is_none());
}

#[test]
fn test_resource_contents_text_serialization() {
    let contents = ResourceContents::Text {
        uri: "file:///test.txt".into(),
        mime_type: Some("text/plain".into()),
        text: "hello".into(),
    };
    let json = serde_json::to_string(&contents).unwrap();
    assert!(json.contains(r#""type":"text""#));
    assert!(json.contains(r#""text":"hello""#));
}

#[test]
fn test_resource_contents_text_without_mime_type() {
    let contents = ResourceContents::Text {
        uri: "file:///test.txt".into(),
        mime_type: None,
        text: "hello".into(),
    };
    let json = serde_json::to_string(&contents).unwrap();
    assert!(!json.contains("mimeType"));
}

#[test]
fn test_resource_contents_blob_serialization() {
    let contents = ResourceContents::Blob {
        uri: "file:///image.png".into(),
        mime_type: Some("image/png".into()),
        blob: "aGVsbG8=".into(),
    };
    let json = serde_json::to_string(&contents).unwrap();
    assert!(json.contains(r#""type":"blob""#));
    assert!(json.contains(r#""blob":"aGVsbG8=""#));
}

#[test]
fn test_resource_contents_blob_without_mime_type() {
    let contents = ResourceContents::Blob {
        uri: "file:///data".into(),
        mime_type: None,
        blob: "AAAA".into(),
    };
    let json = serde_json::to_string(&contents).unwrap();
    assert!(!json.contains("mimeType"));
}

#[test]
fn test_resource_contents_deserialization() {
    let json = json!({
        "type": "text",
        "uri": "file:///test.txt",
        "mimeType": "text/plain",
        "text": "hello world"
    });
    let contents: ResourceContents = serde_json::from_value(json).unwrap();
    match contents {
        ResourceContents::Text { text, uri, .. } => {
            assert_eq!(text, "hello world");
            assert_eq!(uri, "file:///test.txt");
        }
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn test_resource_contents_blob_deserialization() {
    let json = json!({
        "type": "blob",
        "uri": "file:///img.png",
        "blob": "aGVsbG8="
    });
    let contents: ResourceContents = serde_json::from_value(json).unwrap();
    match contents {
        ResourceContents::Blob { blob, uri, .. } => {
            assert_eq!(blob, "aGVsbG8=");
            assert_eq!(uri, "file:///img.png");
        }
        _ => panic!("Expected Blob variant"),
    }
}

#[tokio::test]
async fn test_resource_registry_multiple_resources() {
    let mut registry = ResourceRegistry::new();

    registry.register(
        "file:///a".into(),
        mcp_sdk::Resource {
            uri: "file:///a".into(),
            name: "A".into(),
            description: None,
            mime_type: None,
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(vec![]) })),
    );
    registry.register(
        "file:///b".into(),
        mcp_sdk::Resource {
            uri: "file:///b".into(),
            name: "B".into(),
            description: None,
            mime_type: None,
        },
        std::sync::Arc::new(|_| Box::pin(async { Ok(vec![]) })),
    );

    assert_eq!(registry.list().len(), 2);
}

#[tokio::test]
async fn test_resource_registry_overwrite() {
    let mut registry = ResourceRegistry::new();

    registry.register(
        "file:///x".into(),
        mcp_sdk::Resource {
            uri: "file:///x".into(),
            name: "v1".into(),
            description: None,
            mime_type: None,
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(vec![ResourceContents::Text {
                    uri: "file:///x".into(),
                    mime_type: None,
                    text: "v1".into(),
                }])
            })
        }),
    );

    registry.register(
        "file:///x".into(),
        mcp_sdk::Resource {
            uri: "file:///x".into(),
            name: "v2".into(),
            description: None,
            mime_type: None,
        },
        std::sync::Arc::new(|_| {
            Box::pin(async {
                Ok(vec![ResourceContents::Text {
                    uri: "file:///x".into(),
                    mime_type: None,
                    text: "v2".into(),
                }])
            })
        }),
    );

    assert_eq!(registry.list().len(), 1);
    assert_eq!(registry.list()[0].name, "v2");

    let contents = registry.read("file:///x").await.unwrap();
    match &contents[0] {
        ResourceContents::Text { text, .. } => assert_eq!(text, "v2"),
        _ => panic!("Expected v2"),
    }
}

#[tokio::test]
async fn test_resource_registry_empty() {
    let registry = ResourceRegistry::new();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_resource_registry_default() {
    let registry = ResourceRegistry::default();
    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn test_resource_handler_returns_blob() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///image.png", "Image")
        .handler(|_| async move {
            Ok(vec![ResourceContents::Blob {
                uri: "file:///image.png".into(),
                mime_type: Some("image/png".into()),
                blob: "iVBORw0KGgo=".into(),
            }])
        })
        .build();

    registry.register(uri, resource, handler);

    let contents = registry.read("file:///image.png").await.unwrap();
    match &contents[0] {
        ResourceContents::Blob { blob, .. } => assert_eq!(blob, "iVBORw0KGgo="),
        _ => panic!("Expected blob"),
    }
}

#[tokio::test]
async fn test_resource_handler_returns_multiple_contents() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///multi", "Multi")
        .handler(|_| async move {
            Ok(vec![
                ResourceContents::Text {
                    uri: "file:///multi".into(),
                    mime_type: None,
                    text: "part1".into(),
                },
                ResourceContents::Text {
                    uri: "file:///multi".into(),
                    mime_type: None,
                    text: "part2".into(),
                },
            ])
        })
        .build();

    registry.register(uri, resource, handler);

    let contents = registry.read("file:///multi").await.unwrap();
    assert_eq!(contents.len(), 2);
}

#[tokio::test]
async fn test_resource_handler_returns_error() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///err", "Err")
        .handler(|_| async move { Err(McpError::Internal("read failed".into())) })
        .build();

    registry.register(uri, resource, handler);

    let result = registry.read("file:///err").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::Internal(msg) => assert_eq!(msg, "read failed"),
        _ => panic!("Expected Internal error"),
    }
}

#[tokio::test]
async fn test_resource_handler_receives_uri() {
    let mut registry = ResourceRegistry::new();
    let (uri, resource, handler) = ResourceBuilder::new("file:///echo", "Echo")
        .handler(|uri| {
            let uri = uri.to_string();
            async move {
                Ok(vec![ResourceContents::Text {
                    uri: uri.clone(),
                    mime_type: None,
                    text: uri,
                }])
            }
        })
        .build();

    registry.register(uri, resource, handler);

    let contents = registry.read("file:///echo").await.unwrap();
    match &contents[0] {
        ResourceContents::Text { text, .. } => assert_eq!(text, "file:///echo"),
        _ => panic!("Expected text"),
    }
}

#[tokio::test]
async fn test_resource_builder_with_string_args() {
    let uri: String = "dynamic://uri".into();
    let name: String = "Dynamic".into();
    let (built_uri, built_resource, _) = ResourceBuilder::new(uri, name).build();
    assert_eq!(built_uri, "dynamic://uri");
    assert_eq!(built_resource.name, "Dynamic");
}
