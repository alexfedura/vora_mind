# Vora Two-File Workspace Storage

## Goal

Use one SQLite database file for workspace records and a companion JSON file
for all application, chart, canvas, policy, drawing, layout, and worker
preferences. A workspace is a paired set of files.

## File Pairing

- The database remains the durable store for normalized workspace data,
  boards, canvas records, charts, and chart narrative snapshots.
- Settings use a JSON object with string values in a companion file sharing
  the database file's stem and directory:
  `<database-stem>.settings.json`.
- The application uses one fixed database path, `vora_data.vora`, in its app
  data directory. Its companion settings file is `vora_data.settings.json`.
- Database switching is not supported; the user does not have to manage
  database locations or path-selector configuration.
- For an initialized database with no companion file, migrate settings from
  its legacy `vora_settings` table. If that table is absent, create an empty
  settings file.
- If both a companion JSON file and legacy database settings exist, the JSON
  values win conflicts and missing JSON keys are filled from the legacy table.
- No database path-selector file is created. Older `database.json` selectors
  pointing at the fixed database are removed after startup migration.

## Migration and Runtime

- On startup, load/validate the companion JSON, migrate
  legacy SQLite settings idempotently, atomically persist the merged result,
  and only then remove the legacy `vora_settings` table.
- Settings reads, writes, deletion, replacement, store initialization, flush,
  and refresh all use the companion JSON through typed Tauri commands.
- Existing browser-local-storage settings are migrated into the companion file
  through the existing frontend settings bootstrap.
- Malformed settings JSON or file I/O errors are reported; do not silently
  replace user settings with defaults.

## Backup Compatibility

- New `.vr` workspace backups include all settings and snapshots alongside the
  database archive; restore writes settings to the active database's companion
  file and refreshes the frontend settings store.
- Continue accepting old `.vr` archives with settings in `vora_settings` or
  legacy browser storage.
- New databases do not create or write `vora_settings`.

## Verification

- Test atomic settings JSON read/write and legacy settings migration and
  precedence. Verify that startup uses the fixed database path and removes a
  legacy path selector only after successful migration.
- Test new backup export/restore of settings and snapshots and import of a
  legacy archive containing `vora_settings`.
- Run `pnpm tsc --noEmit`, `pnpm build`, and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
