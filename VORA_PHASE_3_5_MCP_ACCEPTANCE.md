# VORA Phase 3.5 MCP Acceptance

## Implemented

- Official MCP SDK subprocess transport test.
- Isolated `VORA_GO_DATA_DIR` database directory.
- Executable build before client launch.
- Protocol initialization through `Client.Connect`.
- Tool discovery.
- Permitted read-only `system_get_info` invocation.
- Session close and context cancellation.
- stdout remains dedicated to MCP transport; diagnostics use the existing
  stderr-oriented process lifecycle.

## Pending

- Third-party external desktop MCP client acceptance.
- Denied write and malformed-input cases across the executable process.
