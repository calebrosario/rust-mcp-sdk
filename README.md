# rust-mcp-sdk

> Rust implementation of the Model Context Protocol (MCP) — build MCP servers with type safety, async performance, and a single static binary.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Protocol](https://img.shields.io/badge/MCP-2024--11--05-blue.svg)](https://modelcontextprotocol.io)

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

### Resources

```rust
use mcp_sdk::{ResourceBuilder, ResourceContents};

server.register_resource(
    ResourceBuilder::new("file:///config.json", "Config")
        .description("Server configuration")
        .mime_type("application/json")
        .handler(|uri| async move {
            Ok(vec![ResourceContents::Text {
                uri: uri.to_string(),
                mime_type: Some("application/json".into()),
                text: r#"{"version": "1.0"}"#.into(),
            }])
        })
).await;
```

### Prompts

```rust
use mcp_sdk::{PromptBuilder, PromptMessage};

server.register_prompt(
    PromptBuilder::new("code_review")
        .description("Review code")
        .argument("language")
        .argument_with("max_length", |a| a.description("Max words").required())
        .handler(|args| async move {
            let lang = args.get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("rust");
            Ok(mcp_sdk::GetPromptResult {
                description: Some("Code review".into()),
                messages: vec![PromptMessage {
                    role: "user".into(),
                    content: mcp_sdk::Content::text(format!("Review this {lang} code")),
                }],
            })
        })
).await;
```

## MCP Protocol Compliance

### Implemented Methods

| Method | Status | Description |
|--------|--------|-------------|
| `initialize` | ✅ | Server handshake + capabilities exchange |
| `notifications/initialized` | ✅ | Client init notification + state tracking |
| `tools/list` | ✅ | List all registered tools |
| `tools/call` | ✅ | Execute a tool by name |
| `ping` | ✅ | Health check |
| `resources/list` | ✅ | List available resources |
| `resources/read` | ✅ | Read a resource by URI |
| `prompts/list` | ✅ | List available prompts |
| `prompts/get` | ✅ | Execute a prompt by name |
| `completion/complete` | 📌 Planned | Autocomplete for arguments |
| `logging/setLevel` | 📌 Planned | Server log level control |

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
| `ResourceNotFound` | -32002 | Unknown resource URI |
| `PromptNotFound` | -32002 | Unknown prompt name |
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

## Security

### Built-in Protections

| Protection | Status |
|-----------|--------|
| Payload size limit (stdio) | ✅ 10MB max line |
| Argument logging level | ✅ DEBUG (not INFO) |
| Raw payload removed from error logs | ✅ |
| Handler error isolation | ✅ Errors return JSON-RPC error responses |
| Error messages don't leak registry contents | ✅ |
| No dependency CVEs | ✅ (cargo audit clean) |

### Security Considerations for SDK Users

When building an MCP server with this SDK, consider these security responsibilities:

1. **Resource URI validation** — The SDK passes URIs directly to your handlers. If your handler reads from the filesystem, validate URIs against path traversal (`../../../etc/passwd`).

2. **HTTP transport authentication** — The HTTP transport has no built-in auth. If exposing over a network, put it behind a reverse proxy with authentication (e.g., nginx + basic auth, Cloudflare Access).

3. **HTTP bind address** — Always bind to `127.0.0.1:port` unless you explicitly want remote access. Never bind to `0.0.0.0` without auth.

4. **Tool handler safety** — Tool handlers receive arbitrary JSON from the client. Validate all inputs. Never pass tool arguments directly to shell commands, SQL queries, or file operations without sanitization.

5. **Handler panics** — Handlers should return `Err(McpError::...)` instead of panicking. A panicking handler will abort the process. Use `Result` for all error paths.

6. **Rate limiting** — For HTTP transport, implement rate limiting at the reverse proxy level to prevent DoS.

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
