# Stage 2.1 — Migration Safety Foundation

## Scope and integration

`mind::portable_db::initialize_workspace_schema` remains the only workspace
schema initializer. The new `mind::migration` module is an opt-in staging and
publication coordinator; application startup does not invoke it, and this
stage does not migrate an active workspace.

The current profile is recognized by comparing the read-only source schema to
an in-memory schema produced by the current initializer. The only recognized
profile is the exact current schema with `PRAGMA user_version = 1`. The
repository does not establish a certified historical workspace profile, so
other shapes are rejected as incomplete, unknown, or unsupported.

## Operation boundaries

1. A coordinator file lock serializes coordinator operations. The caller must
   also close every application connection; the lock cannot prevent unrelated
   SQLite clients from writing.
2. The source is opened read-only and its profile is checked before a staging
   directory is created. Existing `-wal` or `-shm` sidecars block an operation.
3. SQLite's online backup API creates an isolated staging copy on the same
   volume as the source. Migration steps are explicitly supplied by the
   caller, identified by deterministic IDs and SQL checksums, and applied in
   individual staging transactions.
4. The staged database must pass `integrity_check`, `foreign_key_check`,
   required-table checks, the caller-pinned schema fingerprint, and migration
   ledger checks before publication.
5. Publication moves the original file to `backup.sqlite` in the operation
   directory, then renames the validated staging file into place. The backup
   is retained after publication.

An append-only journal records operation phases. Recovery completes a stage
only when a durable validation event exists and the staged file still passes
its expected fingerprint and integrity gates. It abandons unvalidated work,
or restores the original if publication was interrupted before a valid stage
could be installed. An operation with ambiguous source/backup files fails
closed.

## Foreign keys and determinism

Connections used by the coordinator explicitly enable and verify foreign-key
enforcement; `foreign_key_check` independently checks existing rows. Migration
IDs, SHA-256 SQL checksums, ordered ledger entries, and a required output
fingerprint make a plan repeatable. A matching completed plan is a no-op;
other non-current schemas or unrecognized ledger contents are rejected.

## Limitations

No production migration steps and no supported legacy schema profiles are
registered. Tests use disposable synthetic SQLite fixtures only. Before
integrating the coordinator into application startup, the application must
quiesce and close all connections, add approved release-specific profile
fingerprints, and register only migrations validated against sanitized
historical samples.
