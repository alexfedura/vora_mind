# Vora Mind Board Backup and Restore

> **Status**: Approved for implementation
> **Scope**: `boards | db | settings`

## Goal

Let users export and later restore a complete copy of the application's SQLite
workspace database using one `.vr` file, including all boards, cards, canvas
nodes and edges, snapshots, sources, and other persisted database records.

## Behavior

- Export opens a native save dialog filtered to `.vr` and writes a versioned
  JSON archive to the selected path.
- Restore opens a native file dialog filtered to `.vr`, validates the archive
  and its database table/column layout, then replaces the current database
  contents in one SQLite transaction.
- A failed validation or restore leaves the current database unchanged.
- A successful restore reloads the application so the restored state is read
  from SQLite.
- The archive includes SQLite-backed app data, not external files referenced
  by database paths or browser-only preferences.

## Safety and format

- The archive uses a versioned JSON envelope and preserves SQLite null,
  integer, real, text, and blob values.
- Identifiers from the archive are checked against the live database schema
  and quoted before use; archive content cannot introduce SQL statements.
- Import requires the archive's table set and column order to match the
  initialized database. Incompatible or incomplete archives are rejected
  before any data is changed.
- Foreign-key checks remain enabled and are deferred during transactional
  replacement so interrelated records can be restored safely.

## Validation

- `pnpm tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Rust tests exercise archive validation and restoration, including rollback
  when invalid data violates database constraints.
