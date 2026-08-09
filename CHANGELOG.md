# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-09

### Added
- Resources support (`resources/list`, `resources/read`) with `ResourceBuilder` + `ResourceRegistry`
- Prompts support (`prompts/list`, `prompts/get`) with `PromptBuilder` + `PromptRegistry`
- Typed `ResourcesCapability`, `PromptsCapability`, `LoggingCapability`
- `listChanged` flag on `ToolsCapability`
- `ResourceLink` content type in `Content` enum
- Pagination support (`nextCursor`) on all list result types
- `notifications/initialized` state tracking (`is_initialized()`)
- New error variants: `ResourceNotFound` (-32002), `PromptNotFound` (-32002)
- `PromptArgumentBuilder` for configuring prompt arguments
- Demo example server (`cargo run --example demo`) with tools + resources + prompts
- CI workflow (GitHub Actions) — test + clippy + fmt on push/PR
- `CONTRIBUTING.md` with build/test/lint instructions
- `CHANGELOG.md`
- `rust-toolchain.toml` (MSRV 1.75)
- `rustfmt.toml`

### Changed
- `ServerCapabilities` fields are now properly typed (was `Option<Value>`)
- `ProtocolVersion` now uses `#[derive(Default)]` instead of manual impl
- `.gitignore` updated to exclude project artifacts
- README badges updated, Resources/Prompts API docs added
- `handle_notification` is now `async` for state tracking

### Removed
- Orphaned `examples/Cargo.toml`

## [0.1.0] - 2026-08-09

### Added
- Initial release
- MCP protocol version `2024-11-05`
- JSON-RPC 2.0 types (`JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`)
- `McpServer` with tool registration and request dispatch
- `ToolBuilder` + `ToolRegistry` for async tool handlers
- `StdioTransport` (default feature)
- `HttpTransport` (feature-gated via `http`)
- 45 tests (protocol, server, tool, error)
- Echo server example
