# MCP Protocol Implementation

This document describes how `rust-mcp-sdk` implements the Model Context Protocol.

## Protocol Version

Currently implements `2024-11-05` of the MCP specification.

## Message Format

All communication uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) over newline-delimited JSON (for stdio) or HTTP POST (for HTTP transport).

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": { "name": "echo", "arguments": { "message": "hello" } }
}
```

### Response (success)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "hello" }],
    "isError": false
  }
}
```

### Response (error)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": { "code": -32601, "message": "Tool not found: unknown" }
}
```

### Notification (no response)

```json
{ "jsonrpc": "2.0", "method": "notifications/initialized" }
```

## Lifecycle

```
Client                          Server
  │                               │
  │── initialize ────────────────▶│
  │◀── capabilities + serverInfo ─│
  │                               │
  │── initialized (notification) ▶│
  │                               │
  │── tools/list ────────────────▶│
  │◀── [tool definitions] ────────│
  │                               │
  │── tools/call ────────────────▶│
  │◀── result ────────────────────│
  │                               │
  │── ping ──────────────────────▶│
  │◀── {} ────────────────────────│
```

## Methods

### initialize

Request params:
```json
{
  "protocolVersion": "2024-11-05",
  "capabilities": {},
  "clientInfo": { "name": "claude-code", "version": "1.0" }
}
```

Response:
```json
{
  "protocolVersion": "2024-11-05",
  "capabilities": { "tools": {} },
  "serverInfo": { "name": "my-server", "version": "0.1.0" }
}
```

### tools/list

No params. Returns all registered tools.

Response:
```json
{
  "tools": [
    {
      "name": "echo",
      "description": "Echo back the provided message",
      "inputSchema": {
        "type": "object",
        "properties": { "message": { "type": "string" } },
        "required": ["message"]
      }
    }
  ]
}
```

### tools/call

Params:
```json
{ "name": "echo", "arguments": { "message": "hello" } }
```

Response:
```json
{
  "content": [{ "type": "text", "text": "hello" }],
  "isError": false
}
```

### ping

No params. Returns empty object `{}`. Used for health checks.

## Error Codes

| Code | Meaning | Rust Enum |
|------|---------|-----------|
| -32700 | Parse error | `McpError::Parse` |
| -32601 | Method not found | `McpError::MethodNotFound` |
| -32602 | Invalid params | `McpError::InvalidParams` |
| -32603 | Internal error | `McpError::Internal` |
| -32000 | Transport error | `McpError::Transport` |

## Transport: stdio

- Client writes JSON-RPC messages to server's stdin, one per line (newline-delimited)
- Server writes responses to stdout, one per line
- stderr is used for logging (tracing)
- EOF on stdin signals client disconnect

## Transport: HTTP (feature-gated)

- POST `/mcp` with JSON-RPC body
- Response is JSON-RPC response
- Notifications return `{ "status": "ok" }`
