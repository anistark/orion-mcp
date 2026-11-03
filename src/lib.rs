//! Orion MCP: the tools of an MCP server as [`orion_core`] agent tools.
//!
//! Connect to a [Model Context Protocol](https://modelcontextprotocol.io) server
//! over Streamable HTTP, list its tools, and hand them to an [`Agent`] beside the
//! host's own. Each tool keeps the server's name, description and input schema;
//! running it sends `tools/call` to the server.
//!
//! ```no_run
//! use orion_core::{Agent, AgentConfig, CoreResult};
//! use orion_mcp::McpServer;
//!
//! # async fn run() -> CoreResult<()> {
//! let docs = McpServer::builder("https://mcp.example.com/mcp")
//!     .prefix("docs")
//!     .bearer_token("token supplied by the host")
//!     .connect()
//!     .await?;
//!
//! let mut agent = Agent::new(AgentConfig::default());
//! agent.set_tools(docs.tools().await?);
//! # Ok(())
//! # }
//! ```
//!
//! Every failure (a server that is down, an error answer, a timeout) is a
//! [`CoreError::Tool`](orion_core::CoreError::Tool), which the agent hands back
//! to the model as an error result.
//!
//! [`Agent`]: orion_core::Agent

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

mod server;
mod tool;

pub use orion_core;
pub use server::{McpServer, McpServerBuilder, PREFIX_SEPARATOR};
