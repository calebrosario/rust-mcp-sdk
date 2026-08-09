use crate::error::{McpError, McpResult};
use crate::protocol::{GetPromptResult, Prompt, PromptArgument};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type PromptHandler = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = McpResult<GetPromptResult>> + Send>> + Send + Sync,
>;

pub struct PromptBuilder {
    name: String,
    description: Option<String>,
    arguments: Vec<PromptArgument>,
    handler: Option<PromptHandler>,
}

impl PromptBuilder {
    pub fn new<S: Into<String>>(name: S) -> Self {
        PromptBuilder {
            name: name.into(),
            description: None,
            arguments: Vec::new(),
            handler: None,
        }
    }

    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn argument<S: Into<String>>(mut self, name: S) -> Self {
        self.arguments.push(PromptArgument {
            name: name.into(),
            description: None,
            required: false,
        });
        self
    }

    pub fn argument_with<F>(mut self, name: &str, configure: F) -> Self
    where
        F: FnOnce(PromptArgumentBuilder) -> PromptArgumentBuilder,
    {
        let builder = configure(PromptArgumentBuilder {
            name: name.to_string(),
            description: None,
            required: false,
        });
        self.arguments.push(builder.build());
        self
    }

    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = McpResult<GetPromptResult>> + Send + 'static,
    {
        self.handler = Some(Arc::new(move |args| Box::pin(f(args))));
        self
    }

    pub fn build(self) -> (String, Prompt, PromptHandler) {
        let prompt = Prompt {
            name: self.name.clone(),
            description: self.description,
            arguments: self.arguments,
        };
        let handler = self.handler.unwrap_or_else(|| {
            Arc::new(|_| {
                Box::pin(async {
                    Ok(GetPromptResult {
                        description: None,
                        messages: Vec::new(),
                    })
                })
            })
        });
        (self.name, prompt, handler)
    }
}

pub struct PromptArgumentBuilder {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

impl PromptArgumentBuilder {
    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn build(self) -> PromptArgument {
        PromptArgument {
            name: self.name,
            description: self.description,
            required: self.required,
        }
    }
}

#[derive(Default)]
pub struct PromptRegistry {
    prompts: HashMap<String, (Prompt, PromptHandler)>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        PromptRegistry {
            prompts: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, prompt: Prompt, handler: PromptHandler) {
        self.prompts.insert(name, (prompt, handler));
    }

    pub fn list(&self) -> Vec<Prompt> {
        self.prompts
            .values()
            .map(|(p, _): &(Prompt, PromptHandler)| p.clone())
            .collect()
    }

    pub fn get_handler(&self, name: &str) -> Option<PromptHandler> {
        self.prompts.get(name).map(|(_, h)| h.clone())
    }

    pub async fn get(&self, name: &str, arguments: Value) -> McpResult<GetPromptResult> {
        match self.prompts.get(name) {
            Some((_, handler)) => {
                let handler = handler.clone();
                match tokio::spawn(async move { handler(arguments).await }).await {
                    Ok(result) => result,
                    Err(join_err) => {
                        tracing::error!("Prompt handler panicked: {}", join_err);
                        Err(McpError::Internal(format!(
                            "Handler panicked: {}",
                            join_err
                        )))
                    }
                }
            }
            None => Err(McpError::PromptNotFound(name.to_string())),
        }
    }
}
