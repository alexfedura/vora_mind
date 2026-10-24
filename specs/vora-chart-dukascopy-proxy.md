# Feature Specification: Dukascopy Local Proxy Policy

> **Status**: DRAFT
> **Target Scope**: `market | config`

## Intent

Use the persisted local proxy settings in VeraChart Policy to optionally route Dukascopy requests through an app-scoped SOCKS5 or HTTP proxy. When disabled, requests connect directly to Dukascopy without inheriting environment proxies.

## Behavior

- The Policy panel stores `use_local_proxy`, `proxy_protocol`, `proxy_host`, and `proxy_port` in the existing `vora.policy.M` localStorage object.
- Enable local proxy by default in new policy settings and migrate existing installations to enabled once, while preserving the user's setting on later policy edits.
- The market API bridge sends those settings with each historical-candle IPC request.
- Rust builds a reusable `reqwest::Client`, applies only the explicitly configured local proxy, and keeps the Dukascopy request URL at `https://freeserv.dukascopy.com/2.0/index.php`.
- Query parameters are appended to the Dukascopy URL using `reqwest::Url`.

## Non-Goals and Compatibility

- No database schema change; the existing frontend Policy object is the persisted setting and IPC carries its current values.
- Proxy configuration applies only to the Dukascopy HTTP client, not system-wide traffic.

## Verification

- `pnpm tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Unit coverage verifies SOCKS5 and HTTP proxy URL construction and successful client configuration.