# VORA Phase 3.7 Security Report

## Findings and Resolutions

- A real Wails delete invocation was incorrectly rejected with `APPROVAL_REQUIRED` because the binding called the legacy execution path without setting the approved human context.
- Fixed `RuntimeService.DeleteProject` to use `ExecuteWithContext` with `OriginHuman`, human principal identity, project write/read permissions, exact project scope, correlation ID, and approval bound to the exact delete input.
- Added a test requiring approval by default and success only for the exact approved human context.
- Caller-controlled execution-origin spoofing remains fail-closed in `Execute`.
- Project filesystem isolation, symlink escape, credential revocation/expiration, restore rollback, malformed MCP input, and permission denial all pass automated tests.
- Secret-pattern review found no private key or credential-shaped production secret.

## Known Security Limitations

- The Windows symlink test was skipped because symlink privilege differs in this environment.
- Live third-party MCP client acceptance remains blocked by unstable VS Code Chat automation.
