# VORA Go/Wails Architecture Gap Analysis

## Verified strengths

- `go-app/main.go` owns startup, data directory, filesystem scope, SQLite, runtime, Wails service, and loopback API setup.
- `go-app/backend/application/runtime.go` is the capability registry and policy execution gateway.
- The MCP and HTTP adapters call `ExecuteWithContext`.
- Wails calls `RuntimeService`, which delegates to the runtime.
- `go-app/backend/workflows/workflows.go` contains durable workflow and approval-fingerprint logic.
- Restore rebind callbacks replace runtime database-dependent services after restoration.

## Verified gaps

1. The Go runtime exposes 20 capabilities, far fewer than the Rust/Tauri product surface.
2. Credentials are exposed through Wails/HTTP but MCP does not expose credential lifecycle.
3. HTTP filesystem parity is partial.
4. The foundation Wails UI covers only projects, capabilities, workflows, credentials, and backups.
5. No Go domains exist for research, evidence, analysis, learning, experiments, models, knowledge transfer, boards/cards/media, market data, or Forge.
6. HTTP routes implement some capability-specific DTO behavior rather than every operation as one uniform capability route.
7. Approval-bound workflow parameter substitution is tested, but GUI approval/denial remains unresolved.
8. MCP origin/trust is structurally separated, but live independent client acceptance is unresolved.
9. Wails V3 official documentation could not be retrieved due to HTTP 403 in this environment; version evidence is repository manifest/source evidence, not a live documentation confirmation.

## Dependency safety

The Go runtime is currently authoritative only for the foundation capabilities. It is not yet authoritative for the complete product.
