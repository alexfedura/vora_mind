# Vora Data Export and Backup

> **Status**: Approved for implementation
> **Scope**: `mind | market | tauri | db | layout`

## Goal

Make the three user-facing export/import formats distinct and predictable:

- `.vora`: a complete, consistent SQLite workspace backup. It contains all
  database records, including user-created data and saved snapshots, but never
  the companion settings JSON.
- `.json`: an export of the complete companion settings map. Import adds only
  keys that do not already exist, preserving every current value.
- `.vr`: a selective, portable package of user-created workspace content.
  Import appends only content not already present, leaves existing content
  untouched, and records where each imported item came from.

## User-confirmed behavior

- `.vora` represents all database data; settings remain in the separately
  stored companion JSON file.
- The settings JSON is independently exportable/importable. On import, existing
  keys win and keys missing from the current settings are added.
- `.vr` can select all user-created data categories and individual content.
  It must not include provider caches or other machine-specific/generated
  market data.
- `.vr` import is additive. Identical content is skipped; new content is
  appended. It must not overwrite or delete destination data.
- Imported items retain source package identity and show an imported/source
  marker so they are distinguishable from locally authored items.
- The three formats are presented together in one clearly labeled settings
  section, with their different purpose and import behavior explained.
- Legacy `.vr` archives remain importable through their existing full-restore
  compatibility path; the new selective package format must never be confused
  with a destructive full restore.

## Data coverage

- Database backup includes every application-owned SQLite table and its rows,
  including Boards, canvas objects and links, Board versions, semantic
  snapshots and their change/checkpoint rows, Library item groups, saved chart
  snapshots, chart templates, and drawing alerts.
- Companion settings export includes the complete key/value map. Selective
  `.vr` packages may include only explicitly selected user-authored chart
  content stored in settings (for example, drawings), not unrelated personal
  preferences or secrets.
- Selective package UI enumerates selectable user-owned records and their
  required dependencies. Exporting a Board includes its canvas objects, cards,
  links, versions, and Board snapshots as one consistent unit.
- Exclude downloaded candles, market-provider caches, worker/runtime data,
  credentials, and machine-specific preferences from `.vr`.

## Safety and provenance

- `.vora` export uses SQLite's online backup API so WAL-backed data is captured
  consistently while the app is running.
- Importing `.vora` replaces the active database only, after validating the
  backup; it leaves the currently active settings JSON unchanged.
- `.vr` packages have an explicit format/version and selection manifest.
- Re-importing the same package ID is idempotent; exact matching rows are
  skipped, and ID collisions for different rows are remapped together with
  their foreign-key references.
- Import runs atomically; a failure leaves the active workspace unchanged.
- Persist package provenance (package ID/name, source metadata, import time)
  against each imported record and display an imported-items/source list in
  the backup settings section.
- Settings JSON files are validated as string-to-string JSON objects. Import
  merges only absent keys and reports how many keys were added or retained.

## Legacy behavior

- Continue reading existing `.vr` database archives. Their legacy full-restore
  behavior must be explicitly labeled and confirmed before replacement.
- The `.vr` settings section lists selectable boards (with their full canvas
  contents and history), sources (with their dependent records), charts,
  snapshots, Library item groups, drawing templates, alerts, and chart drawing
  sets. It also displays provenance for imported records.
- Continue accepting existing `.vora` database backups.
- Do not implicitly import or replace a companion JSON merely because a
  similarly named file exists next to an imported `.vora`.

## Verification

- Test consistent `.vora` export and data-only restore while settings remain
  unchanged.
- Test full settings JSON export and missing-key-only import.
- Test `.vr` selective export/import, dependency integrity, duplicate skipping,
  ID collision remapping, provenance, and rollback on failure.
- Test legacy `.vr` compatibility and ensure legacy full restore requires a
  clear destructive confirmation.
- Run `pnpm tsc --noEmit`, `pnpm build`, and
  `cargo check --manifest-path src-tauri/Cargo.toml`, plus focused Rust tests.
