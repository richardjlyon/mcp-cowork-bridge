# `mcp-cowork-bridge` v0.1 — design

**Date:** 2026-05-21
**Status:** drafted, awaiting user review
**Predecessor:** Inline implementation in `zotero-connector` (`crates/zotero-mcp/src/{http_transport,oauth,bearer,setup,state}.rs`, ~2900 LOC).

## Purpose

Expose a local `rmcp::ServerHandler` to **Claude.ai's web Cowork sandbox** via [Tailscale Funnel](https://tailscale.com/kb/1223/funnel), with OAuth 2.1 + PKCE authentication, bearer-middleware-gated `/mcp` traffic, persistent token store, and `launchd` integration for unattended operation.

Two known consumers — `zotero-mcp` (today, inline; will migrate) and `things-mcp` (Plan 8; will consume). Designed so a third consumer can adopt the library in ~200 LOC: an `Arc<dyn ServerHandler>` plus four config values.

## Non-goals (v0.1)

- **Windows / Linux.** macOS-only. launchd + Tailscale assumptions are baked in.
- **Multi-tenant OAuth.** Single `client_id` per machine, per consumer.
- **Token revocation endpoint** (RFC 7009). Manual remediation: delete `tokens.json`, re-run `setup`.
- **Health endpoint / metrics.** `<consumer> status` covers the operational need.
- **Keychain integration.** Filesystem (0600) — same pattern as `zotero-connector`.
- **Transports other than Tailscale Funnel.** The Funnel topology is the only supported exposer in v0.1.
- **Tool implementation.** This crate ships no tools. Consumers' `ServerHandler` does.

## Architecture overview

Consumers wire two transports:

- **stdio** — invoked by Claude Code on the Mac. Unchanged by this library; the consumer keeps its existing stdio path.
- **HTTP** (this library) — invoked as a `<consumer-binary> http-server` subcommand from a launchd plist. Binds `127.0.0.1:<port>` (config-driven, default 7892). Tailscale Funnel forwards `https://<consumer>.<tailnet>.ts.net` → the loopback port. OAuth 2.1 + bearer middleware gates every `/mcp` request.

Both transports share the consumer's `Arc<dyn ServerHandler>`. The library is **stateless about tools** — it never inspects what the handler does.

```
                                            ┌───── consumer's ServerHandler ─────┐
                                            │  (29 tools for things-mcp,         │
                                            │   34 tools for zotero-mcp, etc.)   │
                                            └─────────────────▲──────────────────┘
                                                              │ rmcp dispatch
Claude.ai ── HTTPS ─▶ Tailscale Funnel ── TLS ──▶ 127.0.0.1:7892 ─▶ axum Router
                                                                       │
                                                                       ├── /.well-known/oauth-authorization-server
                                                                       ├── /.well-known/oauth-protected-resource
                                                                       ├── /register   (OAuth dynamic client reg)
                                                                       ├── /authorize  (OAuth, interactive HTML)
                                                                       ├── /token      (OAuth, code + refresh)
                                                                       └── /mcp        (bearer-gated, streamable HTTP + SSE)
```

## Module surface

| Path | LOC est. | Purpose | Public? |
|---|---|---|---|
| `transport.rs` | ~150 | `rmcp::StreamableHttpService` wiring; `run_http_server(handler, cfg) -> ServeHandle` | yes |
| `bearer.rs` | ~150 | `tower-http` middleware: `BearerLayer::new(validator)`; consumer mounts on `/mcp` only | yes |
| `oauth/mod.rs` | ~250 | Public types (`OAuthConfig`, `OAuthState`), router builder `oauth_router(state) -> Router` | yes |
| `oauth/endpoints.rs` | ~350 | Implementations of `/authorize`, `/token`, `/register`, both discovery endpoints | crate-internal |
| `oauth/pkce.rs` | ~80 | `code_challenge` generation + verification, S256 only | crate-internal |
| `oauth/token_store.rs` | ~400 | `TokenStore` trait + filesystem impl (tokens.json, 0600, SHA-256 hashed at rest) | yes (trait + filesystem impl) |
| `setup/mod.rs` | ~120 | `WizardStep` trait, default step impls, composable `run_wizard(steps)` driver | yes |
| `setup/steps.rs` | ~200 | Concrete step impls: detect Tailscale, generate credentials, install launchd plist, self-test | yes |
| `setup/plist.rs` | ~80 | launchd plist template + bootstrap/teardown helpers | yes |
| `cli.rs` | ~150 | Optional shared clap subcommand fragments: `SetupCmd`, `StatusCmd`, `ShowCredentialsCmd`, `HttpServerCmd` | yes (re-exports clap derive types) |
| `error.rs` | ~80 | `BridgeError` enum (thiserror); covers IO, OAuth, Setup, Transport variants | yes |

**Total est. ~2010 LOC** in the library (down from ~2900 inline because shared error types, removed duplication, no per-consumer copy of the launchd plist string).

## Public API sketch

```rust
// transport.rs
pub struct ServerCfg {
    pub bind: std::net::SocketAddr,            // default 127.0.0.1:7892
    pub host_allow_list: Vec<String>,          // default: ["localhost"]
    pub sse_keep_alive: std::time::Duration,   // default 15s
}

pub struct ServeHandle { /* opaque; impls Drop + waits for shutdown */ }

pub async fn run_http_server(
    handler: std::sync::Arc<dyn rmcp::ServerHandler>,
    oauth_state: oauth::OAuthState,
    cfg: ServerCfg,
) -> Result<ServeHandle, BridgeError>;

// oauth/mod.rs
pub struct OAuthConfig {
    pub issuer: url::Url,                       // https://<consumer>.<tailnet>.ts.net
    pub access_token_ttl: std::time::Duration,  // default 7d
    pub refresh_token_ttl: std::time::Duration, // default 90d
    pub authorization_html: Option<String>,     // consumer can supply branded HTML; default works
}

pub struct OAuthState { /* contains TokenStore impl + client credentials + config */ }

impl OAuthState {
    pub fn new(cfg: OAuthConfig, store: impl token_store::TokenStore + 'static) -> Self;
    pub fn router(&self) -> axum::Router;       // mountable into the consumer's axum app
    pub fn bearer_validator(&self) -> bearer::BearerValidator;
}

// bearer.rs
pub struct BearerLayer { /* tower_layer::Layer */ }
impl BearerLayer { pub fn new(validator: BearerValidator) -> Self; }

// token_store.rs
#[async_trait::async_trait]
pub trait TokenStore: Send + Sync + 'static {
    async fn issue_access(&self, client_id: &str) -> Result<String, BridgeError>;
    async fn validate(&self, raw_token: &str) -> Result<Validation, BridgeError>;
    async fn refresh(&self, raw_refresh: &str) -> Result<TokenPair, BridgeError>;
    async fn revoke_all(&self) -> Result<(), BridgeError>;
}

pub struct FilesystemTokenStore { /* tokens.json under config_dir, 0600 */ }

// setup/mod.rs
#[async_trait::async_trait]
pub trait WizardStep: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&self, ctx: &mut WizardCtx) -> Result<(), BridgeError>;
}

pub async fn run_wizard(steps: Vec<Box<dyn WizardStep>>) -> Result<(), BridgeError>;

// cli.rs
#[derive(clap::Subcommand)]
pub enum Subcommand {
    Setup(SetupCmd),
    Status(StatusCmd),
    ShowCredentials(ShowCredentialsCmd),
    HttpServer(HttpServerCmd),
}
```

The shapes above are illustrative — final signatures fall out of the extraction plan. The library's MSRV will be the same as `rmcp`'s (currently 1.85 stable).

## Data layout under the consumer's config dir

This library writes / reads exactly two files in the consumer-supplied config directory:

| File | Mode | Owner | Sensitive? |
|---|---|---|---|
| `oauth.toml` | 0600 | library | client_id (low-sensitivity), `client_secret_hash` (SHA-256) |
| `tokens.json` | 0600 | library | all tokens at rest are SHA-256 hashed |

Consumers own everything else in their config dir (e.g., `config.toml`, `backups/`). The library never reads or writes outside its two files.

## launchd integration

`setup/plist.rs` embeds a parameterised launchd plist template (`include_str!`). The setup step generates it with:

- `Label`: `com.<consumer-id>.http` (e.g. `com.things-mcp.http`).
- `Program`: the consumer's installed binary path + `["http-server"]` argv.
- `KeepAlive`, `RunAtLoad`: `true`.
- `StandardOutPath`/`StandardErrorPath`: `~/Library/Logs/<consumer-id>.http.log`.

Bootstrap: `launchctl bootstrap gui/$UID <plist-path>`. Teardown: `launchctl bootout gui/$UID/<label>`.

The plist label and consumer-id are parameters to `setup::steps::InstallLaunchdPlist::new(...)` — no string interpolation in the library; each consumer provides its own identifier.

## Tailscale Funnel integration

`setup/steps.rs::DetectTailscale` shells out to `tailscale status --json` and `tailscale funnel status`. If Funnel isn't granted, surfaces the exact `tailscale funnel ...` invocation the user needs to run.

`setup/steps.rs::PublishFunnel` runs `tailscale serve funnel <port>` and parses the resulting URL from stdout. This URL becomes the OAuth `issuer` and is written into the consumer's config.

The library does **not** depend on Tailscale's Rust SDK — everything is shell-out to the `tailscale` CLI. This keeps the dependency footprint minimal and matches the existing zotero-connector implementation.

## Testing strategy

Target: ~60 tests in the library, broken down as:

| Layer | Tests | Tooling |
|---|---|---|
| `bearer.rs` | ~6 | tower middleware harness, recording validator |
| `oauth/pkce.rs` | ~6 | known-vector S256 challenges |
| `oauth/token_store.rs` (filesystem impl) | ~12 | tempfile + recording clock |
| `oauth/endpoints.rs` | ~14 | `axum_test::TestServer`, full request lifecycle |
| `transport.rs` | ~6 | TestServer + a stub `ServerHandler` |
| `setup/steps.rs` | ~10 | injectable shellout seam — no actual `tailscale` / `launchctl` calls in tests |
| `setup/plist.rs` | ~4 | snapshot tests (insta) for rendered plist XML |
| End-to-end | ~3 | Full HTTP + OAuth + bearer happy path with a stub handler |

Tests do **not** call out to the host's `tailscale` or `launchctl` binaries. The setup steps each take an injectable command runner; tests substitute a `RecordingRunner`.

## Migration path for `zotero-mcp`

Out of scope for this design, but planned as the immediate follow-on:

1. Land `mcp-cowork-bridge` v0.1.0 on crates.io (a separate plan in this repo).
2. In `zotero-connector`: add the dependency, replace the inline modules with library calls, ensure all existing tests pass.
3. Cut `zotero-mcp` point release.

## Migration path for `things-mcp` (Plan 8 — separate plan in things-mcp-server)

1. Add `mcp-cowork-bridge` dependency.
2. New file: `crates/things-mcp/src/cowork.rs` (~50 LOC) constructing the `OAuthConfig`, `OAuthState`, `ServerCfg` from the consumer's existing `Config`.
3. `main.rs` grows the four library-provided clap subcommands.
4. `things-mcp setup` becomes a thin wrapper composing the library's wizard steps (plus one or two Things-specific steps: probe Things 3 install, prompt for `THINGS_AUTH_TOKEN`).

Estimated things-mcp Plan-8 LOC delta: ~200 LOC + 6 tests.

## Open questions

None. Every dimension is resolved by either:
- the source-of-truth implementation in `zotero-connector`,
- explicit user choice during brainstorming (full mirror, standalone repo + crates.io, single-tenant OAuth, FS-backed token store, macOS-only),
- or stated YAGNI exclusions (multi-tenant, mTLS, revocation, keychain, non-Funnel transports, Windows/Linux).

## Next step

Write `docs/superpowers/plans/2026-05-21-extract-from-zotero-mcp.md` — the executable plan to mine, port, and clean up the inline implementation into this library. The plan is structured as a sequence of atomic commits, each independently testable.
