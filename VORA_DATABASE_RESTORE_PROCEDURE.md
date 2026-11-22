# Vora Mind Database Restore Procedure

Restore is a staged, approval-gated operation:

1. Restrict the source to the configured backup directory.
2. Open it read-only and verify SQLite integrity and foreign-key consistency.
3. Quiesce database operations and close the active database.
4. Copy the active database to a pre-restore recovery copy.
5. Copy and validate a staged restore database.
6. Rename the active database aside, then rename the staged database into place.
7. Reopen and query the restored database.
8. If reopen fails, copy the recovery copy back and reopen it.

A restore outside the backup directory or a corrupted backup is rejected.
The live database is never intentionally left on a partially restored file.

Verified behavior includes post-restore runtime rebind tests and successful
real-query execution after reopen. Windows replacement/rollback at OS process
level remains limited until GUI/application integration invokes the restore
operation and exercises real service shutdown.
