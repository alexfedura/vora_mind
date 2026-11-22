# VORA Phase 3.12.1 Isolated Restore Acceptance

## Isolation

The implementation supports `VORA_GO_DATA_DIR` for a dedicated database directory. The MCP executable and Wails executable both use this supported environment variable.

## Automated evidence

Passed:

- `TestValidBackupRestoreReopensDatabase`
- `TestRestorePreservesProjectAndWorkspaceRoot`
- `TestRestoreRejectsWrongDatabaseAndIncompatibleSchema`
- `TestBackupRejectsCorruptDatabase`
- `TestRestorePreflightRejectsOutsideScope`

## Human-supervised evidence

Not performed. No real or active database was restored.

## Status

**HUMAN REQUIRED**
