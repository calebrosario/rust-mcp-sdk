use crate::error::{McpError, McpResult};
use crate::protocol::*;
use crate::tool::{ToolBuilder, ToolRegistry};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct McpServer {
    server_info: ServerInfo,
    protocol_version: ProtocolVersion,
    registry: Arc<Mutex<ToolRegistry>>,
}

impl McpServer {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, version: S2) -> Self {
        McpServer {
            server_info: ServerInfo {
                name: name.into(),
                version: version.into(),
            },
            protocol_version: ProtocolVersion::Latest,
            registry: Arc::new(Mutex::new(ToolRegistry::new())),
        }
    }

    pub fn protocol_version(mut self, version: ProtocolVersion) -> Self {
        self.protocol_version = version;
        self
    }

    pub async fn register_tool(&self, builder: ToolBuilder) {
        let (name, tool, handler) = builder.build();
        self.registry.lock().await.register(name, tool, handler);
    }

    pub async fn handle_request(&self, request: &JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params.as_ref()).await?,
            "tools/list" => self.handle_list_tools().await?,
            "tools/call" => self.handle_call_tool(request.params.as_ref()).await?,
            "ping" => serde_json::json!({}),
            _ => {
                return Err(McpError::MethodNotFound(request.method.clone()));
            }
        };

        Ok(JsonRpcResponse::success(request.id.clone(), result))
    }

    pub fn handle_notification(&self, notification: &JsonRpcNotification) {
        tracing::debug!("Received notification: {}", notification.method);
    }

    async fn handle_initialize(&self, params: Option<&Value>) -> McpResult<Value> {
        tracing::info!("Client initializing: {:?}", params);

        let result = InitializeResult {
            protocol_version: self.protocol_version.as_str().to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
                ..Default::default()
            },
            server_info: self.server_info.clone(),
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_tools(&self) -> McpResult<Value> {
        let tools = self.registry.lock().await.list();
        Ok(serde_json::to_value(ListToolsResult { tools })?)
    }

    async fn handle_call_tool(&self, params: Option<&Value>) -> McpResult<Value> {
        let raw = params.ok_or_else(|| McpError::InvalidParams("missing params".into()))?;
        let call_params: CallToolParams = serde_json::from_value(raw.clone())?;

        tracing::info!(
            "Tool called: {} with args: {}",
            call_params.name,
            call_params.arguments
        );

        let result: CallToolResult = self.registry.lock().await.call(call_params).await?;
        let value: Value = serde_json::to_value(result)?;
        Ok(value)
    }

    pub fn server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    pub fn registry(&self) -> &Arc<Mutex<ToolRegistry>> {
        &self.registry
    }
}
