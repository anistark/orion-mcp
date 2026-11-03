//! Connecting to an MCP server and listing its tools.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use http::{HeaderName, HeaderValue};
use orion_core::{CoreError, CoreResult, Tool};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, Implementation,
};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{RoleClient, ServiceExt};

use crate::tool::McpTool;

/// What joins a server's prefix to each of its tool names: `docs` and `search`
/// become `docs__search`.
pub const PREFIX_SEPARATOR: &str = "__";

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Settings for connecting to one MCP server, made by [`McpServer::builder`].
///
/// Header values and the bearer token are sent with every request and kept only
/// as long as the connection. They never appear in errors or in `Debug` output.
pub struct McpServerBuilder {
    url: String,
    prefix: Option<String>,
    bearer_token: Option<String>,
    headers: Vec<(String, String)>,
    connect_timeout: Duration,
    call_timeout: Duration,
}

impl McpServerBuilder {
    /// Offer the server's tools as `{prefix}__{name}`, so two servers, or a
    /// server and the host's own tools, can share a tool name.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Send `Authorization: Bearer {token}` with every request.
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    /// Send a header with every request, such as an API key.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// How long connecting and listing tools may take (default 10 seconds).
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// How long one tool call may take (default 60 seconds).
    pub fn call_timeout(mut self, timeout: Duration) -> Self {
        self.call_timeout = timeout;
        self
    }

    /// Connect and run the MCP handshake.
    ///
    /// Fails with [`CoreError::Tool`] when a header is invalid, or when the
    /// server cannot be reached, refuses the handshake, or does not answer
    /// within the connect timeout.
    pub async fn connect(self) -> CoreResult<McpServer> {
        let label = label(&self.url);
        let fail = |msg: String| CoreError::Tool(format!("MCP server {label}: {msg}"));

        let mut headers = HashMap::with_capacity(self.headers.len());
        for (name, value) in &self.headers {
            let name = HeaderName::try_from(name.as_str())
                .map_err(|_| fail(format!("invalid header name '{name}'")))?;
            let mut value = HeaderValue::try_from(value.as_str())
                .map_err(|_| fail(format!("invalid value for header '{name}'")))?;
            value.set_sensitive(true);
            headers.insert(name, value);
        }

        let mut config = StreamableHttpClientTransportConfig::with_uri(self.url.as_str())
            .custom_headers(headers);
        if let Some(token) = self.bearer_token {
            config = config.auth_header(token);
        }
        let transport = StreamableHttpClientTransport::from_config(config);
        let info = ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        );

        let service = tokio::time::timeout(self.connect_timeout, info.serve(transport))
            .await
            .map_err(|_| fail(format!("no answer within {:?}", self.connect_timeout)))?
            .map_err(|e| fail(format!("could not connect: {e}")))?;

        Ok(McpServer {
            inner: Arc::new(Connection {
                label,
                prefix: self.prefix,
                service,
                connect_timeout: self.connect_timeout,
                call_timeout: self.call_timeout,
            }),
        })
    }
}

impl fmt::Debug for McpServerBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headers: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("McpServerBuilder")
            .field("url", &label(&self.url))
            .field("prefix", &self.prefix)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .field("headers", &headers)
            .field("connect_timeout", &self.connect_timeout)
            .field("call_timeout", &self.call_timeout)
            .finish()
    }
}

/// A connected MCP server.
///
/// Cheap to clone. The connection closes once the server and every tool taken
/// from it are dropped.
#[derive(Clone)]
pub struct McpServer {
    inner: Arc<Connection>,
}

impl McpServer {
    /// Start configuring a connection to the Streamable HTTP endpoint at `url`.
    pub fn builder(url: impl Into<String>) -> McpServerBuilder {
        McpServerBuilder {
            url: url.into(),
            prefix: None,
            bearer_token: None,
            headers: Vec::new(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            call_timeout: DEFAULT_CALL_TIMEOUT,
        }
    }

    /// Connect to `url` with the default settings.
    pub async fn connect(url: impl Into<String>) -> CoreResult<Self> {
        Self::builder(url).connect().await
    }

    /// The prefix the server's tool names carry, if any.
    pub fn prefix(&self) -> Option<&str> {
        self.inner.prefix.as_deref()
    }

    /// List the server's tools (`tools/list`), ready for
    /// [`Agent::set_tools`](orion_core::Agent::set_tools).
    ///
    /// Call it again after the server's tools change to get the new set.
    pub async fn tools(&self) -> CoreResult<Vec<Box<dyn Tool>>> {
        let conn = &self.inner;
        let listed = tokio::time::timeout(conn.connect_timeout, conn.service.list_all_tools())
            .await
            .map_err(|_| {
                conn.error(format!(
                    "tools/list: no answer within {:?}",
                    conn.connect_timeout
                ))
            })?
            .map_err(|e| conn.error(format!("tools/list failed: {e}")))?;
        Ok(listed
            .into_iter()
            .map(|tool| Box::new(McpTool::new(conn.clone(), tool)) as Box<dyn Tool>)
            .collect())
    }
}

impl fmt::Debug for McpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpServer")
            .field("url", &self.inner.label)
            .field("prefix", &self.inner.prefix)
            .finish_non_exhaustive()
    }
}

/// One live connection, shared by a server and the tools taken from it.
pub(crate) struct Connection {
    label: String,
    prefix: Option<String>,
    service: RunningService<RoleClient, ClientConfig>,
    connect_timeout: Duration,
    call_timeout: Duration,
}

impl Connection {
    /// The name the agent knows a remote tool by.
    pub(crate) fn tool_name(&self, remote: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}{PREFIX_SEPARATOR}{remote}"),
            None => remote.to_string(),
        }
    }

    /// Send `tools/call`, within the call timeout.
    pub(crate) async fn call(&self, params: CallToolRequestParams) -> CoreResult<CallToolResult> {
        let name = params.name.clone();
        tokio::time::timeout(self.call_timeout, self.service.call_tool(params))
            .await
            .map_err(|_| {
                self.error(format!(
                    "tools/call '{name}': no answer within {:?}",
                    self.call_timeout
                ))
            })?
            .map_err(|e| self.error(format!("tools/call '{name}' failed: {e}")))
    }

    fn error(&self, msg: String) -> CoreError {
        CoreError::Tool(format!("MCP server {}: {msg}", self.label))
    }
}

/// The URL without its query or fragment, which may carry a key.
fn label(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or_default().to_string()
}
