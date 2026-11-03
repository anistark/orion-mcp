//! Connect to an MCP server and print the tools it offers.
//!
//! ```sh
//! cargo run --example list_tools -- https://mcp.example.com/mcp
//! MCP_TOKEN=... cargo run --example list_tools -- https://mcp.example.com/mcp
//! ```

use orion_core::CoreResult;
use orion_mcp::McpServer;

#[tokio::main]
async fn main() -> CoreResult<()> {
    let Some(url) = std::env::args().nth(1) else {
        eprintln!("usage: list_tools <server url>");
        std::process::exit(2);
    };

    let mut builder = McpServer::builder(url);
    if let Ok(token) = std::env::var("MCP_TOKEN") {
        builder = builder.bearer_token(token);
    }
    let server = builder.connect().await?;

    for tool in server.tools().await? {
        println!("{}: {}", tool.name(), tool.description());
    }
    Ok(())
}
