<h1 align="center">Orion MCP</h1>

<p align="center"><em>The tools of any MCP server, as tools for an <a href="https://github.com/anistark/orion-core">Orion</a> agent.</em></p>

[![Crates.io](https://img.shields.io/crates/v/orion-mcp.svg?style=flat-square)](https://crates.io/crates/orion-mcp) [![docs.rs](https://img.shields.io/docsrs/orion-mcp?style=flat-square)](https://docs.rs/orion-mcp) [![CI](https://img.shields.io/github/actions/workflow/status/anistark/orion-mcp/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/anistark/orion-mcp/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/crates/l/orion-mcp.svg?style=flat-square)](LICENSE) ![MSRV](https://img.shields.io/badge/MSRV-1.95-blue?style=flat-square)

`orion-mcp` connects to a [Model Context Protocol](https://modelcontextprotocol.io) server over Streamable HTTP, lists its tools, and offers each one as an [`orion_core::Tool`](https://docs.rs/orion-core/latest/orion_core/trait.Tool.html). Hand them to an agent beside your own tools and the agent calls them like any other: the call goes to the server as `tools/call`, and its answer comes back to the model.

It is built on [`rmcp`](https://crates.io/crates/rmcp), the official Rust SDK, and kept apart from `orion-core` so the harness stays free of a network client and MCP can move at its own pace.

## Usage

```toml
[dependencies]
orion-core = "0.10"
orion-mcp = "0.2"
```

```rust
use orion_core::{Agent, AgentConfig, CoreResult};
use orion_mcp::McpServer;

async fn run() -> CoreResult<()> {
    let docs = McpServer::builder("https://mcp.example.com/mcp")
        .prefix("docs")
        .bearer_token("token supplied by the host")
        .connect()
        .await?;

    let mut agent = Agent::new(AgentConfig::default());
    agent.set_tools(docs.tools().await?);
    Ok(())
}
```

To mix in your own tools, extend the list before `set_tools`:

```rust,ignore
let mut tools: Vec<Box<dyn Tool>> = vec![Box::new(MyTool)];
tools.extend(docs.tools().await?);
tools.extend(search.tools().await?);
agent.set_tools(tools);
```

`orion-mcp` re-exports `orion_core`. Use that re-export, or the same `orion-core` version it depends on, so the tools fit your agent.

## What it does

- **Tools as they are.** Each tool keeps the server's name, description and input schema. Its label is the tool's title, if the server gives one.
- **Prefixes.** `.prefix("docs")` offers `search` as `docs__search`, so two servers, or a server and your own tools, can share a name.
- **Headers.** `.bearer_token(...)` and `.header(name, value)` are sent with every request. They live only as long as the connection, and never appear in errors or `Debug` output.
- **Failures are tool errors.** A server that is down, an error answer, or no answer within the timeout is a `CoreError::Tool`, which the agent hands back to the model as an error result. Nothing panics or hangs.
- **Timeouts.** `.connect_timeout(...)` (default 10s) covers connecting and listing tools. `.call_timeout(...)` (default 60s) covers each call.
- **Stopping.** A call stops when the agent's run is stopped, and the server is told the request was cancelled.
- **Results.** Text comes through as is. Images, audio and binary resources are noted by type, and structured content goes to the result's `details`.

Call `tools()` again when a server's tools change. The connection closes once the server and every tool taken from it are dropped.

## Not yet

- stdio servers (a local process), and SSE-only servers
- OAuth sign-in (pass a token you already have with `.bearer_token`)
- Connecting the servers an `orion-core` plugin declares (`McpServerSpec`)
- Sampling, elicitation and progress events

## Examples

```sh
cargo run --example list_tools -- https://mcp.example.com/mcp
MCP_TOKEN=... cargo run --example list_tools -- https://mcp.example.com/mcp
```

## License

MIT © Kumar Anirudha
