# VORA Phase 3.10 Adapter Parity Report

| Operation | Wails | HTTP | MCP | Workflow |
|---|---|---|---|---|
| Project create/list/get/update | yes | yes | yes | shared runtime |
| Project open/context | yes | yes | yes | shared runtime |
| Project delete | approval UI | structured approval-required denial | structured approval-required denial | approval support |
| List roots | yes | no external filesystem policy in HTTP test yet | yes | shared runtime |
| Update root | yes | implemented route; external filesystem write denied by origin policy | filesystem write denied for AI origin | capability |
| Remove root | approval UI | approval required | approval required | capability |
| Resolve root | yes | implemented route | yes | capability |

All adapters call the shared Go runtime or runtime service. No HTTP/MCP/Wails project database implementation was added.
