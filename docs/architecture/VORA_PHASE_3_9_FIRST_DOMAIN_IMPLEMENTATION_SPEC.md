# VORA Phase 3.9 First Domain Implementation Spec

## Selected domain

Projects and workspace ownership.

## Objective

Complete the project context contract and make it reusable for every future project-scoped Go domain.

## Dependencies

- project repository
- filesystem scope
- runtime capability registry
- permission policy
- audit store
- migration ledger
- Wails runtime service

## Required contracts

- stable project ID
- project lifecycle
- workspace root
- project membership/context
- project-scoped entity ownership
- project boundary validation
- exact delete approval
- project deletion preserving workspace files
- project reopen/recovery
- audit and execution history

## Acceptance criteria

- A project entity cannot expose another project's filesystem root or entity ID.
- Project delete requires exact user approval and no partial side effects.
- Workspace roots recover after restart and restore.
- Wails, HTTP, MCP, and workflows use the same Go project service.
- HTTP/MCP permissions cannot caller-spoof execution origin or principal identity.
- Contract tests cover invalid, duplicate, and cross-project inputs.

## Estimated complexity

Medium.

## Risks

- Legacy project ownership schemas may use implicit or unassigned ownership.
- Workspace roots may conflict with asset and document paths.
- Delete semantics must remain metadata-only in the first migration.

## Rollback

Keep legacy code and databases unchanged. Revert only new Go contract, tests, or Wails bindings if acceptance fails.
