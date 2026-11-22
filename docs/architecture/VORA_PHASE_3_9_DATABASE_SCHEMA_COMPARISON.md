# VORA Phase 3.9 Database Schema Comparison

## Evidence

- Legacy Rust/Tauri source contains 328 table-definition/create references across SQL and Rust.
- 255 unique table names are represented in that source inventory.
- Go SQLite migrations currently create 9 foundation tables.
- The live Go SQLite database reports 7 applied migrations and no project/workflow rows.

The live Go database must not be used to infer that legacy product data is empty; it is a separate clean foundation database.

## Go foundation tables

- `schema_migrations`
- `projects`
- `audit_events`
- `execution_history`
- `workflows`
- `workflow_steps`
- `api_credentials`
- `application_identity`
- `project_filesystem_roots`

## Legacy domain groups

Legacy source evidence includes:

- canvas, boards, cards, snapshots, objects, edges, and library tables
- source and acquisition tables
- research, questions, hypotheses, observations, and evidence tables
- experiment tables
- analysis tables
- model and validation tables
- learning tables
- knowledge-transfer tables
- market and chart templates
- media and document tables
- worker queue tables

## Preservation requirements

- Do not modify or delete live Rust/Tauri databases.
- Preserve legacy foreign keys and project ownership semantics.
- Map source revisions, evidence, and provenance before schema simplification.
- Validate backup/restore compatibility at file and manifest level.
- Use additive migrations and compatibility views before any legacy retirement.
