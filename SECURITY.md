# Security Policy

## Supported versions

`orion-mcp` is pre-1.0. Security fixes land on the latest `0.x` release. Please upgrade to the most recent version before reporting.

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report privately via one of:

- GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) ("Report a vulnerability" under the repository's **Security** tab), or
- email **orion@anirudha.dev** with a description and reproduction steps.

You can expect an acknowledgement within a few days. Once a fix is ready we'll coordinate a release and credit you in the advisory unless you prefer to remain anonymous.

## Scope

Unlike `orion-core`, this crate performs network I/O: it connects to the MCP servers the host names and sends them the headers the host supplies. In scope: leaking a token or header value (in errors, logs or `Debug` output), sending it somewhere other than the server it was given for, or a hang or panic caused by a server's answer.

Out of scope: what a server's tools do, and what their names, descriptions and results say. Those reach the model as written, so treat every server as untrusted input and gate its tools with `orion-core`'s approval hook where it matters.
