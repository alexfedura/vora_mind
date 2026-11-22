# VORA Phase 3.11 Capability Parity Report

| Operation | Go ID | Wails | HTTP | MCP | Workflow |
|---|---|---|---|---|---|
| Create project | `project.create` | yes | yes | yes | registry |
| List projects | `project.list` | yes | yes | yes | registry |
| Get project | `project.get` | yes | yes | yes | registry |
| Update project | `project.update` | yes | yes | yes | registry |
| Delete project | `project.delete` | approval | approval denial | approval denial | approval |
| Open project | `project.open` | yes | yes | yes | registry |
| Get context | `project.get_context` | yes | yes | yes | registry |
| Get root | `project.filesystem_root` | method | not dedicated | yes | registry |
| List roots | `project.filesystem_roots` | yes | route | yes | registry |
| Update root | `project.filesystem_root_update` | yes | route | yes | registry |
| Remove root | `project.filesystem_root_remove` | approval | route | approval required | registry |
| Resolve path | `project.filesystem_root_resolve` | yes | route | yes | registry |

All project operations execute through `Runtime.ExecuteWithContext` or the Wails `RuntimeService` shared runtime path.
