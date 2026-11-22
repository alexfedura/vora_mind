# VORA Phase 3.11 Database Restore Report

## Source evidence

`go-app/backend/persistence/backup.go`:

- creates a consistent SQLite backup with `VACUUM INTO`
- validates integrity and foreign keys
- checks required tables and migration metadata
- preflights sources inside the backup directory
- copies the active database to a recovery copy
- stages the replacement, renames, reopens, and post-validates it
- restores the recovery copy if reopen/post-validation fails

## Automated isolated evidence

`TestRestorePreservesProjectAndWorkspaceRoot` passed.

It verified:

- test project creation
- workspace-root creation
- backup creation
- deletion of test-only project/root data
- restore
- restored project count
- restored workspace root

`TestValidBackupRestoreReopensDatabase` and invalid/corrupt restore tests pass in the full suite.

No restore was run against the active real database.
