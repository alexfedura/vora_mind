# Stage 2.0 — Synthetic Compatibility Inventory

**Status:** Synthetic fixture gate; no application migration is implemented or run by this report.  
**Evidence:** Current repository source and disposable test fixtures only. No installed or historical user database was inspected.

## Repository baseline and active database boundary

At the start of Stage 2.0, `git status --short` was clean on branch `main`.
The Tauri startup path resolves `AppData/vora_data.vora`, optionally copies
`vora_workspace.db`, runs `migrate_legacy_database_selector`, then calls
`initialize_workspace_schema` in `src-tauri/src/lib.rs` setup. The legacy
selector implementation can initialize its selected database before copying
it (`src-tauri/src/mind/portable_db.rs`, `migrate_legacy_database_selector_file`).
Neither entry point is called by the synthetic inventory tests.

The test helper creates temporary databases only below
`%TEMP%/vora-stage2-0-fixtures/<random-test-id>/`. The current fresh fixture
is produced with the repository's `initialize_workspace_schema` path; all
other profiles clone this isolated fixture or create a deliberately partial
schema. Inventory opens fixture files with SQLite read-only flags and
`PRAGMA query_only=ON`; it calls no schema initializer or migration function.

## Current schema and version inventory

| Domain | Current initializer / schema | Version evidence | Legacy data and ownership behavior |
|---|---|---|---|
| Workspace composition | `mind::portable_db::initialize_workspace_schema` in `src-tauri/src/mind/portable_db.rs` calls Canvas, workflow, Market, Source, Research, Experiment, Analysis, Model, Learning, Knowledge Transfer, Forge, and portable initializers in a fixed order. | No single workspace ledger. | Initializer order alone does not certify existing database compatibility or atomicity across domains. |
| Canvas / questions / hypotheses | `canvas_db::init_db`, `questions::init_schema`, `hypothesis::init_schema` in `src-tauri/src/mind/`. Questions and hypotheses use `CREATE TABLE IF NOT EXISTS` plus column-presence checks and `ALTER TABLE`. | No unified version ledger for these paths. | Legacy question and hypothesis rows remain in their original tables. Existing polymorphic/string references are not proven valid by schema shape alone. |
| Source | `source::db::init_schema` in `src-tauri/src/source/db.rs`, using `schema_source.sql`; acquisition DDL is `src-tauri/schema_acquisition.sql`. | Shared `schema_migrations` table plus `PRAGMA user_version`; Source acquisition migration checks version 1 and checksum. | `sources`, `segments`, `extracts`, `links`, and `source_revisions` are workspace-shared. Legacy rows do not imply a SourceVersion or project assignment. |
| Research | `research::init_schema` in `src-tauri/src/research/mod.rs`; DDL in `src-tauri/src/research/schema.sql`. | `research_schema_migrations`, version 1 and SQL checksum in `research::repository::initialize_schema`. | Legacy questions/hypotheses are synchronized into typed records with null `project_id` and `unassigned` state. The schema also permits `unknown`; no existing record is assigned by this inventory. |
| Experiment | `experiment::repository::initialize_schema` with `schema.sql` and `schema_v2.sql`. | `experiment_schema_migrations`, numbered migrations 1 and 2, migration IDs/checksums. | Experiment records are project-scoped. The schema initializer uses transactions per numbered migration. |
| Analysis | `analysis::repository::initialize_schema` and `analysis/schema.sql`. | `analysis_schema_migrations`, version 1, ID/checksum. | Analysis records carry project ownership; legacy record compatibility remains unverified outside code-defined initializers. |
| Model | `model::repository::initialize_schema` and `model/schema.sql`. | `model_schema_migrations`, version 1, ID/checksum. | Model identity origin may be null; ModelVersion `created_in_project_id` is non-null and repository logic keeps versions project-local. |
| Learning | `learning::repository::initialize_schema` and `learning/schema.sql`. | `learning_schema_migrations`, version 1, ID/checksum. | Learning schema is project-contextual; actual historical database populations are unknown. |
| Knowledge Transfer | `knowledge_transfer::repository::initialize_schema` and `knowledge_transfer/schema.sql`. | `knowledge_transfer_schema_migrations`, version 1, ID/checksum. | Stores exact origin/recipient, transfer, adaptation, and lineage references. Focused source tests exercise backup/reopen persistence; deployed backup coverage is not implied. |
| Market / Forge / Media | Initializers in `market/db.rs`, `forge/index.rs`, and `mind/media.rs`. | Market uses `_vora_meta` markers and compatibility SQL; several other changes are schema-probed/ad hoc. | Some paths have destructive or suppressed DDL operations. They are noted as existing risk, not executed or generalized by Stage 2.0. |

### Connection and backup evidence

- Tauri startup initializes the shared connection and enables foreign keys in
  `src-tauri/src/lib.rs` setup. `open_default_db_connection` opens a separate
  connection and initializes Market without explicitly setting/verifying the
  FK pragma.
- `.vora` export uses SQLite online backup in
  `mind::portable_db::create_database_backup`. Import stages the database and
  runs `quick_check` before schema initialization in
  `validate_imported_database`; this does not establish a supported historical
  profile. The settings JSON and external assets are separate.
- A fresh-workspace test in `mind::portable_db::tests` runs initialization
  twice and checks SQLite integrity and foreign keys. It does not certify
  historical profile compatibility.
- These are source-code findings. Only the focused fixture tests listed below
  have been runtime-exercised in this stage.

## Synthetic profiles

Every profile below is generated for tests only. Profiles B–D are synthetic
row variants based on a current-schema fixture, not claims about historical
release schemas. They are classified by exact current-schema fingerprint
because their structure is current; row content is deliberately synthetic.

| Profile | Construction and inspected structure | Ownership / marker observations | Classification |
|---|---|---|---|
| **A — Current fresh schema** | Calls the current workspace initializer in a unique temp directory. Inventory records every non-internal table, column, index, FK declaration, row count, SQLite `user_version`/`application_id`, and domain migration marker. | Contains current source/research/model tables and migration ledgers; no ownership backfill is performed. | Recognized only by exact equality to the fresh fixture's schema fingerprint. This is a current-schema recognition check, not a legacy compatibility profile. |
| **B — Legacy Research rows** | Clones A and adds synthetic legacy `questions`/`hypotheses` plus their typed Research projections. Both source rows and typed projections are inspectable. | Includes null `project_id` with `unassigned` and `unknown` ownership states, and legacy hypothesis references to absent inquiry IDs. This records the soft-reference shape; it does not repair it. | Current schema / synthetic row variant. |
| **C — Model ownership** | Clones A and inserts a synthetic project, Model identity with `origin_project_id=NULL`, and a ModelVersion with a valid `created_in_project_id`. | Preserves the nullable identity origin and non-null version owner exactly. | Current schema / synthetic row variant. |
| **D — Shared legacy Source** | Clones A and adds synthetic `sources`, `segments`, `extracts`, `links`, and `source_revisions`. | The Source record has no project field; SourceVersion table remains empty. No project ownership or SourceVersion is inferred. | Current schema / synthetic row variant. |
| **E — Partial initialization** | Creates only small synthetic `research_projects` and `research_questions` tables, without running application initializers. | Represents a possible incomplete state only; no deployed occurrence is asserted. | Incomplete; must not be repaired or called supported. |
| **F — Unknown version marker** | Clones A and sets SQLite `user_version=99` in the fixture generator. | Deliberately unsupported marker; this does not represent a known released schema. | Unsupported version marker. |

The test-only inventory representation includes full table/column/index/FK
metadata, table row counts, schema fingerprint, SQLite version markers,
migration marker IDs/checksums, and selected ownership fields. Synthetic IDs
and data are fixed. SQLite migration `applied_at_us` values on a freshly
initialized schema are generated by the current initializer and therefore
are not promised byte-for-byte reproducible across separate fixture builds.

## Read-only inventory guarantees

`stage2_0_inventory_tests.rs` opens databases with
`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`, enables `PRAGMA query_only`,
and limits inspection to SQLite metadata PRAGMAs and `SELECT` queries. It
does not call `initialize_workspace_schema`, `initialize_schema`, or domain
migration functions. Tests reject a write attempt, compare a fixture hash
before and after inspection, compare directory entries to detect sidecars,
and verify partial/unsupported fixture hashes remain unchanged.

## Validation and status

Initial focused attempt:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib stage2_0_inventory_tests --no-default-features
```

Result: **blocked before test execution**; `tauri-build` panicked with
`missing cargo:dev instruction`.

Successful focused command:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib stage2_0_inventory_tests
```

Result: **passed**, 6 passed, 0 failed, 238 filtered out. No
code/configuration/dependency workaround was made.

Formatting:

```powershell
rustfmt --edition 2021 src-tauri/src/mind/portable_db.rs src-tauri/src/mind/stage2_0_inventory_tests.rs
```

Result: completed successfully; a separate `rustfmt --check` verification is
also required before closing this stage.

## Limitations and non-claims

- All generated legacy profiles are synthetic; no actual installed or
  historical user database was inspected.
- Passing synthetic fixture tests cannot certify real-world legacy
  compatibility. Real compatibility requires owner-approved, sanitized
  database samples tied to application releases, including populated source
  and research data, varied migration markers, and partial/interrupted
  initialization states.
- No application migration was performed, no existing workspace database
  was opened or modified, and no active selector or settings file was changed.
- No unassigned records were assigned to projects. No Source/project
  associations were created. No archived-write policy was changed.
- No SourceVersion history was manufactured for legacy Source records.
- No Model identity or ModelVersion ownership was changed.
- The six historical Market/Board test failures documented in
  `docs/audit/PHASE_8_INTEGRATION_STATUS.md` remain separate known technical
  debt; they were not rerun or claimed fixed.

## Recommended Stage 2.1 gate

Do not begin ownership assignment or source association until approved
sanitized historical samples are fingerprinted and the supported profile
matrix is explicit. Stage 2.1 should first address migration profile and
backup validation requirements; it should not use these synthetic variants
as evidence of deployed compatibility.
