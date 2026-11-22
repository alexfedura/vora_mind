# VORA Phase 3.7 Acceptance Matrix

| Workstream | Automated | Subprocess | Source/UI | Actual GUI | Third-party client | Status |
|---|---|---|---|---|---|---|
| Project create/open/list/delete | yes | n/a | yes | passed create/open/delete | n/a | Passed |
| Project workspace/root isolation | yes | n/a | yes | source-level; GUI root not displayed | n/a | Passed automated |
| Credential create/rotate/revoke | yes | n/a | yes | passed masking, rotate, revoke | n/a | Passed |
| Backup create/validate | yes | n/a | yes | passed create/list/validate | n/a | Passed |
| Restore execution | yes | n/a | yes | intentionally not run against live DB | n/a | Blocked GUI; automated passed |
| Workflow approval/fingerprints | yes | n/a | yes | no workflow UI activity performed | n/a | Automated passed |
| MCP discovery/read-only calls | yes | passed | n/a | n/a | blocked live UI | Subprocess passed |
| MCP permission/error handling | yes | passed | n/a | n/a | blocked live UI | Subprocess passed |

## Capability Summary

Go runtime remains the shared implementation for project, capability, credential, backup, restore, workflow, and execution-history operations. Wails calls the runtime through `RuntimeService`. MCP and HTTP transports call the same runtime or shared application services rather than duplicating business logic.
