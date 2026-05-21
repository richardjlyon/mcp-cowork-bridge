//! `mcp-cowork-bridge` — expose a local `rmcp::ServerHandler` to Claude.ai's
//! web Cowork sandbox via Tailscale Funnel, with OAuth 2.1 + launchd
//! integration.
//!
//! This is the scaffolding crate root. Modules are added in subsequent plans:
//! - `transport`  — `rmcp::StreamableHttpService` wiring
//! - `oauth`      — OAuth 2.1 + PKCE endpoints, discovery
//! - `token_store`— `tokens.json` persistence (SHA-256 hashed at rest)
//! - `bearer`     — tower-http Authorization: Bearer middleware
//! - `setup`      — composable setup wizard steps + launchd plist template
//! - `cli`        — optional shared clap subcommand definitions

#![doc(html_root_url = "https://docs.rs/mcp-cowork-bridge")]
