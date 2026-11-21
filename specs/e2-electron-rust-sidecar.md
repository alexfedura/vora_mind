# E2.4 Electron-Rust Sidecar Integration

## Objective
Integrate the verified DuckDB/Parquet Rust Sidecar through Electron Main and a narrow typed Preload API.

## Protocol
- Protocol version: `1`
- Transport: newline-delimited JSON over child-process stdin/stdout
- stdout: protocol data only
- stderr: diagnostic logs only
- Health method: `health`
- Synthetic database: `create_synthetic_database`
- Approved query: `execute_approved_query`
- Parquet fixture: `create_parquet_fixture`
- Parquet read: `read_parquet_fixture`
- Shutdown: `shutdown`

## Boundaries
- Renderer never launches or directly accesses the Sidecar.
- Electron Main owns process lifecycle and pending-request state.
- Preload exposes only status, engine information and one synthetic smoke operation.
- No workspace database or real user data is accessed.
