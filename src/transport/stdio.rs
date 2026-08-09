use crate::error::{McpError, McpResult};
use crate::protocol::*;
use crate::server::McpServer;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport;

impl StdioTransport {
    pub async fn serve(server: Arc<McpServer>) -> McpResult<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();

        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        tracing::info!("MCP server listening on stdio");

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await.map_err(McpError::from)?;

            if bytes_read == 0 {
                tracing::info!("stdin EOF — client disconnected");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let message: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to parse JSON: {} — raw: {}", e, trimmed);
                    let error_response = JsonRpcResponse::error(
                        RequestId::null(),
                        -32700,
                        format!("Parse error: {e}"),
                        None,
                    );
                    let serialized = serde_json::to_string(&error_response)?;
                    stdout
                        .write_all(serialized.as_bytes())
                        .await
                        .map_err(McpError::from)?;
                    stdout.write_all(b"\n").await.map_err(McpError::from)?;
                    stdout.flush().await.map_err(McpError::from)?;
                    continue;
                }
            };

            if message.get("id").is_some() {
                let request: JsonRpcRequest = match serde_json::from_value(message) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("Invalid request: {}", e);
                        continue;
                    }
                };

                tracing::debug!("Request: {} (id={:?})", request.method, request.id);

                match server.handle_request(&request).await {
                    Ok(response) => {
                        let serialized = serde_json::to_string(&response)?;
                        stdout
                            .write_all(serialized.as_bytes())
                            .await
                            .map_err(McpError::from)?;
                        stdout.write_all(b"\n").await.map_err(McpError::from)?;
                        stdout.flush().await.map_err(McpError::from)?;
                    }
                    Err(e) => {
                        let response = JsonRpcResponse::error(
                            request.id,
                            e.code(),
                            e.to_string(),
                            None,
                        );
                        let serialized = serde_json::to_string(&response)?;
                        stdout
                            .write_all(serialized.as_bytes())
                            .await
                            .map_err(McpError::from)?;
                        stdout.write_all(b"\n").await.map_err(McpError::from)?;
                        stdout.flush().await.map_err(McpError::from)?;
                    }
                }
            } else {
                let notification: JsonRpcNotification = match serde_json::from_value(message) {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::warn!("Invalid notification: {}", e);
                        continue;
                    }
                };

                tracing::debug!("Notification: {}", notification.method);
                server.handle_notification(&notification);
            }
        }

        tracing::info!("MCP server stopped");
        Ok(())
    }
}
