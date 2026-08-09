# Contributing to rust-mcp-sdk

Thank you for your interest in contributing! This project aims to be a fast, type-safe Rust implementation of the [Model Context Protocol](https://modelcontextprotocol.io/).

## Getting Started

### Prerequisites

- **Rust 1.75+** (MSRV)
- `cargo`, `rustfmt`, `clippy` (included with rustup)

### Build & Test

```bash
# Clone
git clone https://github.com/calebrosario/rust-mcp-sdk.git
cd rust-mcp-sdk

# Run all tests
cargo test

# Run with all features
cargo test --all-features

# Run the example server
cargo run --example echo
```

### Code Quality Checks

Before submitting a PR, ensure all of these pass:

```bash
# Formatting
cargo fmt --all

# Linting (must be warning-free)
cargo clippy --all-features -- -D warnings

# Tests
cargo test --all-features
```

CI runs all of these on every push and PR. PRs will be blocked if any check fails.

## Architecture Overview

```
src/
├── lib.rs          — Public API re-exports
├── protocol.rs     — JSON-RPC 2.0 + MCP protocol types
├── server.rs       — McpServer (request dispatch, capability advertisement)
├── tool.rs         — ToolBuilder + ToolRegistry (async handler dispatch)
├── error.rs        — McpError enum with JSON-RPC code mapping
└── transport/
    ├── mod.rs      — Feature-gated transport exports
    ├── stdio.rs    — stdin/stdout newline-delimited JSON
    └── http.rs     — POST /mcp via axum (feature = "http")
```

### Key Design Decisions

1. **Feature flags** — `stdio` (default), `http` (opt-in). Keeps binary small.
2. **`serde` for all serialization** — no manual JSON.
3. **`tokio` for async** — handlers are `async` via boxed futures.
4. **`thiserror` for errors** — `McpResult<T> = Result<T, McpError>`.
5. **`tracing` for logging** — never `println!` or `eprintln!`.

### Tool Handler Pattern

```rust
ToolBuilder::new("name")
    .description("...")
    .schema(json!({ "type": "object", "properties": { ... } }))
    .handler(|args: Value| async move {
        Ok(CallToolResult::text("result"))
    })
```

Handlers are boxed as `Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = McpResult<CallToolResult>> + Send>> + Send + Sync>`.

## How to Contribute

### Reporting Bugs

1. Check existing issues first.
2. Open a new issue with:
   - Rust version (`rustc --version`)
   - Crate version
   - Minimal reproduction (code snippet)
   - Expected vs actual behavior

### Suggesting Features

1. Check the [protocol compliance table](README.md#mcp-protocol-compliance) — is it already planned?
2. Open an issue describing the use case and proposed API.

### Pull Requests

1. **Fork** the repo and create a branch: `git checkout -b feat/my-feature`
2. **Write tests** for new functionality.
3. **Run all quality checks** (see above).
4. **Keep changes focused** — one feature/fix per PR.
5. **Write clear commit messages** — follow [Conventional Commits](https://www.conventionalcommits.org/).

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`

Examples:
- `feat(resources): add resources/list handler`
- `fix(transport): handle empty stdin lines`
- `docs: update protocol compliance table`
- `test(http): add integration tests for HttpTransport`

## Protocol Compliance Roadmap

| Method | Status |
|--------|--------|
| `initialize` | ✅ |
| `notifications/initialized` | ✅ |
| `tools/list` | ✅ |
| `tools/call` | ✅ |
| `ping` | ✅ |
| `resources/list` | 🚧 In progress |
| `resources/read` | 🚧 In progress |
| `prompts/list` | 🚧 In progress |
| `prompts/get` | 🚧 In progress |
| `completion/complete` | 📌 Planned |

See [issues](https://github.com/calebrosario/rust-mcp-sdk/issues) for the full roadmap.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
