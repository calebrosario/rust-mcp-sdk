pub mod error;
pub mod prompt;
pub mod protocol;
pub mod resource;
pub mod server;
pub mod tool;
pub mod transport;

pub use error::{McpError, McpResult};
pub use prompt::{PromptArgumentBuilder, PromptBuilder, PromptHandler, PromptRegistry};
pub use protocol::*;
pub use resource::{ResourceBuilder, ResourceHandler, ResourceRegistry};
pub use server::McpServer;
pub use tool::{ToolBuilder, ToolHandler, ToolRegistry};
pub use transport::StdioTransport;

#[cfg(feature = "http")]
pub use transport::HttpTransport;
