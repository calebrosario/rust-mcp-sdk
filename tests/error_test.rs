use mcp_sdk::error::McpError;

#[test]
fn test_error_codes() {
    assert_eq!(McpError::Parse(serde_json::from_str::<serde_json::Value>("bad").unwrap_err()).code(), -32700);
    assert_eq!(McpError::MethodNotFound("foo".into()).code(), -32601);
    assert_eq!(McpError::InvalidParams("bad".into()).code(), -32602);
    assert_eq!(McpError::Internal("oops".into()).code(), -32603);
    assert_eq!(McpError::ToolNotFound("missing".into()).code(), -32601);
    assert_eq!(McpError::Transport("io".into()).code(), -32000);
}

#[test]
fn test_error_display_messages() {
    let e = McpError::MethodNotFound("foo/bar".into());
    assert!(e.to_string().contains("foo/bar"));

    let e = McpError::ToolNotFound("echo".into());
    assert!(e.to_string().contains("echo"));

    let e = McpError::InvalidParams("missing name".into());
    assert!(e.to_string().contains("missing name"));
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
    let result: Result<serde_json::Value, _> = serde_json::from_str("not valid json");
    let serde_err = result.unwrap_err();
    let mcp_err: McpError = serde_err.into();
    match mcp_err {
        McpError::Parse(_) => {}
        _ => panic!("Expected Parse error"),
    }
}
