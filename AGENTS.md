# Orion MCP: Project Instructions

## About
Orion MCP is the MCP client for the Orion Agent Harness, published as the `orion-mcp` crate. It connects to a Model Context Protocol server over Streamable HTTP, lists its tools, and offers each one as an `orion_core::Tool`, so a host can hand a server's tools to an agent beside its own. Built on `rmcp`, the official Rust SDK. It uses only `orion-core`'s public API. Anything it needs from the agent loop goes into `orion-core` as a generic hook, never as MCP-specific code.

- Repository: https://github.com/anistark/orion-mcp
- Harness: https://github.com/anistark/orion-core
- License: MIT © Kumar Anirudha

## Project Structure
```sh
├── Cargo.toml                  # Crate manifest, crates.io metadata
├── README.md                   # Usage guide (the first snippet mirrors the lib.rs doctest)
├── src/
│   ├── lib.rs                  # Public API + re-exports (incl. orion_core); #![deny(missing_docs)]
│   ├── server.rs               # McpServer, McpServerBuilder, the shared connection, timeouts
│   └── tool.rs                 # McpTool: one server tool as an orion_core::Tool, result rendering
├── examples/
│   └── list_tools.rs           # Connect to a server URL and print its tools
├── tests/mcp_tests.rs          # Integration tests against an in-process rmcp server
├── .github/workflows/ci.yml    # fmt + clippy + test on stable & MSRV 1.95
├── justfile                    # Task runner commands
└── AGENTS.md                   # This file
```

## Documentation
- ALWAYS update `CHANGELOG.md` when making user-facing or API changes: add entries under `[Unreleased]`.
- ALWAYS keep `README.md` current. Its usage snippet is mirrored by the compile-checked doctest in `src/lib.rs`, so update both together and run `cargo test --doc`.
- Every public item must be documented: `#![deny(missing_docs)]` and `deny(rustdoc::broken_intra_doc_links)` are on. Keep `cargo doc` clean.

## Development
- Use `just` as task runner (see `justfile`)
- Minimum Rust version: 1.95, the same as `orion-core`. `resolver = "3"` makes Cargo pick dependency versions that still build on it, so check MSRV with a real 1.95 toolchain first on `PATH`: `rustup run 1.95.0 cargo` alone can still pick up another `rustc`.
- Keep `rmcp` types out of the public API, so an `rmcp` major bump stays an internal change. Errors are `orion_core::CoreError`.
- Secrets (bearer tokens, header values) are never stored beyond the connection, logged, or put in errors or `Debug` output.
- Track `orion-core`: depend on its latest minor and release alongside it.
- Clean code, proper naming, minimal comments, proper docstrings
- Avoid over-commenting. Add only useful comments like docstrings, TODOs and NOTEs.
- Latest stable dependency versions
- After completing a set of tasks, run these checks:
  - `just format`: `cargo fmt`
  - `just lint`: `cargo clippy --all-targets -- -D warnings`
  - `just check`: `cargo check`
  - `just test`: `cargo test`

## Stability
- Follows [SemVer](https://semver.org/). While `0.x`, a minor bump may carry breaking changes and a patch bump is additive/fixes only.
- Moving to a new `orion-core` minor is a breaking change here too, since the `Tool` trait comes from it.
- MSRV is raised only in a minor release. See `CONTRIBUTING.md` for the full policy.

## Git
- Do NOT commit automatically
- Wait for explicit "commit it" instruction
- Then stage all changes, write a short brief commit message, and commit

## Publishing
- Bump the version in `Cargo.toml`, update `CHANGELOG.md`, then:
  - `just publish-dry`: `cargo publish --dry-run` (verify the package)
  - `just publish`: `cargo publish` to crates.io
- Confirm docs.rs builds and tag the release after publishing.

## Project
- Open source, MIT licensed
- Ask instead of assuming when confused
