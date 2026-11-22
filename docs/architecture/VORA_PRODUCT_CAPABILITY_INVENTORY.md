# VORA Product Capability Inventory

## Fully implemented in the Go/Wails foundation

| Capability | Evidence | UI | HTTP | MCP | Workflows |
|---|---|---|---|---|---|
| System information | `runtime.go` | yes | yes | yes | n/a |
| Project create/list/get/update/delete | runtime and project repository | yes | yes | yes | n/a |
| Project workspace roots | project repository and filesystem scope | partial | partial | yes | n/a |
| Database status/schema/table discovery | runtime allowlisted schema logic | partial | yes | yes | n/a |
| Filesystem list/stat/read/create | runtime + scoped filesystem | no foundation UI | partial | yes | approval as applicable |
| Capability discovery/get | runtime registry | yes | yes | yes | n/a |
| Workflow create/get/start/cancel/resume/approve/list/history | workflow engine | list/start/cancel/resume | yes | yes | yes |
| Credentials create/list/rotate/revoke | runtime service + credential manager | yes | yes | not exposed | n/a |
| Backup create/list/validate/restore | backup manager + Wails methods | yes | partial | runtime exposed | n/a |
| Execution audit/history | audit store + SQLite | no dedicated UI in foundation | yes | yes | yes |

## Main product frontend / legacy Rust

The following domains are evidenced in `src/` and `src-tauri/src/`, but are not migrated to the Go foundation:

- Research sources, source revisions, acquisition, source trees.
- Questions, hypotheses, evidence, experiments, results.
- Analysis records and analytical workflows.
- Models and model versions.
- Learning/knowledge transfer.
- Decisions and Vora Trace graphs.
- Boards, cards, shapes, snapshots, media, links, and canvas history.
- Task flows and journals.
- File ingestion, document parsing, highlights, and document metadata.
- Drawings, charts, market data, watchlists, alerts, templates, replay, and snapshots.
- Forge agent, AST, patches, commit/lineage, planning and patch tools.
- Settings, database import/export, backups, updates, logs, and application lifecycle.

Status for these domains: **legacy-only or partially represented; not proved available through Go/Wails**.

## Unknown or unproved

- Provider-independent AI orchestration in Go.
- Durable scheduling.
- Complete source ingestion and provenance parity.
- Complete visual-object and chart capability parity.
- Independent third-party MCP runtime interaction.
- Live GUI approval/denial and restore scenarios.

These items must not be counted as migrated solely because UI or Rust source exists.
