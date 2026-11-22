# VORA Phase 3.9 Rust/Tauri Retirement Readiness

## Readiness: NOT READY

The Rust/Tauri implementation remains the only evidenced runtime for most product domains.

## Blockers

- 333 Tauri commands require systematic domain and caller disposition.
- 284 commands have no direct frontend invoke string and require indirect/runtime review.
- Scientific schemas and provenance relationships are not represented in the Go database.
- Legacy backup, settings, media, board, canvas, and source contracts lack Go equivalents.
- Tauri plugins and native operations have no complete Wails replacement inventory.

## Retirement prerequisites

1. Complete active indirect-caller verification.
2. Implement shared Go domain contracts.
3. Port frontend calls to Wails bindings.
4. Verify legacy data migration and backup compatibility.
5. Prove HTTP/MCP/workflow parity for migrated capabilities.
6. Run end-to-end acceptance before removing Tauri commands or plugins.
