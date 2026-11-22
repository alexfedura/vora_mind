# VORA MIND Backup and Restore

## Backup

Backup creation executes SQLite `VACUUM INTO` to a temporary file in the
configured backup directory, then atomically renames it to a timestamped
database file.

Validation opens the resulting database read-only and checks:

- SQLite integrity
- Foreign-key consistency

## Restore

Restore preflight verifies that the source path is inside the configured backup
directory and validates the database.

The final atomic restore implementation remains a Phase 2 follow-up if the live
database connection must be replaced safely rather than reopening after
shutdown. No restore command currently replaces active database state.
