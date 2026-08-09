use crate::error::{McpError, McpResult};
use crate::prompt::{PromptBuilder, PromptRegistry};
use crate::protocol::*;
use crate::resource::{ResourceBuilder, ResourceRegistry};
use crate::tool::{ToolBuilder, ToolRegistry};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct McpServer {
    server_info: ServerInfo,
    protocol_version: ProtocolVersion,
    registry: Arc<Mutex<ToolRegistry>>,
    resource_registry: Arc<Mutex<ResourceRegistry>>,
    prompt_registry: Arc<Mutex<PromptRegistry>>,
    initialized: Arc<Mutex<bool>>,
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
            resource_registry: Arc::new(Mutex::new(ResourceRegistry::new())),
            prompt_registry: Arc::new(Mutex::new(PromptRegistry::new())),
            initialized: Arc::new(Mutex::new(false)),
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

    pub async fn register_resource(&self, builder: ResourceBuilder) {
        let (uri, resource, handler) = builder.build();
        self.resource_registry
            .lock()
            .await
            .register(uri, resource, handler);
    }

    pub async fn register_prompt(&self, builder: PromptBuilder) {
        let (name, prompt, handler) = builder.build();
        self.prompt_registry
            .lock()
            .await
            .register(name, prompt, handler);
    }

    pub async fn handle_request(&self, request: &JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let method = request.method.as_str();

        if method != "initialize" && method != "ping" && !self.is_initialized().await {
            return Err(McpError::InvalidParams(
                "Server not initialized — send 'initialize' first".into(),
            ));
        }

        let result = match method {
            "initialize" => self.handle_initialize(request.params.as_ref()).await?,
            "tools/list" => self.handle_list_tools().await?,
            "tools/call" => self.handle_call_tool(request.params.as_ref()).await?,
            "resources/list" => self.handle_list_resources().await?,
            "resources/read" => self.handle_read_resource(request.params.as_ref()).await?,
            "prompts/list" => self.handle_list_prompts().await?,
            "prompts/get" => self.handle_get_prompt(request.params.as_ref()).await?,
            "ping" => serde_json::json!({}),
            _ => {
                return Err(McpError::MethodNotFound(request.method.clone()));
            }
        };

        Ok(JsonRpcResponse::success(request.id.clone(), result))
    }

    pub async fn handle_notification(&self, notification: &JsonRpcNotification) {
        tracing::debug!("Received notification: {}", notification.method);
        if notification.method == "notifications/initialized" {
            let mut initialized = self.initialized.lock().await;
            *initialized = true;
            tracing::info!("Client initialized handshake complete");
        }
    }

    pub async fn is_initialized(&self) -> bool {
        *self.initialized.lock().await
    }

    async fn handle_initialize(&self, params: Option<&Value>) -> McpResult<Value> {
        tracing::info!("Client initializing: {:?}", params);

        let result = InitializeResult {
            protocol_version: self.protocol_version.as_str().to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                prompts: Some(PromptsCapability::default()),
                ..Default::default()
            },
            server_info: self.server_info.clone(),
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_tools(&self) -> McpResult<Value> {
        let tools = self.registry.lock().await.list();
        Ok(serde_json::to_value(ListToolsResult {
            tools,
            next_cursor: None,
        })?)
    }

    async fn handle_call_tool(&self, params: Option<&Value>) -> McpResult<Value> {
        let raw = params.ok_or_else(|| McpError::InvalidParams("missing params".into()))?;
        let call_params: CallToolParams = serde_json::from_value(raw.clone())?;

        tracing::info!("Tool called: {}", call_params.name);

        let handler = {
            let registry = self.registry.lock().await;
            registry.get_handler(&call_params.name)
        };

        let result: CallToolResult = match handler {
            Some(h) => match tokio::spawn(async move { h(call_params.arguments).await }).await {
                Ok(r) => r?,
                Err(join_err) => {
                    tracing::error!("Tool handler panicked: {}", join_err);
                    return Err(McpError::Internal("Handler execution failed".into()));
                }
            },
            None => return Err(McpError::ToolNotFound(call_params.name)),
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_resources(&self) -> McpResult<Value> {
        let resources = self.resource_registry.lock().await.list();
        Ok(serde_json::to_value(ListResourcesResult {
            resources,
            next_cursor: None,
        })?)
    }

    async fn handle_read_resource(&self, params: Option<&Value>) -> McpResult<Value> {
        let raw = params.ok_or_else(|| McpError::InvalidParams("missing params".into()))?;
        let read_params: ReadResourceParams = serde_json::from_value(raw.clone())?;

        tracing::info!("Resource read: {}", read_params.uri);

        let handler = {
            let registry = self.resource_registry.lock().await;
            registry.get_handler(&read_params.uri)
        };

        let contents = match handler {
            Some(h) => {
                let uri = read_params.uri;
                match tokio::spawn(async move { h(&uri).await }).await {
                    Ok(c) => c?,
                    Err(join_err) => {
                        tracing::error!("Resource handler panicked: {}", join_err);
                        return Err(McpError::Internal("Handler execution failed".into()));
                    }
                }
            }
            None => return Err(McpError::ResourceNotFound(read_params.uri)),
        };

        let result = ReadResourceResult { contents };
        Ok(serde_json::to_value(result)?)
    }

    async fn handle_list_prompts(&self) -> McpResult<Value> {
        let prompts = self.prompt_registry.lock().await.list();
        Ok(serde_json::to_value(ListPromptsResult {
            prompts,
            next_cursor: None,
        })?)
    }

    async fn handle_get_prompt(&self, params: Option<&Value>) -> McpResult<Value> {
        let raw = params.ok_or_else(|| McpError::InvalidParams("missing params".into()))?;
        let get_params: GetPromptParams = serde_json::from_value(raw.clone())?;

        tracing::info!("Prompt requested: {}", get_params.name);

        let handler = {
            let registry = self.prompt_registry.lock().await;
            registry.get_handler(&get_params.name)
        };

        let arguments = get_params.arguments.unwrap_or(Value::Null);
        let result = match handler {
            Some(h) => match tokio::spawn(async move { h(arguments).await }).await {
                Ok(r) => r?,
                Err(join_err) => {
                    tracing::error!("Prompt handler panicked: {}", join_err);
                    return Err(McpError::Internal("Handler execution failed".into()));
                }
            },
            None => return Err(McpError::PromptNotFound(get_params.name)),
        };

        Ok(serde_json::to_value(result)?)
    }

    pub fn server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    pub fn registry(&self) -> &Arc<Mutex<ToolRegistry>> {
        &self.registry
    }
}
