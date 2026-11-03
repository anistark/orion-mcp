//! One MCP tool, offered as an Orion [`Tool`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use orion_core::{CoreError, CoreResult, Tool, ToolContext, ToolOutput};
use rmcp::model::{CallToolRequestParams, ContentBlock, ResourceContents};
use serde_json::Value;

use crate::server::Connection;

/// How often a running call checks the run's stop flag.
const ABORT_POLL: Duration = Duration::from_millis(50);

pub(crate) struct McpTool {
    conn: Arc<Connection>,
    remote: String,
    name: String,
    label: String,
    description: String,
    schema: Value,
}

impl McpTool {
    pub(crate) fn new(conn: Arc<Connection>, tool: rmcp::model::Tool) -> Self {
        let remote = tool.name.into_owned();
        let label = tool
            .title
            .or_else(|| tool.annotations.and_then(|a| a.title))
            .unwrap_or_else(|| remote.clone());
        Self {
            name: conn.tool_name(&remote),
            conn,
            remote,
            label,
            description: tool.description.map(|d| d.into_owned()).unwrap_or_default(),
            schema: Value::Object((*tool.input_schema).clone()),
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> CoreResult<ToolOutput> {
        let params = match args {
            Value::Object(args) => {
                CallToolRequestParams::new(self.remote.clone()).with_arguments(args)
            }
            Value::Null => CallToolRequestParams::new(self.remote.clone()),
            other => {
                return Err(CoreError::Tool(format!(
                    "{}: arguments must be a JSON object, got {other}",
                    self.name
                )));
            }
        };

        let abort = ctx.abort_flag();
        let result = tokio::select! {
            biased;
            () = stopped(&abort) => return Err(CoreError::Aborted),
            result = self.conn.call(params) => result?,
        };

        let mut content = render(&result.content);
        if content.is_empty()
            && let Some(structured) = &result.structured_content
        {
            content = structured.to_string();
        }
        if result.is_error == Some(true) {
            return Err(CoreError::Tool(format!("{}: {content}", self.name)));
        }

        let output = ToolOutput::new(content);
        Ok(match result.structured_content {
            Some(structured) => output.with_details(structured),
            None => output,
        })
    }
}

/// Resolves once the run's stop flag is set.
async fn stopped(flag: &AtomicBool) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(ABORT_POLL).await;
    }
}

/// The text the model reads: text blocks as they are, other blocks as a short
/// note of what was returned.
fn render(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text(text) => text.text.clone(),
            ContentBlock::Image(image) => format!("[image: {}]", image.mime_type),
            ContentBlock::Audio(audio) => format!("[audio: {}]", audio.mime_type),
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { text, .. } => text.clone(),
                ResourceContents::BlobResourceContents { uri, .. } => format!("[resource: {uri}]"),
                _ => "[resource]".to_string(),
            },
            ContentBlock::ResourceLink(link) => format!("[resource link: {}]", link.uri),
            _ => "[unsupported content]".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
