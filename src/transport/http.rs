use crate::error::McpResult;
use crate::server::McpServer;
use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde_json::Value;
use std::sync::Arc;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

pub struct HttpTransport;

impl HttpTransport {
    pub async fn serve(server: Arc<McpServer>, addr: &str) -> McpResult<()> {
        let app = Router::new()
            .route("/mcp", post(handle_mcp))
            .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE))
            .with_state(server);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::McpError::Transport(e.to_string()))?;

        tracing::info!("MCP server listening on HTTP at http://{addr}/mcp");

        axum::serve(listener, app)
            .await
            .map_err(|e| crate::error::McpError::Transport(e.to_string()))?;

        Ok(())
    }
}

async fn handle_mcp(
    State(server): State<Arc<McpServer>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if body.get("id").is_some() {
        let request: crate::protocol::JsonRpcRequest =
            serde_json::from_value(body).map_err(|_| StatusCode::BAD_REQUEST)?;

        match server.handle_request(&request).await {
            Ok(response) => Ok(Json(serde_json::to_value(response).unwrap_or_default())),
            Err(e) => {
                let error_response = crate::protocol::JsonRpcResponse::error(
                    request.id,
                    e.code(),
                    e.to_string(),
                    None,
                );
                Ok(Json(
                    serde_json::to_value(error_response).unwrap_or_default(),
                ))
            }
        }
    } else {
        let notification: crate::protocol::JsonRpcNotification =
            serde_json::from_value(body).map_err(|_| StatusCode::BAD_REQUEST)?;
        server.handle_notification(&notification).await;
        Ok(Json(serde_json::json!({"status": "ok"})))
    }
}
