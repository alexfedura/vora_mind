# VORA Phase 3.12 Capability Parity

| Operation | Go capability | Wails | HTTP | MCP | Automated | Live GUI |
|---|---|---|---|---|---|---|
| Create project | `project.create` | yes | yes | yes | yes | yes |
| List projects | `project.list` | yes | yes | yes | yes | yes |
| Get project | `project.get` | yes | yes | yes | yes | yes |
| Update project | `project.update` | yes | yes | yes | yes | yes |
| Delete project | `project.delete` | approval | approval | approval | yes | approved cleanup |
| Open project | `project.open` | yes | yes | yes | yes | yes |
| Active context | `project.get_context` | yes | yes | yes | yes | yes |
| List roots | `project.filesystem_roots` | yes | yes | yes | yes | yes |
| Update root | `project.filesystem_root_update` | yes | yes | yes | yes | yes |
| Remove root | `project.filesystem_root_remove` | approval | yes | approval | yes | not live |
| Resolve path | `project.filesystem_root_resolve` | yes | yes | yes | yes | source/automated |
