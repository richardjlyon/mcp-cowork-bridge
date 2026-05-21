# mcp-cowork-bridge

Expose a local [`rmcp`](https://crates.io/crates/rmcp) MCP server to **Claude.ai's web Cowork sandbox** via [Tailscale Funnel](https://tailscale.com/kb/1223/funnel), with OAuth 2.1 + PKCE authentication and `launchd` integration.

A reusable building block for local-first MCPs on macOS — extracted from [`zotero-connector`](https://github.com/richardjlyon/zotero-connector) and shared with [`things-mcp`](https://github.com/richardjlyon/things-mcp) and any future MCP that needs to be reachable from Claude.ai's web sandbox.

**Status:** scaffolding (v0.1.0 not yet published). Module surface and design are tracked in `docs/superpowers/specs/`.

## What this crate does

A consumer MCP — e.g. `things-mcp`, `zotero-mcp`, your next domain bridge — provides an `Arc<dyn rmcp::ServerHandler>`. This crate handles everything else:

- **HTTP transport.** `rmcp::transport::streamable_http_server::StreamableHttpService` bound to `127.0.0.1`. POST `/mcp` for streamable HTTP, GET `/mcp` for SSE.
- **OAuth 2.1 + PKCE.** Discovery endpoints (RFC 8414 / 9728), `/register` (RFC 7591), `/authorize`, `/token`. Single-tenant: one `client_id` per machine.
- **Bearer middleware.** `tower-http` Authorization layer in front of `/mcp`. Tokens SHA-256-hashed at rest under `~/Library/Application Support/<config_dir>/tokens.json` (0600).
- **Setup wizard primitives.** Composable steps: Tailscale Funnel detection, credential generation, launchd plist install, self-test. Consumers wire the ones they need.
- **Optional shared `clap` subcommands.** `setup`, `status`, `show-credentials`, `http-server`. Consumers can use them as-is or define their own.

## Non-goals

- **Windows / Linux support.** macOS only — launchd + Tailscale assumptions are baked in.
- **Multi-tenant OAuth.** One user, one machine, one `client_id`.
- **Transports other than Tailscale Funnel.** Future plans may abstract this; today, Funnel is the only supported exposer.
- **Tool implementation.** This crate doesn't ship tools. The consumer's `ServerHandler` does.

## License

Dual-licensed under MIT OR Apache-2.0.
