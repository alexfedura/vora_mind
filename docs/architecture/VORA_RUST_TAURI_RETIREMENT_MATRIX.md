# VORA Rust/Tauri Retirement Matrix

## Evidence

- 333 `#[tauri::command]` declarations or references exist under `src-tauri/src`.
- `src-tauri/src/lib.rs` registers the Tauri invoke handler and plugins.
- `package.json` and `.github/workflows/release.yml` make Tauri a current build/release target.
- Main frontend services invoke 64 Tauri/import call-site lines across 23 service/component files.

## Major Rust domains

| Domain | Rust location | Migration status |
|---|---|---|
| Projects/core app state | `mind/canvas_db.rs`, `mind/resolver.rs`, `project_policy.rs` | partially established in Go project repository |
| Research/sources | `research/`, `source/` | legacy-only |
| Learning | `learning/` | legacy-only |
| Analysis | `analysis/` | legacy-only |
| Models | `model/` | legacy-only |
| Knowledge transfer | `knowledge_transfer/`, `transfer/` | legacy-only |
| Experiments | `experiment/` | legacy-only |
| Forge/agent | `forge/` | legacy-only |
| Market/chart support | `market/` | legacy-only |
| Backup/portable DB | `mind/backup.rs`, `mind/portable_db.rs`, `mind/migration.rs` | distinct backup logic; Go backup work partial |
| Media/player | `mind/media.rs`, `mind/player.rs` | legacy-only |
| Task flow/journal/library | `mind/task_flow.rs`, `mind/library_groups.rs`, `mind/journal.rs` | legacy-only |
| Vora Trace | `vora_trace.rs`, `mind/hypothesis.rs` | legacy-only |

## Retirement rule

Useful Rust implementations must be mined for contracts and behavior, not translated line-by-line. Reimplementation targets shared Go domain/application packages with exact validation and observable output preservation.

Retirement prerequisites:

1. Catalog each Tauri command and classify active frontend caller.
2. Map every active caller to a Go capability or an explicit removal.
3. Reimplement persistence and project isolation in Go.
4. Verify schema migration and backup compatibility.
5. Move native OS operations to Wails/Go as appropriate.
6. Remove plugins and dependencies only after their replacements pass tests.
