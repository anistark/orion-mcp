use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use orion_core::{
    Agent, AgentConfig, CoreError, CoreResult, GenerationResult, InferenceParams, LlmBackend, Role,
    TokenCallback, Tool, ToolContext,
};
use orion_mcp::McpServer;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

/// A small MCP server: `echo`, `whoami`, `fail` and `slow`.
#[derive(Clone)]
struct TestServer;

fn schema(value: Value) -> Arc<serde_json::Map<String, Value>> {
    Arc::new(value.as_object().unwrap().clone())
}

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut echo = rmcp::model::Tool::new(
            "echo",
            "Repeat the text back",
            schema(json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            })),
        );
        echo.title = Some("Echo".into());
        let empty = || schema(json!({ "type": "object" }));
        Ok(ListToolsResult::with_all_items(vec![
            echo,
            rmcp::model::Tool::new("whoami", "The Authorization header sent", empty()),
            rmcp::model::Tool::new("fail", "Always fails", empty()),
            rmcp::model::Tool::new("slow", "Answers after five seconds", empty()),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let text = |s: &str| CallToolResult::success(vec![ContentBlock::text(s)]);
        let result = match request.name.as_ref() {
            "echo" => {
                let args = request.arguments.unwrap_or_default();
                text(args.get("text").and_then(Value::as_str).unwrap_or_default())
            }
            "whoami" => {
                let parts = context.extensions.get::<http::request::Parts>();
                let header = parts
                    .and_then(|p| p.headers.get("authorization"))
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("none");
                let key = parts
                    .and_then(|p| p.headers.get("x-api-key"))
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("none");
                text(&format!("{header} / {key}"))
            }
            "fail" => CallToolResult::error(vec![ContentBlock::text("disk is full")]),
            "slow" => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                text("done")
            }
            other => {
                return Err(ErrorData::invalid_params(
                    format!("no tool named {other}"),
                    None,
                ));
            }
        };
        Ok(result.into())
    }
}

/// Serve [`TestServer`] on a free local port and return its URL.
async fn serve() -> (String, CancellationToken) {
    let config = StreamableHttpServerConfig::default();
    let ct = config.cancellation_token.clone();
    let service: StreamableHttpService<TestServer, LocalSessionManager> =
        StreamableHttpService::new(|| Ok(TestServer), Default::default(), config);
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let shutdown = ct.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await;
    });
    (format!("http://{addr}/mcp"), ct)
}

fn find<'a>(tools: &'a [Box<dyn Tool>], name: &str) -> &'a dyn Tool {
    tools
        .iter()
        .find(|t| t.name() == name)
        .unwrap_or_else(|| panic!("no tool named {name}"))
        .as_ref()
}

fn ctx(name: &str) -> ToolContext {
    ToolContext::detached("call-1", name)
}

#[tokio::test]
async fn lists_tools_under_the_prefix() {
    let (url, _ct) = serve().await;
    let server = McpServer::builder(&url)
        .prefix("demo")
        .connect()
        .await
        .unwrap();
    assert_eq!(server.prefix(), Some("demo"));

    let tools = server.tools().await.unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    names.sort();
    assert_eq!(
        names,
        ["demo__echo", "demo__fail", "demo__slow", "demo__whoami"]
    );

    let echo = find(&tools, "demo__echo");
    assert_eq!(echo.label(), "Echo");
    assert_eq!(echo.description(), "Repeat the text back");
    assert_eq!(echo.parameters_schema()["required"], json!(["text"]));
    assert_eq!(find(&tools, "demo__fail").label(), "fail");
}

#[tokio::test]
async fn keeps_server_names_without_a_prefix() {
    let (url, _ct) = serve().await;
    let tools = McpServer::connect(&url)
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    assert!(tools.iter().any(|t| t.name() == "echo"));
}

#[tokio::test]
async fn calls_a_tool() {
    let (url, _ct) = serve().await;
    let tools = McpServer::connect(&url)
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    let out = find(&tools, "echo")
        .execute(json!({ "text": "hello" }), &ctx("echo"))
        .await
        .unwrap();
    assert_eq!(out.content, "hello");
    assert!(!out.pending);
}

#[tokio::test]
async fn sends_the_hosts_headers() {
    let (url, _ct) = serve().await;
    let server = McpServer::builder(&url)
        .bearer_token("s3cret")
        .header("x-api-key", "k1")
        .connect()
        .await
        .unwrap();
    let tools = server.tools().await.unwrap();
    let out = find(&tools, "whoami")
        .execute(json!({}), &ctx("whoami"))
        .await
        .unwrap();
    assert_eq!(out.content, "Bearer s3cret / k1");
}

#[tokio::test]
async fn never_shows_secrets_in_debug_output() {
    let builder = McpServer::builder("https://mcp.example.com/mcp?key=abc")
        .bearer_token("s3cret")
        .header("x-api-key", "k1");
    let shown = format!("{builder:?}");
    assert!(!shown.contains("s3cret") && !shown.contains("k1") && !shown.contains("abc"));
    assert!(shown.contains("x-api-key"));
}

#[tokio::test]
async fn an_error_result_fails_the_call() {
    let (url, _ct) = serve().await;
    let tools = McpServer::connect(&url)
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    let err = find(&tools, "fail")
        .execute(json!({}), &ctx("fail"))
        .await
        .unwrap_err();
    assert!(
        matches!(&err, CoreError::Tool(msg) if msg.contains("disk is full")),
        "{err}"
    );
}

#[tokio::test]
async fn rejects_arguments_that_are_not_an_object() {
    let (url, _ct) = serve().await;
    let tools = McpServer::connect(&url)
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    let err = find(&tools, "echo")
        .execute(json!("hello"), &ctx("echo"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Tool(_)), "{err}");
}

#[tokio::test]
async fn a_server_that_is_down_fails_to_connect() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    drop(listener);

    let started = Instant::now();
    let err = McpServer::builder(url)
        .connect_timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Tool(_)), "{err}");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn a_server_that_stops_fails_the_call() {
    let (url, ct) = serve().await;
    let tools = McpServer::builder(&url)
        .call_timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    ct.cancel();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let err = find(&tools, "echo")
        .execute(json!({ "text": "hello" }), &ctx("echo"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Tool(_)), "{err}");
}

#[tokio::test]
async fn a_slow_call_times_out() {
    let (url, _ct) = serve().await;
    let tools = McpServer::builder(&url)
        .call_timeout(Duration::from_millis(200))
        .connect()
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    let err = find(&tools, "slow")
        .execute(json!({}), &ctx("slow"))
        .await
        .unwrap_err();
    assert!(
        matches!(&err, CoreError::Tool(msg) if msg.contains("no answer")),
        "{err}"
    );
}

#[tokio::test]
async fn stopping_the_run_stops_the_call() {
    let (url, _ct) = serve().await;
    let tools = McpServer::connect(&url)
        .await
        .unwrap()
        .tools()
        .await
        .unwrap();
    let abort = Arc::new(AtomicBool::new(false));
    let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
    let ctx = ToolContext::new("call-1", "slow", abort.clone(), events);

    let stop = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        abort.store(true, Ordering::Relaxed);
    });
    let started = Instant::now();
    let err = find(&tools, "slow")
        .execute(json!({}), &ctx)
        .await
        .unwrap_err();
    stop.await.unwrap();
    assert!(matches!(err, CoreError::Aborted), "{err}");
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// Calls `demo__echo` once, then answers with plain text.
struct Scripted(AtomicUsize);

impl LlmBackend for Scripted {
    fn generate(
        &self,
        _prompt: &str,
        _params: &InferenceParams,
        _abort: Arc<AtomicBool>,
        mut on_token: TokenCallback,
    ) -> CoreResult<GenerationResult> {
        let text = match self.0.fetch_add(1, Ordering::SeqCst) {
            0 => {
                "```tool_call\n{\"name\": \"demo__echo\", \"arguments\": {\"text\": \"from mcp\"}}\n```"
            }
            _ => "Done.",
        };
        on_token(text, 1, 1.0);
        Ok(GenerationResult {
            text: text.into(),
            tokens_generated: 1,
            prompt_tokens: 0,
            tokens_per_sec: 1.0,
            time_to_first_token_ms: 1.0,
            generation_time_ms: 1.0,
        })
    }

    fn tokenize_count(&self, text: &str) -> CoreResult<u32> {
        Ok(text.split_whitespace().count() as u32)
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn an_agent_runs_server_tools() {
    let (url, _ct) = serve().await;
    let server = McpServer::builder(&url)
        .prefix("demo")
        .connect()
        .await
        .unwrap();

    let mut agent = Agent::new(AgentConfig::default());
    agent.set_tools(server.tools().await.unwrap());
    let backend: Arc<dyn LlmBackend> = Arc::new(Scripted(AtomicUsize::new(0)));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    agent.prompt("Echo something", backend, tx).await.unwrap();

    let result = agent
        .messages()
        .iter()
        .find(|m| m.role == Role::ToolResult)
        .expect("a tool result");
    assert!(result.content.contains("from mcp"), "{}", result.content);
    assert_eq!(agent.messages().last().unwrap().content, "Done.");
}
