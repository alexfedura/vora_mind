# VORA Phase 3.6 Implementation Report

Version: 0.26.10
Starting commit: c4fa41202af2bd7e1d308d5424dfe39e3117323f

## Priorities

- A Windows DPAPI: Native Windows tests passed for roundtrip and corrupted
  ciphertext. The provider is isolated behind `SecretProvider` and fails closed.
- B Project filesystem: Durable `project_filesystem_roots` migration added,
  project creation registers and persists a dedicated root, reopening recovers
  roots, project deletion preserves files, and cross-project tests pass.
- C Restore: Wails runtime methods and UI controls now list, create, validate,
  and restore backups; restore requires validation and explicit confirmation.
- D MCP: Subprocess tests now include read invocation, permission denial,
  malformed input, discovery, and session shutdown. External desktop client
  acceptance remains environment-dependent.
- E UI: Credential and restore operational controls, plus a source-level
  component integration guard, were added. Visual GUI acceptance remains pending.
- F Capability parity: Shared runtime remains the implementation path.

## Decision

Partially accepted: major remaining operational gaps were closed, with external
desktop MCP client acceptance and actual visual GUI interaction still pending.
