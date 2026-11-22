# VORA Phase 3.12.1 Final Acceptance Matrix

| Acceptance criterion | Evidence | Status |
|---|---|---|
| Read-only real database integrity audit | `PRAGMA integrity_check` and foreign-key check | Passed |
| Real VS Code MCP client invocation | Not performed | HUMAN REQUIRED |
| Unauthorized MCP behavior | Protocol test | Passed automated; live VS Code pending |
| Human-supervised isolated restore | Not performed | HUMAN REQUIRED |
| Project/root relationship restore | Isolated automated tests | Passed automated |
| Workspace-root UI fix | Frontend tests and live Wails | Passed |
| Full Go/frontend/Wails verification | Commands in final report | Passed |

## Final decision

**PARTIALLY ACCEPTED**
