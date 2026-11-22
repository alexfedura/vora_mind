# VORA Shared Capability Parity Matrix

| Capability/domain | Go runtime | React/Wails UI | HTTP API | MCP | Workflow |
|---|---|---:|---:|---:|---:|
| System information | yes | partial | yes | yes | n/a |
| Project CRUD | yes | yes | yes | yes | n/a |
| Project workspace roots | yes | get-only concept | partial | yes | n/a |
| Filesystem | yes | no foundation UI | partial | yes | approval as applicable |
| Credentials | yes | yes | yes | no | n/a |
| Backup create/list/validate | yes | yes | partial | yes runtime | n/a |
| Restore | yes | validate/confirm/execute | partial | yes runtime | n/a |
| Capability discovery/get | yes | yes | yes | yes | n/a |
| Workflow lifecycle | yes | list/start/cancel/resume | yes | yes | yes |
| Workflow approval | yes | no independent UI flow | yes | yes | fingerprinted |
| Execution history | yes | no dedicated UI | yes | yes | yes |
| Database status/schema | yes | no dedicated UI | yes | yes | n/a |
| Sources/revisions/acquisition | no | legacy Tauri UI | no | no | no |
| Evidence/observations/claims | no | legacy Tauri UI/domain | no | no | no |
| Research entities | no | legacy Tauri UI/domain | no | no | no |
| Search | no shared Go service | legacy UI/Rust | no | no | no |
| Visual boards/cards/shapes | no | legacy UI/Rust | no | no | no |
| Charts/datasets/market tools | no | legacy UI/Rust | no | no | no |
| Analysis/model/learning | no | legacy UI/Rust | no | no | no |
| Knowledge transfer | no | legacy UI/Rust | no | no | no |
| AI providers/streams | no shared provider-independent Go service | legacy Tauri UI | no | no | no |

Availability here means verified source/runtime/test evidence, not merely documentation or a placeholder.
