use mcp_sdk::error::McpError;

#[test]
fn test_error_codes() {
    assert_eq!(
        McpError::Parse(serde_json::from_str::<serde_json::Value>("bad").unwrap_err()).code(),
        -32700
    );
    assert_eq!(McpError::MethodNotFound("foo".into()).code(), -32601);
    assert_eq!(McpError::InvalidParams("bad".into()).code(), -32602);
    assert_eq!(McpError::Internal("oops".into()).code(), -32603);
    assert_eq!(McpError::ToolNotFound("missing".into()).code(), -32601);
    assert_eq!(McpError::ResourceNotFound("missing".into()).code(), -32002);
    assert_eq!(McpError::PromptNotFound("missing".into()).code(), -32002);
    assert_eq!(McpError::Transport("io".into()).code(), -32000);
}

#[test]
fn test_error_display_messages() {
    let parse_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
    assert!(McpError::Parse(parse_err)
        .to_string()
        .contains("JSON-RPC parse error"));

    assert_eq!(
        McpError::MethodNotFound("unknown".into()).to_string(),
        "Method not found: unknown"
    );
    assert_eq!(
        McpError::InvalidParams("missing field".into()).to_string(),
        "Invalid params: missing field"
    );
    assert_eq!(
        McpError::Internal("panic".into()).to_string(),
        "Internal error: panic"
    );
    assert_eq!(
        McpError::ToolNotFound("mytool".into()).to_string(),
        "Tool not found: mytool"
    );
    assert_eq!(
        McpError::ResourceNotFound("file:///x".into()).to_string(),
        "Resource not found: file:///x"
    );
    assert_eq!(
        McpError::PromptNotFound("myprompt".into()).to_string(),
        "Prompt not found: myprompt"
    );
    assert_eq!(
        McpError::Transport("connection reset".into()).to_string(),
        "Transport error: connection reset"
    );
}

#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let mcp_err: McpError = io_err.into();
    match mcp_err {
        McpError::Transport(msg) => assert!(msg.contains("refused")),
        _ => panic!("Expected Transport error"),
    }
}

#[test]
fn test_serde_error_conversion() {
    let serde_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
    let mcp_err: McpError = serde_err.into();
    match mcp_err {
        McpError::Parse(_) => {}
        _ => panic!("Expected Parse error"),
    }
}

#[test]
fn test_resource_not_found_code() {
    let err = McpError::ResourceNotFound("file:///missing".into());
    assert_eq!(err.code(), -32002);
}

#[test]
fn test_prompt_not_found_code() {
    let err = McpError::PromptNotFound("missing".into());
    assert_eq!(err.code(), -32002);
}

#[test]
fn test_resource_not_found_display() {
    let err = McpError::ResourceNotFound("file:///test.txt".into());
    assert_eq!(err.to_string(), "Resource not found: file:///test.txt");
}

#[test]
fn test_prompt_not_found_display() {
    let err = McpError::PromptNotFound("review".into());
    assert_eq!(err.to_string(), "Prompt not found: review");
}

#[test]
fn test_method_not_found_with_empty_string() {
    let err = McpError::MethodNotFound("".into());
    assert_eq!(err.code(), -32601);
    assert_eq!(err.to_string(), "Method not found: ");
}

#[test]
fn test_invalid_params_with_empty_string() {
    let err = McpError::InvalidParams("".into());
    assert_eq!(err.code(), -32602);
}

#[test]
fn test_transport_with_empty_string() {
    let err = McpError::Transport("".into());
    assert_eq!(err.code(), -32000);
}

#[test]
fn test_tool_not_found_with_special_chars() {
    let err = McpError::ToolNotFound("tool/with:special@chars".into());
    assert_eq!(err.code(), -32601);
    assert!(err.to_string().contains("tool/with:special@chars"));
}

#[test]
fn test_internal_with_long_message() {
    let long_msg = "x".repeat(10000);
    let err = McpError::Internal(long_msg.clone());
    assert_eq!(err.code(), -32603);
    assert_eq!(err.to_string(), format!("Internal error: {}", long_msg));
}

#[test]
fn test_error_with_unicode() {
    let err = McpError::ToolNotFound("工具".into());
    assert_eq!(err.to_string(), "Tool not found: 工具");
}

#[tokio::test]
async fn test_join_error_conversion() {
    let handle = tokio::spawn(async {
        panic!("test panic");
    });
    let join_err = handle.await.unwrap_err();
    let mcp_err: McpError = join_err.into();
    match mcp_err {
        McpError::Internal(msg) => assert!(msg.contains("test panic")),
        _ => panic!("Expected Internal error"),
    }
}

#[test]
fn test_result_type_alias() {
    let ok_result: mcp_sdk::McpResult<i32> = Ok(42);
    assert_eq!(ok_result.unwrap(), 42);

    let err_result: mcp_sdk::McpResult<i32> = Err(McpError::Internal("fail".into()));
    assert!(err_result.is_err());
}
