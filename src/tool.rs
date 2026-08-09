use crate::error::McpResult;
use crate::protocol::{CallToolParams, CallToolResult};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ToolHandler = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = McpResult<CallToolResult>> + Send>> + Send + Sync,
>;

pub struct ToolBuilder {
    name: String,
    description: Option<String>,
    input_schema: Value,
    handler: Option<ToolHandler>,
}

impl ToolBuilder {
    pub fn new<S: Into<String>>(name: S) -> Self {
        ToolBuilder {
            name: name.into(),
            description: None,
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            handler: None,
        }
    }

    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = McpResult<CallToolResult>> + Send + 'static,
    {
        self.handler = Some(Arc::new(move |args| Box::pin(f(args))));
        self
    }

    pub fn build(self) -> (String, crate::protocol::Tool, ToolHandler) {
        let tool = crate::protocol::Tool {
            name: self.name.clone(),
            description: self.description,
            input_schema: self.input_schema,
        };
        let handler = self.handler.unwrap_or_else(|| {
            Arc::new(|_| Box::pin(async { Ok(CallToolResult::error("No handler registered")) }))
        });
        (self.name, tool, handler)
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, (crate::protocol::Tool, ToolHandler)>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, tool: crate::protocol::Tool, handler: ToolHandler) {
        self.tools.insert(name, (tool, handler));
    }

    pub fn list(&self) -> Vec<crate::protocol::Tool> {
        self.tools
            .values()
            .map(|(t, _): &(crate::protocol::Tool, ToolHandler)| t.clone())
            .collect()
    }

    pub fn get_handler(&self, name: &str) -> Option<ToolHandler> {
        self.tools.get(name).map(|(_, h)| h.clone())
    }

    pub async fn call(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        match self.tools.get(&params.name) {
            Some((_, handler)) => {
                let handler = handler.clone();
                let args = params.arguments;
                match tokio::spawn(async move { handler(args).await }).await {
                    Ok(result) => result,
                    Err(join_err) => {
                        tracing::error!("Tool handler panicked: {}", join_err);
                        Err(crate::error::McpError::Internal(format!(
                            "Handler panicked: {}",
                            join_err
                        )))
                    }
                }
            }
            None => Err(crate::error::McpError::ToolNotFound(params.name)),
        }
    }
}
