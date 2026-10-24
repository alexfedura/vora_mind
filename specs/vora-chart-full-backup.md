# Vora Chart Full Backup and Restore

> **Status**: APPROVED
> **Target Scope**: `chart`, `tauri`, `db`

## Intent

Allow a user to move or reinstall Vora and later reconstruct the saved chart
workspace from one `.vr` file. The backup must contain all application SQLite
tables, browser `localStorage` values, and chart snapshots from IndexedDB,
including per-symbol chart drawings, chart settings, and snapshot metadata.

## Behavior

- Export captures the complete current SQLite workspace, the current WebView's
  `localStorage`, and saved chart snapshots from IndexedDB in one `.vr` file.
- Import validates the file and restores the SQLite workspace transactionally.
- After a successful import, restore the backed-up `localStorage` and chart
  snapshots, then reload the application so chart state is hydrated from the
  imported values.
- Current-format backups with no browser storage (including existing `.vr`
  files) remain importable; importing one must leave current browser storage
  unchanged.
- A failed import must not replace either the current database or browser
  storage.

## Non-Goals

- Automatically backing up on every edit or app close.
- Exporting application binaries, cached provider responses outside SQLite,
  or operating-system-level credentials.
- Changing the existing per-symbol drawing persistence format.

## Verification

- Rust backup tests cover database/browser storage round-trip and legacy
  archive compatibility.
- `pnpm tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Manually export a chart with drawings/settings, import it into a clean app
  profile, and verify symbol-specific drawings, chart preferences, and saved
  snapshots are restored.