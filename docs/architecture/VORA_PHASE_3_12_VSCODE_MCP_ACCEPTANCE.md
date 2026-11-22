# VORA Phase 3.12 VS Code MCP Acceptance

## Evidence class

- Source inspection: passed
- Protocol-level test: passed
- Independent client test: BLOCKED
- Human-supervised acceptance: HUMAN REQUIRED

The official MCP Go SDK subprocess test invoked project tools through a protocol client. It verified project create/list/open/context/root list/path resolve and outside-root rejection.

VS Code itself was not used to invoke the MCP server in this phase. No live VS Code tool-call evidence is claimed.

Exact configuration:

- Server: `vora-mind`
- Transport: `stdio`
- Executable: `D:\workspace\vora_mind\vora-mcp.exe`
- MCP config: `C:\Users\HELIX\AppData\Roaming\Code\User\mcp.json`
- Environment: `VORA_GO_DATA_DIR=<dedicated test directory>`
