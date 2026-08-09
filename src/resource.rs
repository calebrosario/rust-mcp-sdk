use crate::error::{McpError, McpResult};
use crate::protocol::{Resource, ResourceContents};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ResourceHandler = Arc<
    dyn Fn(&str) -> Pin<Box<dyn Future<Output = McpResult<Vec<ResourceContents>>> + Send + 'static>>
        + Send
        + Sync,
>;

pub struct ResourceBuilder {
    uri: String,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
    handler: Option<ResourceHandler>,
}

impl ResourceBuilder {
    pub fn new<S1: Into<String>, S2: Into<String>>(uri: S1, name: S2) -> Self {
        ResourceBuilder {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            handler: None,
        }
    }

    pub fn description<S: Into<String>>(mut self, desc: S) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn mime_type<S: Into<String>>(mut self, mime: S) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = McpResult<Vec<ResourceContents>>> + Send + 'static,
    {
        self.handler = Some(Arc::new(move |uri| Box::pin(f(uri))));
        self
    }

    pub fn build(self) -> (String, Resource, ResourceHandler) {
        let resource = Resource {
            uri: self.uri.clone(),
            name: self.name,
            description: self.description,
            mime_type: self.mime_type,
        };
        let handler = self.handler.unwrap_or_else(|| {
            Arc::new(|uri: &str| {
                let uri = uri.to_string();
                Box::pin(async move {
                    Ok(vec![ResourceContents::Text {
                        uri,
                        mime_type: None,
                        text: String::new(),
                    }])
                })
            })
        });
        (self.uri, resource, handler)
    }
}

#[derive(Default)]
pub struct ResourceRegistry {
    resources: HashMap<String, (Resource, ResourceHandler)>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        ResourceRegistry {
            resources: HashMap::new(),
        }
    }

    pub fn register(&mut self, uri: String, resource: Resource, handler: ResourceHandler) {
        self.resources.insert(uri, (resource, handler));
    }

    pub fn list(&self) -> Vec<Resource> {
        self.resources
            .values()
            .map(|(r, _): &(Resource, ResourceHandler)| r.clone())
            .collect()
    }

    pub fn get_handler(&self, uri: &str) -> Option<ResourceHandler> {
        self.resources.get(uri).map(|(_, h)| h.clone())
    }

    pub async fn read(&self, uri: &str) -> McpResult<Vec<ResourceContents>> {
        match self.resources.get(uri) {
            Some((_, handler)) => {
                let handler = handler.clone();
                let uri_owned = uri.to_string();
                match tokio::spawn(async move { handler(&uri_owned).await }).await {
                    Ok(result) => result,
                    Err(join_err) => {
                        tracing::error!("Resource handler panicked: {}", join_err);
                        Err(McpError::Internal(format!(
                            "Handler panicked: {}",
                            join_err
                        )))
                    }
                }
            }
            None => Err(McpError::ResourceNotFound(uri.to_string())),
        }
    }
}
