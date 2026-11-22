# VORA Phase 3.11 MCP Acceptance Report

## Independent protocol evidence

The official MCP Go SDK client connected to the built executable through stdio in `TestMCPExecutableSubprocess`.

Verified:

- server/tool discovery
- project create
- project open
- project list
- active project context
- workspace-root list
- project-relative path resolve
- traversal rejection
- malformed project input rejection
- filesystem-write denial

Evidence source: `go-app/backend/mcp/server_test.go`.

## Defects fixed

`PermissionPolicy` now permits `FilesystemRead` for `OriginAI`, while `FilesystemWrite` remains denied for AI/MCP origins.

Credential-scoped project operations are checked in `ExecuteWithContext` before side effects.

## Remaining acceptance

No independent VS Code Chat tool invocation was claimed. A third-party live MCP client must still execute the sequence manually or through a separate external harness.
