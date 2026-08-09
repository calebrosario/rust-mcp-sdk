pub mod error;
pub mod protocol;
pub mod server;
pub mod tool;
pub mod transport;

pub use error::{McpError, McpResult};
pub use protocol::*;
pub use server::McpServer;
pub use tool::{ToolBuilder, ToolRegistry, ToolHandler};
pub use transport::StdioTransport;

#[cfg(feature = "http")]
pub use transport::HttpTransport;
