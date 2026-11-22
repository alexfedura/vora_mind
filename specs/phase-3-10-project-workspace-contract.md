# Phase 3.10 Project and Workspace Root Contract

## Scope

This phase implements the first fully Go-owned project and workspace-root domain.

## Capabilities

- Existing project CRUD and project filesystem root lookup remain authoritative.
- `project.open` sets and persists the trusted active project context.
- `project.get_context` returns the active project context.
- `project.filesystem_roots` lists registered roots.
- `project.filesystem_root_update` registers or updates a project root.
- `project.filesystem_root_remove` removes a project root.
- `project.filesystem_root_resolve` resolves and validates a project-relative path.

No separate `project.switch` capability is added. Wails and external adapters use
`project.open` for switching.

## Security

- Project existence and ownership are checked before every operation.
- Active project state is trusted application state, not caller metadata.
- HTTP/MCP cannot supply caller-controlled origin or principal identity.
- Project-relative filesystem inputs must be relative paths.
- Absolute, drive-qualified, UNC, mixed-separator, and encoded traversal inputs
  are rejected.
- Canonical paths must remain inside the project root.
- Existing symlinks/junctions are canonicalized when resolving containment.

## Persistence

Active project context uses `application_identity.active_project_id`.
Workspace roots remain durable in `project_filesystem_roots`.
