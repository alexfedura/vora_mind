# VORA Phase 3.9 Product Domain Inventory

The machine-readable matrix is [VORA_PHASE_3_9_DOMAIN_MATRIX.json](./VORA_PHASE_3_9_DOMAIN_MATRIX.json).

## Foundation domains

| Domain | Rust evidence | Go evidence | Status |
|---|---|---|---|
| Projects | canvas/project and Rust project policy | project repository and runtime | Partially migrated |
| Workspace roots | legacy project policy/filesystem behavior | durable Go project roots | Partially migrated |
| Workflows | no equivalent primary Rust domain evidenced | durable Go workflow engine | Migrated foundation |
| Credentials | legacy settings/API concepts | Go credential manager | Partially migrated |
| Backups/restore | Rust portable DB and backup code | Go backup manager | Partially migrated |
| Settings | Rust Tauri setting commands | no complete shared Go settings contract | Not migrated |

## Scientific domains

| Domain | Evidence | Go status |
|---|---|---|
| Research | `src-tauri/src/research/` and frontend research services | Not migrated |
| Sources/source revisions/acquisition | `src-tauri/src/source/` and `src-tauri/src/research/migrations/` | Not migrated |
| Questions/hypotheses/evidence | mind, research, and hypothesis modules | Not migrated |
| Experiments/results | `src-tauri/src/experiment/` | Not migrated |
| Analysis | `src-tauri/src/analysis/` | Not migrated |
| Models/model versions | `src-tauri/src/model/` | Not migrated |
| Learning | `src-tauri/src/learning/` | Not migrated |
| Knowledge transfer | `src-tauri/src/knowledge_transfer/` and `src-tauri/src/transfer/` | Not migrated |
| Decisions/Vora Trace | `src-tauri/src/vora_trace.rs` and React Vora Trace | Not migrated |

## Visual and analytical domains

- Boards/cards/shapes/canvas/snapshots: legacy Tauri and React implementation.
- Media/documents/highlights/file ingestion: legacy Tauri and React implementation.
- Journals/tasks/library: legacy Rust/React implementation.
- Charts, market data, watchlists, alerts, templates: legacy Rust/React implementation.
- Forge, agents, AST, patches, commit lineage: legacy Rust/React implementation.

None of these domains should be counted as Go-migrated solely because a directory or similarly named Go package exists. Each requires repository, contract, permission, project-context, test, and transport evidence.
