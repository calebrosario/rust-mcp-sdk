# PROJECT KNOWLEDGE BASE

**Generated:** 2026-08-09
**Repo:** rust-mcp-sdk
**Branch:** main

## OVERVIEW

Rust implementation of the Model Context Protocol (MCP) SDK. Enables building MCP servers in Rust with type safety, async performance, and single-binary distribution. Designed to replace the TypeScript `@modelcontextprotocol/sdk` in agentic-armor.

## STRUCTURE

```
rust-mcp-sdk/
├── Cargo.toml              # Package manifest (features: stdio, http)
├── src/
│   ├── lib.rs              # Public API re-exports
│   ├── protocol.rs         # JSON-RPC 2.0 types + MCP protocol types
│   ├── server.rs           # McpServer (tool registration, request dispatch)
│   ├── tool.rs             # ToolBuilder + ToolRegistry (async dispatch)
│   ├── error.rs            # McpError enum with JSON-RPC code mapping
│   └── transport/
│       ├── mod.rs          # Feature-gated transport re-exports
│       ├── stdio.rs        # stdin/stdout newline-delimited JSON
│       └── http.rs         # POST /mcp via axum (feature-gated)
├── tests/
│   ├── protocol_test.rs    # 20 tests: serialization, deserialization
│   ├── server_test.rs      # 12 tests: initialize, tools/list, tools/call
│   ├── tool_test.rs        # 9 tests: ToolRegistry, ToolBuilder
│   └── error_test.rs       # 4 tests: error codes, conversions
├── examples/
│   └── echo.rs             # Working echo+add server example
└── docs/
    └── PROTOCOL.md         # MCP protocol lifecycle documentation
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add a new MCP method | `src/server.rs` handle_request() | Match on method string, add handler |
| Add a new protocol type | `src/protocol.rs` | Derive Serialize+Deserialize |
| Add a transport | `src/transport/` | Implement serve() method |
| Register a tool | `examples/echo.rs` | Use ToolBuilder::new() pattern |
| Understand error codes | `src/error.rs` | McpError::code() maps to JSON-RPC |

## CONVENTIONS

**Rust 2021 Edition, MSRV 1.75**

- `serde` for all serialization (no manual JSON)
- `tokio` for async runtime
- Feature flags: `stdio` (default), `http` (opt-in via axum)
- `thiserror` for error derive
- `tracing` for logging (NOT println!)
- Tests use `#[tokio::test]` for async
- `McpResult<T> = Result<T, McpError>` throughout

**Tool Handler Pattern:**
```rust
ToolBuilder::new("name")
    .description("...")
    .schema(json!({...}))
    .handler(|args: Value| async move {
        Ok(CallToolResult::text("result"))
    })
```

**Handler signature:** `Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = McpResult<CallToolResult>> + Send>> + Send + Sync>`

## COMMANDS

```bash
cargo test                          # Run all 45 tests
cargo test --features stdio         # Run tests with stdio feature
cargo test --all-features           # Run tests with all features
cargo run --example echo            # Run echo server example
cargo check --all-features          # Type check everything
cargo clippy --all-features         # Lint
cargo fmt                           # Format
cargo build --release               # Production build (~3MB binary)
```

## MCP PROTOCOL COMPLIANCE

| Method | Status |
|--------|--------|
| `initialize` | ✅ |
| `notifications/initialized` | ✅ |
| `tools/list` | ✅ |
| `tools/call` | ✅ |
| `ping` | ✅ |
| `resources/list` | 📌 Planned |
| `resources/read` | 📌 Planned |
| `prompts/list` | 📌 Planned |
| `prompts/get` | 📌 Planned |

Protocol version: `2024-11-05`

## ANTI-PATTERNS

1. **Never use println!** — use `tracing::info!` / `tracing::debug!`
2. **Never panic in handlers** — return `Err(McpError::...)`
3. **Never block in async** — use `tokio::spawn` for CPU-heavy work
4. **Don't add dependencies without feature flags** — keep binary small
5. **Don't use `unwrap()` in production code** — use `?` or proper error handling

## NOTES

- Binary size: ~3MB release build (vs ~50MB Node.js)
- Startup: ~2ms (vs ~500ms Node.js)
- Memory: ~3-5MB (vs ~50-100MB Node.js)
- Designed as a dependency for agentic-armor Rust rewrite
- GitHub: https://github.com/calebrosario/rust-mcp-sdk
