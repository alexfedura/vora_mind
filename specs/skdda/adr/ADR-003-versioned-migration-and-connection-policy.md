# ADR-003: Ordered Migrations and Verified SQLite Connections

- **Status:** Accepted for Phase 0 architecture baseline; implementation and legacy-profile certification pending.
- **Date:** 2026-11-11
- **Decision owners:** Architecture proposal; named operations/data owners not recorded.
- **Scope:** Main workspace schema initialization, migrations, imports and DB connection integrity.

## Context

Current schema is assembled from module Rust initializers and included SQL, with ad-hoc column checks and Market-specific markers. There is no unified migration ledger or observed `PRAGMA user_version` protocol. SQLite foreign-key enforcement is connection-local; the audit found a fresh-connection path that does not enable it. Actual deployed workspace schema variants and data were not inspected.

## Decision

- One Rust migration coordinator controls a deterministic numbered migration sequence for the main workspace database. Domain modules may own DDL/typed data migration, but they do not independently mutate schema outside the coordinator.
- Track the fast current version in `PRAGMA user_version` and exact applied migration IDs/checksums/timestamps/app version in `schema_migrations`. Update both with the migration in one transaction.
- Fresh creation runs the same migration chain as a supported upgrade; there is no parallel handwritten “fresh schema” that can drift.
- Preflight is read-only and matches a tested schema fingerprint/profile. Version zero or missing ledger is not assumed to be a particular legacy schema. Unknown profiles fail closed with source and diagnostics preserved.
- Upgrade a staged SQLite online-backup copy, validate it, and replace the active file only after all integrity checks. Preserve the original and recovery backup; never run destructive experiments against the only copy.
- Every app/import/CLI/test connection uses a shared factory that sets and reads back `PRAGMA foreign_keys = ON` before transactions. A connection that cannot prove enforcement is not used for scientific writes.
- One migration is transactional where SQLite supports transactional DDL. Non-transactional file operations use a staged workspace and recovery manifest. No swallowed migration errors.

## Alternatives considered

- Keep independent `CREATE TABLE IF NOT EXISTS` initialization: rejected as it does not identify a supported schema version or make upgrades atomic.
- Use only `user_version`: rejected because it cannot audit migration identity/checksum or diagnose partial/replaced scripts.
- Migrate user DB in place: rejected because destructive/partial failure can leave the only workspace unusable.
- Declare all existing DBs “v0 supported”: rejected because current source audit is not a deployed schema inventory.
- Enable FKs only on the main shared connection: rejected because SQLite settings do not propagate to separately opened connections.

## Consequences and gate

The initial implementation depends on consented, sanitized fixtures for every claimed legacy profile. Until those fixtures exist and pass migration/backup/recovery tests, automatic legacy compatibility is not certified. No schema/version/connection code is changed in this Phase 0 task.
