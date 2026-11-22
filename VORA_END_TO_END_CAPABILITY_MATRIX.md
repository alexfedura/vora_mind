# Vora Mind End-to-End Capability Matrix

| Capability | Wails UI | Go runtime | HTTP API | MCP | Workflow |
|---|---|---|---|---|---|
| Project list | Supported | Supported | Supported | Supported if permissioned | Supported |
| Project create/update | Supported | Supported | Supported | Supported if permissioned | Supported |
| Filesystem read | Not user panel | Supported | Unsupported by default | Supported if permissioned | Supported |
| Filesystem create/edit/delete | Not user panel | Supported | Unsupported by default | Supported if permissioned | Delete approval |
| Capability discovery | Supported | Supported | Supported | Supported | Unsupported as UI action |
| Workflow create/start/cancel/resume | Supported | Supported | Supported | Supported if permissioned | Supported |
| Execution history | Partial UI | Supported | Supported | Supported | Supported |

All adapters call the shared capability runtime. The runtime evaluates
execution origin, principal, permissions, project scope, correlation ID,
workflow ID, approval state, and audit history.
