use crate::error::{McpError, McpResult};
use crate::protocol::*;
use crate::server::McpServer;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport;

const MAX_LINE_SIZE: usize = 10 * 1024 * 1024;

async fn read_bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    max_size: usize,
) -> std::io::Result<usize> {
    buf.clear();
    let mut total = 0usize;

    loop {
        let (len, newline_pos) = {
            let chunk = reader.fill_buf().await?;
            if chunk.is_empty() {
                return Ok(total);
            }
            (chunk.len(), chunk.iter().position(|&b| b == b'\n'))
        };

        if let Some(nl) = newline_pos {
            if total + nl > max_size {
                reader.consume(nl + 1);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Line exceeds max size",
                ));
            }
            {
                let chunk = reader.fill_buf().await?;
                buf.extend_from_slice(&chunk[..nl]);
            }
            reader.consume(nl + 1);
            return Ok(total + nl + 1);
        }

        if total + len > max_size {
            reader.consume(len);
            loop {
                let (dlen, dnl) = {
                    let d = reader.fill_buf().await?;
                    if d.is_empty() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Unterminated oversized line",
                        ));
                    }
                    (d.len(), d.iter().position(|&b| b == b'\n'))
                };
                match dnl {
                    Some(pos) => {
                        reader.consume(pos + 1);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Line exceeds max size",
                        ));
                    }
                    None => reader.consume(dlen),
                }
            }
        }

        {
            let chunk = reader.fill_buf().await?;
            buf.extend_from_slice(chunk);
        }
        reader.consume(len);
        total += len;
    }
}

async fn write_line<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, data: &str) -> McpResult<()> {
    writer
        .write_all(data.as_bytes())
        .await
        .map_err(McpError::from)?;
    writer.write_all(b"\n").await.map_err(McpError::from)?;
    writer.flush().await.map_err(McpError::from)?;
    Ok(())
}

fn write_error_response(id: RequestId, code: i32, message: String) -> McpResult<String> {
    let response = JsonRpcResponse::error(id, code, message, None);
    Ok(serde_json::to_string(&response)?)
}

impl StdioTransport {
    pub async fn serve(server: Arc<McpServer>) -> McpResult<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();

        let mut reader = BufReader::new(stdin);
        let mut raw_buf: Vec<u8> = Vec::with_capacity(8192);

        tracing::info!("MCP server listening on stdio");

        loop {
            match read_bounded_line(&mut reader, &mut raw_buf, MAX_LINE_SIZE).await {
                Ok(0) => {
                    tracing::info!("stdin EOF — client disconnected");
                    break;
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    tracing::warn!(
                        "Line exceeded max size ({} bytes), rejecting",
                        MAX_LINE_SIZE
                    );
                    let serialized = write_error_response(
                        RequestId::null(),
                        -32700,
                        "Payload too large".into(),
                    )?;
                    write_line(&mut stdout, &serialized).await?;
                    continue;
                }
                Err(e) => return Err(McpError::from(e)),
            }

            let trimmed: &[u8] = match std::str::from_utf8(&raw_buf) {
                Ok(s) => s.trim().as_bytes(),
                Err(e) => {
                    tracing::warn!("Invalid UTF-8 in input: {}", e);
                    let serialized =
                        write_error_response(RequestId::null(), -32700, "Invalid UTF-8".into())?;
                    write_line(&mut stdout, &serialized).await?;
                    continue;
                }
            };
            if trimmed.is_empty() {
                continue;
            }

            let message: Value = match serde_json::from_slice(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to parse JSON: {}", e);
                    let serialized = write_error_response(
                        RequestId::null(),
                        -32700,
                        format!("Parse error: {e}"),
                    )?;
                    write_line(&mut stdout, &serialized).await?;
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
                        write_line(&mut stdout, &serialized).await?;
                    }
                    Err(e) => {
                        let serialized = write_error_response(request.id, e.code(), e.to_string())?;
                        write_line(&mut stdout, &serialized).await?;
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
                server.handle_notification(&notification).await;
            }
        }

        tracing::info!("MCP server stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn test_bounded_line_rejects_oversized() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        let oversized = "x".repeat(100) + "\n";
        tx.write_all(oversized.as_bytes()).await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let result = read_bounded_line(&mut reader, &mut buf, 10).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        assert!(buf.len() <= 10);
    }

    #[tokio::test]
    async fn test_bounded_line_accepts_within_limit() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        let data = "{\"id\":1,\"method\":\"ping\"}\n";
        tx.write_all(data.as_bytes()).await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut reader, &mut buf, 1024)
            .await
            .unwrap();

        assert!(n > 0);
        assert!(!buf.is_empty());
        let parsed: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed["method"], "ping");
    }

    #[tokio::test]
    async fn test_bounded_line_eof_returns_zero() {
        let (_tx, rx) = tokio::io::duplex(1024);
        drop(_tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut reader, &mut buf, 1024)
            .await
            .unwrap();

        assert_eq!(n, 0);
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_bounded_line_rejects_oversized_no_newline() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        tx.write_all(&vec![b'x'; 200]).await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let result = read_bounded_line(&mut reader, &mut buf, 10).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bounded_line_continues_after_oversized() {
        let (mut tx, rx) = tokio::io::duplex(1024);

        tx.write_all(b"xxxxxxxxxxxxxxxxxx\n").await.unwrap();
        tx.write_all(b"{\"id\":1,\"method\":\"ping\"}\n")
            .await
            .unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();

        let result = read_bounded_line(&mut reader, &mut buf, 10).await;
        assert!(result.is_err());

        let n = read_bounded_line(&mut reader, &mut buf, 1024)
            .await
            .unwrap();
        assert!(n > 0);
        let parsed: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed["method"], "ping");
    }

    #[tokio::test]
    async fn test_bounded_line_exact_limit() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        let data = "12345\n";
        tx.write_all(data.as_bytes()).await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut reader, &mut buf, 5).await.unwrap();

        assert_eq!(n, 6);
        assert_eq!(&buf[..], b"12345");
    }

    #[tokio::test]
    async fn test_bounded_line_empty_line() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        tx.write_all(b"\n").await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut reader, &mut buf, 1024)
            .await
            .unwrap();

        assert_eq!(n, 1);
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_bounded_line_multibyte_utf8() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        let data = "{\"method\":\"你好\"}\n";
        tx.write_all(data.as_bytes()).await.unwrap();
        tx.flush().await.unwrap();
        drop(tx);

        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut reader, &mut buf, 1024)
            .await
            .unwrap();

        assert!(n > 0);
        let parsed: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed["method"], "你好");
    }
}
