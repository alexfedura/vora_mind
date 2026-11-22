# VORA Migration Dependency Graph

## Dependency graph

```text
Active Tauri command inventory
  -> Rust domain contract extraction
  -> Go domain/repository design
  -> Go capability implementation
  -> Wails binding generation
  -> React feature adapter migration
  -> HTTP/MCP/workflow exposure
  -> Legacy Rust/Tauri removal
  -> Electron sidecar/runtime removal
  -> Wails-only installer/release
```

## Critical path

1. Complete the active Tauri-command and Rust-domain inventory.
2. Define clean initial SQLite/domain boundaries.
3. Implement shared Go services for the highest-value scientific domains.
4. Port the main React feature surface to Go/Wails.
5. Retire Tauri and Rust domains only after contract parity.
6. Retire Electron and the sidecar.
7. Establish Wails-only signing and release.

## Parallel work

- Electron shell and updater removal planning.
- HTTP/MCP parity for foundation capabilities.
- Legacy Rust test extraction.
- UI native-dialog and lifecycle mapping.
- Documentation and security audits.

## Sequential gates

- A Rust domain cannot be removed until a Go replacement or explicit removal decision has acceptance evidence.
- Electron cannot be removed until native lifecycle, updater, dialogs, sidecar behavior, and release packaging are represented.
- Wails-only release waits for installer, signing, updater rollback, and user-acceptance evidence.
