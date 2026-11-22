# VORA Phase 3.12.1 Shared Runtime Security Review

## Evidence

The project and workspace-root operations execute through `Runtime.ExecuteWithContext`, `RuntimeService`, HTTP, MCP, and workflow dispatch.

Credential project IDs are attached to HTTP execution context and checked against `projectScope(input)` before execution.

MCP uses a fixed AI principal and permissions rather than caller-supplied trusted origin metadata.

Filesystem read is allowed for AI origin; filesystem write remains denied.

## Verification

Passed:

- Project isolation
- Credential project scope
- MCP project context
- HTTP project context
- Wails project context
- Junction, symlink, traversal, and outside-root rejection
