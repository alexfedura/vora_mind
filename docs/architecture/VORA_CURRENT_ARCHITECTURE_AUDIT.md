# VORA Current Architecture Audit

## Entry points

- Legacy desktop frontend: `src/App.tsx` with Vite entry `src/main.tsx`.
- Legacy Tauri backend: `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`.
- Electron runtime: `electron/main.ts`, preload `electron/preload.ts`.
- Electron sidecar: `electron/sidecar-manager.ts` and `electron/sidecar-ipc.ts`.
- Go/Wails runtime: `go-app/main.go`, `go-app/runtime_service.go`.
- Go frontend: `go-app/frontend/src/App.tsx`.
- MCP executable: `go-app/cmd/vora-mcp/main.go`.

## Go/Wails

Verified runtime roles:

- Wails initializes the SQLite store and filesystem scope in `go-app/main.go`.
- It registers one `RuntimeService` Wails service.
- It serves an embedded React frontend.
- Business capabilities are registered in `go-app/backend/application/runtime.go`.
- Persistence uses `modernc.org/sqlite`.
- HTTP and MCP are adapters over the shared runtime.
- A workflow engine persists workflow state and approval fingerprints.

The Go runtime currently registers 20 application capabilities.

## React frontends

Two separate frontend surfaces exist:

1. The main product frontend under `src/` uses Tauri APIs and Tauri/Rust services.
2. The Wails foundation frontend under `go-app/frontend/src/` uses generated Wails bindings.

These frontends are not a single converged application. The Wails frontend is a foundation operational UI, while the main product frontend retains the largest feature surface.

## Legacy runtimes

Electron is active as a build/package target and release workflow, with `electron/main.ts`, a Rust sidecar manager, updater code, and Electron-specific security tests.

Tauri is active in the default `package.json` scripts and release workflow. The Rust backend contains 333 `#[tauri::command]` declarations and registered IPC responsibilities.

Rust sidecars and Electron sidecars are separate from the Go/Wails executable.

## Database

The authoritative Go database is currently SQLite with project, audit, execution, workflow, credential, identity, and workspace-root tables. The legacy Rust database contains many additional domain schemas and migrations. No single shared persistence implementation has migrated every Rust/Tauri domain.

## CI/CD

- `.github/workflows/release.yml`: Tauri release.
- `.github/workflows/electron-release.yml`: Electron release.
- No Wails release workflow is currently present.
