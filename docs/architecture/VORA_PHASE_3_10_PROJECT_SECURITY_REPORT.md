# VORA Phase 3.10 Project Security Report

## Isolation controls

- Project existence is checked in the shared project service.
- Active project context is persisted trusted application state.
- HTTP/MCP callers cannot supply origin or principal identity metadata as a trusted identity.
- Project filesystem roots are stored separately and recovered on runtime restart.
- Project resolution canonicalizes existing paths and checks containment.

## Path validation

Project-relative resolution rejects:

- absolute paths
- Windows drive paths
- UNC paths
- backslash traversal
- percent-encoded traversal
- `..` components
- paths that resolve outside the canonical project root

## Root validation

Workspace roots must be absolute and remain inside the configured application filesystem scopes or explicitly allowed filesystem roots.

## Approval and audit

- Project deletion remains approval-required.
- Workspace-root removal is approval-required.
- Execution and audit history receive the capability, origin, principal, project scope, status, and correlation ID.
