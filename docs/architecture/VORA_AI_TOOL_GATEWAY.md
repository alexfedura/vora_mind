# VORA MIND AI Tool Gateway

The canonical tool source is the Go capability registry. MCP and future provider
adapters translate registered capability descriptors rather than owning tool
logic.

## Canonical descriptor fields

- Stable capability ID
- Semantic version
- Name
- Description
- Input JSON schema
- Output JSON schema
- Required permissions

## MCP status

- SDK: `github.com/modelcontextprotocol/go-sdk` v1.6.1
- Specification: official SDK 2025-11-25 support
- Transport: local stdio
- Tool names: capability IDs with dots converted to underscores
- Execution: common capability runtime
- Permissions: AI origin with explicit project/database/read-only filesystem scopes
- Audit: execution history and audit events

MCP is not exposed over loopback HTTP. Network MCP exposure requires a separate
authentication, origin, and consent design.
