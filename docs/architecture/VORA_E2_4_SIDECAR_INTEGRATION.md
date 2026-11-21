# E2.4 Electron-Rust Sidecar Integration

## Verification status

- Integration implementation is implemented and under acceptance review.
- Real Sidecar health and synthetic in-memory SQL were verified.
- Electron and frontend test suites passed.
- Development and packaged runtime smoke require final environment execution.

## Executable

- Development executable: `electron/sidecar/bin/vora_e021_sidecar.exe`
- Packaged resource: `process.resourcesPath/sidecar/bin/vora_e021_sidecar.exe`
- SHA-256: `0E96C035F0F8CD1C339E4E010879AEEF707C6D14BBEC9F355219035FD49D2499`
- Provenance: isolated E0.2.1/E0.2.2 JSONL DuckDB/Parquet Sidecar integration.

## Source and build

- Sidecar source: `electron/sidecar/src/main.rs`
- Sidecar manifest: `electron/sidecar/Cargo.toml`
- Build script: `pnpm electron:sidecar:build`
- The executable uses bundled DuckDB 1.20000.0 and is a Windows x64 native binary.

## Protocol

- Transport: newline-delimited JSON over stdin/stdout.
- Protocol version: `1`.
- Requests use `protocolVersion`, `requestId`, `method`, and `params`.
- Responses use `protocolVersion`, `requestId`, `ok`, `result` or `error`.
- stdout contains protocol data only; stderr contains diagnostic logging.
- Supported methods:
  - `health`
  - `create_synthetic_database`
  - `execute_approved_query`
  - `create_parquet_fixture`
  - `read_parquet_fixture`
  - `shutdown`

## Electron boundary

- `electron/sidecar-manager.ts` owns process spawning, stdout/stderr parsing, pending requests, timeout and shutdown.
- `electron/sidecar-protocol.ts` validates request/response contracts.
- `electron/sidecar-ipc.ts` exposes only status, engine info and synthetic smoke operations.
- Preload exposes only explicit methods; no process handles or raw IPC are exposed.
- Sidecar status responses do not expose executable paths to the renderer.
- Sidecar IPC methods reject any payload and reject unauthorized senders or frames.

## Packaging

- `electron/sidecar/bin/vora_e021_sidecar.exe` is packaged as a resource.
- Development resolves `electron/sidecar/bin/vora_e021_sidecar.exe`.
- Packaged mode resolves `process.resourcesPath/sidecar/bin/vora_e021_sidecar.exe`.

## Lifecycle and failure behavior

- Startup performs a real `health` request and is bounded by the startup timeout.
- Each request is correlated by request ID and bounded by a request timeout.
- stdout and stderr buffers are bounded.
- Oversized or malformed stdout rejects pending work safely.
- Unknown and malformed response IDs do not correlate to another request.
- Unexpected process exit rejects pending requests.
- Shutdown sends `shutdown`, waits for process exit, and forcibly terminates after the bounded shutdown timeout.

## Verification

- `pnpm test:sidecar`: real health and in-memory synthetic SQL tests passed.
- `pnpm test:electron`: Electron security tests passed.
- `pnpm test:frontend`: frontend tests passed.
- `pnpm tsc --noEmit`: passed.
- `pnpm electron:build`: passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed with existing migration warnings.

## Data safety

- No active VORA workspace database is opened by this integration.
- Synthetic smoke uses an in-memory DuckDB database.
- No production migration or schema change is performed.
