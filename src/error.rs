use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("JSON-RPC parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Transport error: {0}")]
    Transport(String),
}

impl McpError {
    pub fn code(&self) -> i32 {
        match self {
            McpError::Parse(_) => -32700,
            McpError::MethodNotFound(_) => -32601,
            McpError::InvalidParams(_) => -32602,
            McpError::Internal(_) => -32603,
            McpError::ToolNotFound(_) => -32601,
            McpError::ResourceNotFound(_) => -32002,
            McpError::PromptNotFound(_) => -32002,
            McpError::Transport(_) => -32000,
        }
    }
}

impl From<std::io::Error> for McpError {
    fn from(e: std::io::Error) -> Self {
        McpError::Transport(e.to_string())
    }
}

impl From<tokio::task::JoinError> for McpError {
    fn from(e: tokio::task::JoinError) -> Self {
        McpError::Internal(e.to_string())
    }
}

pub type McpResult<T> = Result<T, McpError>;
