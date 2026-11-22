# VORA Phase 3.7 Remaining Acceptance

| Item | Classification | Evidence |
|---|---|---|
| Independent third-party MCP invocation | Blocked / human acceptance | VS Code MCP config exists and subprocess tests pass; live chat automation could not submit a stable prompt |
| GUI approval/denial | Requires human acceptance | Project-delete approval is fixed and source/runtime-tested; workflow approval/deny GUI remains unperformed |
| Live database restore | Requires isolated human acceptance | Automated restore/rollback passed; no live database restore was performed |
| Two-project filesystem switching | Automated pass, GUI blocked | Runtime isolation tests passed; real GUI switching was not observed |
| Windows symlink privilege | Blocked | Symlink test skips where Windows privilege differs |
| HTTP filesystem parity | Partial / not complete | Runtime and MCP exist; full HTTP filesystem route parity is not established |

## Safe manual MCP procedure

1. Open VS Code MCP server view and start `vora-mind`.
2. Confirm stdio startup and server initialization.
3. List tools and inspect server information.
4. Call `system.get_info`.
5. Call `project.list` without write permissions in the first pass.
6. Record server name/version, transport, executable path, environment variable names, startup result, tool names, and sanitized outputs.
7. Do not test restore or deletion until isolated data and explicit approval are available.
