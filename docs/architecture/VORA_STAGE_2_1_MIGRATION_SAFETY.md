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

1. A coordinator file lock serializes coordinator operations only. The trusted
   caller must quiesce database activity and close every application
   connection. Neither this lock nor sidecar inspection can prove that
   unrelated SQLite clients are absent or prevent them from writing.
2. The source is opened read-only and its profile is checked before a staging
   directory is created. Existing `-wal`, `-shm`, or rollback-journal sidecars
   block an operation, but their absence is not evidence that no external
   writer exists. An external writer can create or modify WAL/SHM state after
   inspection; copying or replacing the main database in that interval could
   omit committed WAL content. The caller must stop writers and close every
   connection for both migration and recovery.
3. SQLite's online backup API creates an isolated staging copy on the same
   volume as the source. Migration steps are explicitly supplied by the
   caller, identified by deterministic IDs and SQL checksums, and applied in
   individual staging transactions.
4. The staged database must pass `integrity_check`, `foreign_key_check`,
   required-table checks, the caller-pinned schema fingerprint, and migration
   ledger checks before publication.
5. Before publication, the coordinator makes and syncs a byte-for-byte backup
   in the operation directory, checks its SHA-256, independently opens it as
   SQLite, runs `integrity_check` and `foreign_key_check`, and verifies the
   supported current schema profile. Only then does it record
   `BackupPrepared`. Recovery applies the same checks before accepting a
   backup or using it for restoration. An incomplete backup is preserved under
   an unverified name before a replacement copy is created; a hash match alone
   never makes a backup usable. The coordinator then replaces the canonical
   file with one same-volume filesystem rename. The original is never first
   renamed away from the canonical path. The validated backup is retained
   after publication.

An append-only journal records operation phases. For process interruption,
recovery completes a stage only when a durable validation event exists and
the staged file still passes its expected fingerprint and integrity gates. A
recovery run compares the canonical file and a validated backup with the
journaled hashes: it can retry publication when the original is still
canonical, or complete the journal when the validated output is already
canonical. It restores the original from a separately validated temporary
backup copy using the same replacement operation if confirmation of a
published file fails. Incomplete or ambiguous states fail closed, recovery is
repeatable, and unverified artifacts are preserved. A failed replacement is
never emulated with two renames.

On Unix, same-filesystem `rename` atomically replaces an existing destination;
new open attempts observe either the old or the new canonical file, not a
missing-path interval. On Windows, the implementation makes one call to
`std::fs::rename` and requires the OS/filesystem to support replace-existing
same-volume rename semantics. If it does not, publication fails and the
existing canonical file and retained backup are left for recovery; there is
no non-atomic fallback. Other operating systems fail closed at the replacement
helper. Atomic visibility does not itself guarantee persistence across sudden
power loss. This implementation does not flush parent-directory metadata or
provide platform/filesystem-specific evidence for power-loss durability;
already-open handles may continue to refer to the pre-replacement file.

## Foreign keys and determinism

Connections used by the coordinator explicitly enable and verify foreign-key
enforcement; `foreign_key_check` independently checks existing rows. Migration
IDs, SHA-256 SQL checksums, ordered ledger entries, and a required output
fingerprint make a plan repeatable. A matching completed plan is a no-op;
other non-current schemas or unrecognized ledger contents are rejected.

## Limitations

No production migration steps and no supported legacy schema profiles are
registered. The coordinator is unavailable for production use until a trusted
caller can guarantee that all application and external SQLite writers are
stopped and all source connections are closed for the whole operation,
including recovery. The coordinator cannot enforce that condition itself.
Tests use disposable synthetic SQLite fixtures only. Tauri setup before the
shared database connection is opened is a possible lifecycle integration
point, but does not prove that external SQLite clients are quiescent. Before
any production integration, the application must establish that trusted
quiescence gate, add approved release-specific profile fingerprints, and
register only migrations validated against sanitized historical samples.
Passing process-interruption tests is not a power-loss durability guarantee;
that would require platform/filesystem-specific flush guarantees and evidence.
