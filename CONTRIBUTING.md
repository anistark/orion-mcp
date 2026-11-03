# Contributing to Orion MCP

Thanks for your interest in improving Orion MCP! The `orion-mcp` crate lets an [Orion](https://github.com/anistark/orion-core) agent use the tools of any MCP server, and contributions of all kinds are welcome.

## Getting started

```sh
# Requires Rust 1.95+ (the crate's MSRV)
git clone https://github.com/anistark/orion-mcp
cd orion-mcp

cargo build
cargo test
cargo run --example list_tools -- https://mcp.example.com/mcp
```

If you have [`just`](https://github.com/casey/just), the common tasks are wrapped as recipes. Run `just` to list them (`just check`, `just lint`, `just test`, `just format`).

## Before you open a PR

Please make sure these pass (CI runs the same checks):

```sh
cargo fmt --all --check                       # formatting
cargo clippy --all-targets -- -D warnings     # lints (warnings are errors)
cargo test --all-targets
cargo test --doc                              # doctests
```

## Code style

- Build on `orion-core`'s **public API** only. If MCP needs something from the agent loop, propose a generic hook in `orion-core` rather than MCP-specific code there.
- Keep `rmcp` types out of the public API, and report failures as `orion_core::CoreError`.
- Never store, log, or print secrets such as bearer tokens and header values.
- Match the surrounding code: clear names, minimal comments. Document **public** items with `///` doc comments. Keep examples in docs/README compiling.
- New behavior should come with a test in `tests/`, against the in-process server there.

## Stability & versioning

- The crate follows [SemVer](https://semver.org/). While `0.x`, a minor bump (`0.y.0`) may carry breaking changes. Patch bumps (`0.y.z`) are additive or bug fixes only. Any public API removal or rename goes in a minor bump.
- Moving to a new `orion-core` minor is a minor bump here, since hosts must move with it.
- **MSRV is Rust 1.95**, the same as `orion-core`, and raising it is a minor-version (never a patch) change.

## Commit messages & PRs

- Use [Conventional Commits](https://www.conventionalcommits.org/) for the subject (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- Keep PRs focused. Describe what changed and why. Link any related issue.
- Update `CHANGELOG.md` under `[Unreleased]` for user-facing changes.

## Reporting bugs & proposing features

Open an issue with a minimal reproduction (the server in `tests/mcp_tests.rs` is a good starting point) or a clear description of the proposed API and its motivation.

By contributing, you agree that your contributions are licensed under the project's [MIT license](LICENSE).
