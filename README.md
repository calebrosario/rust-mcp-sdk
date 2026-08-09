# rust-mcp-sdk

> Rust implementation of the Model Context Protocol (MCP) — build MCP servers with type safety, async performance, and a single static binary.

[![Crates.io](https://img.shields.io/crates/v/mcp-sdk.svg)](https://crates.io/crates/mcp-sdk)
[![Documentation](https://docs.rs/mcp-sdk/badge.svg)](https://docs.rs/mcp-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io/) is an open standard that lets AI agents (Claude, GPT, etc.) communicate with external tools and services. It uses JSON-RPC 2.0 over stdio or HTTP, enabling agents to call tools, read resources, and execute prompts.

## Why Rust?

| Metric | TypeScript SDK | Rust SDK |
|--------|---------------|----------|
| Binary size | ~50MB (node + deps) | ~3MB (static) |
| Startup time | ~500ms | ~2ms |
| Memory usage | ~50-100MB | ~3-5MB |
| Distribution | `npm install` + Node.js | Single binary download |
| Type safety | Runtime (Zod) | Compile-time (serde) |

## Quick Start

### 1. Add to your project

```toml
[dependencies]
mcp-sdk = { version = "0.1", features = ["stdio"] }
```

### 2. Build a server

```rust
use mcp_sdk::{McpServer, StdioTransport, ToolBuilder};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let server = Arc::new(McpServer::new("my-server", "0.1.0"));

    server.register_tool(
        ToolBuilder::new("greet")
            .description("Greet someone by name")
            .schema(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }))
            .handler(|args| async move {
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                Ok(mcp_sdk::CallToolResult::text(format!("Hello, {name}!")))
            })
    ).await;

    StdioTransport::serve(server).await.unwrap();
}
```

### 3. Connect to Claude Code

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "/path/to/your/binary"
    }
  }
}
```

### 4. Run the example

```bash
cargo run --example echo
```

## Architecture

```
┌──────────────────────────────────────────┐
│           McpServer (builder)             │
│  ┌─────────────┐  ┌──────────────────┐   │
│  │ ToolRegistry │  │  ServerInfo      │   │
│  │  (HashMap)   │  │  (name, version) │   │
│  └──────┬───────┘  └──────────────────┘   │
│         │                                 │
│  ┌──────▼───────┐                         │
│  │ handle_request │                       │
│  │ (JSON-RPC)    │                        │
│  └──────┬───────┘                         │
└─────────┼─────────────────────────────────┘
          │
   ┌──────▼───────┐
   │  Transport   │
   ├──────────────┤
   │ stdio (stdin)│  ← default, for CLI agents
   │ HTTP+SSE     │  ← feature-gated, for web agents
   └──────────────┘
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `stdio` | ✅ | stdin/stdout transport for CLI agents (Claude Code, Cursor) |
| `http` | ❌ | HTTP transport for web agents (Windsurf, custom integrations) |

```toml
# HTTP support
mcp-sdk = { version = "0.1", features = ["stdio", "http"] }

# Minimal (stdio only)
mcp-sdk = "0.1"
```

## API Reference

### McpServer

```rust
let server = McpServer::new("name", "version");
```

Methods:
- `register_tool(ToolBuilder)` — register a tool (async)
- `handle_request(&JsonRpcRequest)` — process a JSON-RPC request (async)
- `server_info()` — get server name/version

### ToolBuilder

```rust
ToolBuilder::new("tool-name")
    .description("What this tool does")
    .schema(json!({ "type": "object", "properties": { ... } }))
    .handler(|args: Value| async move {
        Ok(CallToolResult::text("result"))
    })
```

### CallToolResult

```rust
// Success
CallToolResult::text("output string")

// Error
CallToolResult::error("something went wrong")

// Custom content
CallToolResult {
    content: vec![Content::text("line 1"), Content::text("line 2")],
    is_error: false,
}
```

### Transports

```rust
// Stdio (default)
StdioTransport::serve(Arc::new(server)).await?;

// HTTP
HttpTransport::serve(Arc::new(server), "127.0.0.1:3000").await?;
```

## MCP Protocol Compliance

### Implemented Methods

| Method | Status | Description |
|--------|--------|-------------|
| `initialize` | ✅ | Server handshake + capabilities exchange |
| `notifications/initialized` | ✅ | Client init notification |
| `tools/list` | ✅ | List all registered tools |
| `tools/call` | ✅ | Execute a tool by name |
| `ping` | ✅ | Health check |
| `resources/list` | 📌 Planned | List available resources |
| `resources/read` | 📌 Planned | Read a resource by URI |
| `prompts/list` | 📌 Planned | List available prompts |
| `prompts/get` | 📌 Planned | Execute a prompt by name |

### Protocol Version

Currently implements `2024-11-05`. The `protocol_version` method on `McpServer` can be used to negotiate versions.

## Error Handling

```rust
use mcp_sdk::{McpError, McpResult};

handler(|args| async move {
    let name = args.get("name")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("missing 'name' field".into()))?;

    Ok(CallToolResult::text(format!("Hello, {name}")))
})
```

Errors are automatically converted to JSON-RPC error responses with appropriate codes:

| Error | Code | When |
|-------|------|------|
| `Parse` | -32700 | Invalid JSON |
| `MethodNotFound` | -32601 | Unknown method |
| `InvalidParams` | -32602 | Bad parameters |
| `Internal` | -32603 | Server error |
| `ToolNotFound` | -32601 | Unknown tool name |
| `Transport` | -32000 | I/O error |

## Testing

```bash
# Run tests
cargo test

# Run example
cargo run --example echo

# Type check
cargo check --all-features

# Format
cargo fmt

# Lint
cargo clippy --all-features
```

## Use Cases

- **Tool servers**: Expose CLI tools, database queries, or API calls to AI agents
- **Sandbox execution**: Like [agentic-armor](https://github.com/calebrosario/agentic-armor) — manage Docker containers for untrusted code
- **Data access**: Give agents read-only access to databases, files, or APIs
- **Custom integrations**: Bridge any system to any MCP-compatible AI agent

## Comparison with TypeScript SDK

| Feature | TypeScript (`@modelcontextprotocol/sdk`) | Rust (`mcp-sdk`) |
|---------|------------------------------------------|-------------------|
| Binary | Node.js + npm deps | Single static binary |
| Schema validation | Zod (runtime) | serde + JSON Schema (compile-time) |
| Transport | stdio + HTTP | stdio + HTTP (feature-gated) |
| Async model | Event loop (single-threaded) | Tokio (multi-threaded) |
| Error handling | try/catch + custom errors | `Result<T, McpError>` |
| Tool registration | `server.registerTool()` | `server.register_tool(ToolBuilder)` |
| Startup | ~500ms | ~2ms |
| Memory | ~50-100MB | ~3-5MB |

## License

MIT — see [LICENSE](LICENSE).
