# VORA Phase 3.5 Capability Acceptance Matrix

| Capability | Go runtime | Wails | HTTP | MCP | Workflow | Automated |
|---|---|---|---|---|---|---|
| Project list/get/create/update | yes | yes | yes | yes | n/a | yes |
| Filesystem list/stat/read | yes | execute | yes | yes | yes | partial |
| Filesystem create/edit/rename/copy/move | yes | execute | yes | yes | yes | yes |
| Filesystem delete | yes, approval | execute | yes | yes | approval | yes |
| Capability discovery | yes | yes | yes | yes | n/a | yes |
| Workflow CRUD/start/cancel/resume/approve | yes | yes | yes | yes | yes | yes |
| Execution history | yes | partial | yes | yes | yes | yes |
| Database status/backup/restore | backup/restore foundations | partial | partial | yes runtime | n/a | yes |
| Credential list/create/revoke/rotate | yes | yes | yes | not exposed | n/a | yes |

Actual GUI interaction verification: not performed.
