# Changelog

All notable changes to `orion-mcp` are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-05

### Changed
- **Breaking: moves to `orion-core` 0.10.** `McpServer::tools` hands back `orion_core::Tool`s and `orion_core` is re-exported, so a host moves with it, and one on `orion-core` 0.9 stays on `orion-mcp` 0.1. Nothing else in the API changes.

## [0.1.0] - 2026-11-03

### Added
- **MCP server tools as Orion tools.** `McpServer::builder(url)` connects to an MCP server over Streamable HTTP, and `McpServer::tools` lists its tools (`tools/list`) as `orion_core::Tool`s whose `execute` sends `tools/call`. Each keeps the server's name, description and input schema.
- `McpServerBuilder::prefix` offers a server's tools as `{prefix}__{name}` (`PREFIX_SEPARATOR`).
- `McpServerBuilder::bearer_token` and `header` send host-supplied headers with every request, without storing them past the connection or showing them in errors or `Debug` output.
- `connect_timeout` (default 10s) and `call_timeout` (default 60s). A server that is down, answers with an error, or does not answer in time fails with `CoreError::Tool`. A stopped run stops the call with `CoreError::Aborted`.
- Re-export of `orion_core`, and the `list_tools` example.
