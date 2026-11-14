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
   directory is created. Existing `-wal`, `-shm`, or rollback-journal sidecars
   block an operation. The caller must quiesce SQLite users and close every
   connection; the coordinator lock serializes only coordinator calls and
   cannot prevent external clients from writing.
3. SQLite's online backup API creates an isolated staging copy on the same
   volume as the source. Migration steps are explicitly supplied by the
   caller, identified by deterministic IDs and SQL checksums, and applied in
   individual staging transactions.
4. The staged database must pass `integrity_check`, `foreign_key_check`,
   required-table checks, the caller-pinned schema fingerprint, and migration
   ledger checks before publication.
5. Before publication, the coordinator makes and syncs a byte-for-byte backup
   in the operation directory and records its SHA-256 in the journal. It then
   replaces the canonical file with one same-volume filesystem rename. The
   original is never first renamed away from the canonical path. The backup is
   retained after publication.

An append-only journal records operation phases. Recovery completes a stage
only when a durable validation event exists and the staged file still passes
its expected fingerprint and integrity gates. A recovery run compares the
canonical file and backup with the journaled hashes: it can retry publication
when the original is still canonical, or complete the journal when the
validated output is already canonical. It restores the original from a
temporary backup copy using the same replacement operation if confirmation of
a published file fails. Incomplete or ambiguous states fail closed, and
recovery is repeatable. A failed replacement is never emulated with two
renames.

On Unix, same-filesystem `rename` atomically replaces an existing destination;
new open attempts observe either the old or the new canonical file, not a
missing-path interval. On Windows, the implementation makes one call to
`std::fs::rename` and requires the OS/filesystem to support replace-existing
same-volume rename semantics. If it does not, publication fails and the
existing canonical file and retained backup are left for recovery; there is
no non-atomic fallback. Other operating systems fail closed at the replacement
helper. Atomic visibility does not itself guarantee persistence across sudden
power loss on every filesystem, and already-open handles may continue to refer
to the pre-replacement file.

## Foreign keys and determinism

Connections used by the coordinator explicitly enable and verify foreign-key
enforcement; `foreign_key_check` independently checks existing rows. Migration
IDs, SHA-256 SQL checksums, ordered ledger entries, and a required output
fingerprint make a plan repeatable. A matching completed plan is a no-op;
other non-current schemas or unrecognized ledger contents are rejected.

## Limitations

No production migration steps and no supported legacy schema profiles are
registered. Tests use disposable synthetic SQLite fixtures only. Tauri setup
before the shared database connection is opened is a possible future lifecycle
integration point, but it does not by itself prove that external SQLite
clients are quiescent. Before integrating the coordinator into application
startup, the application must enforce quiescence and close all connections,
add approved release-specific profile fingerprints, and register only
migrations validated against sanitized historical samples.
