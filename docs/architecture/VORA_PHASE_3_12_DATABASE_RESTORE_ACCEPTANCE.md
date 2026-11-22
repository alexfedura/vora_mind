# VORA Phase 3.12 Database Restore Acceptance

## Automated evidence

Passed:

- `TestValidBackupRestoreReopensDatabase`
- `TestRestorePreservesProjectAndWorkspaceRoot`
- `TestRestoreRejectsWrongDatabaseAndIncompatibleSchema`
- `TestBackupRejectsCorruptDatabase`
- `TestRestorePreflightRejectsOutsideScope`

These tests use temporary databases and disposable backup directories.

## Human-supervised evidence

Not performed. Human-supervised backup/restore against a dedicated disposable database is a required blocker.

No active real database was restored or modified.
