# Working in this repo

`mcp-cowork-bridge` is a reusable Rust library that exposes a local `rmcp::ServerHandler` to Claude.ai's web Cowork sandbox via Tailscale Funnel, with OAuth 2.1 + launchd integration on macOS.

Consumers today: `zotero-connector` (`zotero-mcp` crate), `things-mcp` (post-Plan-8). Future consumers: any local-first MCP that needs to reach Claude.ai's web sandbox.

## Conventions

- **Superpowers-driven planning.** Non-trivial changes start with a dated `docs/superpowers/specs/<date>-<topic>-design.md` followed by `docs/superpowers/plans/<date>-<topic>.md`. Implementation follows the plan.
- **TDD enforced.** Tests precede implementation. HTTP endpoints exercised via `axum_test::TestServer`. The bearer-validator and token-store seams use recording impls.
- **Stable public API.** Once v0.1.0 ships, breaking changes go in a minor version bump and require migration notes in `CHANGELOG.md`.
- **macOS-only.** Don't add platform-conditional code for Windows/Linux. The library asserts macOS at build time.
- **No new transports beyond Tailscale Funnel without a design doc.** The Funnel topology is baked in deliberately.

## Layout

| Path | Purpose |
|---|---|
| `crates/mcp-cowork-bridge/src/lib.rs` | Library root + module re-exports |
| `crates/mcp-cowork-bridge/src/transport.rs` | `rmcp::StreamableHttpService` wiring |
| `crates/mcp-cowork-bridge/src/oauth/` | OAuth 2.1 endpoints, discovery, PKCE, token store |
| `crates/mcp-cowork-bridge/src/bearer.rs` | tower-http Authorization: Bearer middleware |
| `crates/mcp-cowork-bridge/src/setup/` | Composable wizard steps + launchd plist template |
| `crates/mcp-cowork-bridge/src/cli.rs` | Optional shared clap subcommand definitions |
| `docs/superpowers/specs/` | per-change design briefs (dated) |
| `docs/superpowers/plans/` | per-change execution plans (dated) |

## Reference repos

- `zotero-connector` (`/Users/rjl/Code/mcp-zotero`) — source of the original inline implementation. Extraction proceeds by mining its `crates/zotero-mcp/src/{http_transport,oauth,bearer,setup,state}.rs`.
- `things-mcp-server` (`/Users/rjl/Code/mcp-things`) — the second consumer. Plan 8 there will depend on this crate.
