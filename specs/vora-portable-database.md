# Feature Specification: Portable Vora Workspace

> **Status**: IMPLEMENTED
> **Target Scope**: `mind | market | tauri | db | layout`

## Intent

The active workspace is a paired set of files: a portable SQLite database for
workspace data and a companion `<database-stem>.settings.json` for all
application, chart, canvas, policy, layout, and worker settings. See
[vora-two-file-workspace-storage.md](./vora-two-file-workspace-storage.md) for
the migration and backup contract.

The application uses a fixed database path, `vora_data.vora`, inside its app
data directory and the companion `vora_data.settings.json` beside it. Database
location switching is not supported, and the application does not create a
separate database path JSON file. Existing `database.json` files are treated
as legacy path selectors and removed after successful startup migration.

## Architecture

- `AppState` owns the active SQLite connection and database path.
- `vora_items` stores portable board/chart/library records.
- `vora_snapshots` stores chart snapshots with a foreign key to `vora_items`.
- Settings are read and written through the existing typed Tauri IPC methods,
  backed by the companion JSON file; new databases do not create
  `vora_settings`.
- Legacy `vora_settings` rows migrate to the companion file; companion values
  take precedence, and legacy-only keys are retained.
- `.vr` backups include database data, companion settings, and snapshots.
  Legacy `.vr` archives with SQLite or browser-stored settings remain
  importable. Restoring a `.vora` backup targets the fixed active database.
- Settings controls and both-file paths are available in Settings. The Policy
  Panel does not duplicate database controls.

## Verification

- Frontend checks: `pnpm tsc --noEmit` and `pnpm build`.
- Backend checks: `cargo check --manifest-path src-tauri/Cargo.toml`.
- Focused tests cover settings migration/precedence, paired-file seeding,
  schema compatibility, and legacy backup restoration.
