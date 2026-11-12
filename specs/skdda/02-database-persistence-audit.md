# SKDDA Phase 0 — Database and Persistence Audit

**Purpose:** Record actual database ownership, runtime schema entrypoints, table inventory, constraints, migrations, backup behavior, and unverified risks. This describes the inspected code; it is not a proposed schema.

## 1. Database files and ownership

| Database/storage identity | Creation/open path | Domain ownership | Evidence / qualification |
|---|---|---|---|
| Main workspace SQLite file, default name `vora_data.vora` | Tauri setup in `src-tauri/src/lib.rs` resolves app data directory and opens `vora_data.vora`; legacy `vora_workspace.db` may be copied on first run. `portable_db::migrate_legacy_database_selector` handles legacy path selection. | Mind/canvas, task flow, journal, questions, hypotheses, source, market, Forge index, portable items/snapshots/settings metadata. | `initialize_workspace_schema` invokes all module schema initializers on the same connection. `open_default_db_connection` resolves the same default path for certain Market/CLI commands. |
| Worker queue SQLite file, `vora_worker_queue.db` | `src-tauri/src/bin/vora_worker.rs` constant `WORKER_DB_PATH`; opened by worker queue code. | Worker task queue, heartbeat, cluster logs/events/locks. | Separate operational queue; not SKDDA source of truth. Whether deployed worker runs from a particular working directory/path is runtime-dependent (**UNVERIFIED**). |
| Forge database connection | `forge::index::CodeIndex::new(db_path)` uses a DB path; main-app calls pass `active_database_path`. | Forge tables. | This is not established as another physical DB file in the main app: it is opened on the active `.vora` path. |
| Tick and other datasets | Tick Parquet files in `data/ticks/<SYMBOL>/**/*.parquet`, as specified by `market/local_db.rs`; media/docs by filesystem paths. | External data/assets, not SQLite files. | Backup inclusion/restore of arbitrary referenced assets is **UNVERIFIED**. |
| Trace | JSON graph and Markdown files through Trace service/commands. | Vora Trace. | Not a SQLite DB in the inspected workflow. |
| Test databases | In-memory and temporary files in Rust tests. | Test-only. | No application user DB was opened in this audit. |

**Answer to “how many database files?”** At the code-architecture level, two long-lived SQLite database identities are evident: the main workspace DB and worker queue DB. Forge and Market may open additional connections, but use the main default/active path in the inspected application flow. Tests create additional ephemeral databases. The number and integrity of actual database files on a user's machine is **UNVERIFIED** and was not enumerated/opened to avoid inspecting user data.

## 2. Initialization and connection behavior

`mind::portable_db::initialize_workspace_schema(conn)` calls, in order:

1. `canvas_db::init_db`
2. `task_flow::init_schema`
3. `journal::init_schema`
4. `questions::init_schema`
5. `hypothesis::init_schema`
6. `market::db::init_schema`
7. `source::db::init_schema`
8. `forge::index::CodeIndex::initialize_schema`
9. `init_vora_schema`

The main Tauri setup opens `vora_data.vora`, may copy legacy DB data, applies the schema initializers, enables `PRAGMA foreign_keys = ON`, and sets `journal_mode = DELETE`. It stores one connection in `AppState`. The root `src-tauri/src/schema.sql` is a historical/duplicated SQL schema and is not the sole or complete runtime initializer.

Connection caveat: `CodeIndex::initialize_schema` explicitly sets foreign keys ON. Worker open path sets `journal_mode=WAL` and foreign keys ON. Main Tauri setup enables foreign keys after initialization. However, `open_default_db_connection()` opens a fresh SQLite connection and initializes Market schema without enabling foreign keys itself. Other commands may use `AppState` or open their own connection. Therefore **foreign-key enforcement on every relevant connection is not verified and is not uniformly guaranteed by the shared open helper**.

Source SQL is `src-tauri/schema_source.sql`, included in `source/db.rs`; Market SQL is `src-tauri/schema_market.sql`, included by `market/db.rs`. Mind tables are created by Rust SQL in `canvas_db.rs`, `questions.rs`, `hypothesis.rs`, etc. A single static schema file cannot be used as the current complete database inventory.

## 3. Existing table inventory

Notation: PK primary key; FK foreign key; `?` nullable. Index list is limited to declared SQL indexes identified in inspected schema/audit. This inventory covers the SKDDA-relevant main workspace domains and worker queue, not every transient SQLite object or test fixture.

### 3.1 Main workspace: Mind/canvas and library (14 tables)

Schema: `src-tauri/src/mind/canvas_db.rs`; operations in `mind/canvas_db.rs`, `mind/resolver.rs`, `mind/library_groups.rs`.

| Table | PK / FK and notable indexes | Delete/version semantics and limitation |
|---|---|---|
| `mind_boards` | `id`; optional `active_version_id`, `branch_origin_version_id`, `library_item_group_id`, `parent_board_id`; no declared FKs for these fields in inspected schema. | Mutable board metadata. |
| `mind_board_versions` | `id`; FK `board_id → mind_boards`, `parent_version_id` and `forked_from_version_id` self-reference; history index by board/time. | Snapshot JSON, semantic major/minor/patch, parent/fork lineage. |
| `library_item_groups` | `id`; no SQL FK to board in schema noted. | Mutable current snapshot/version metadata; soft-delete `deleted_at`. |
| `library_item_group_versions` | `id`; FK `group_id → library_item_groups ON DELETE CASCADE`; unique `(group_id, version)`. | Snapshot JSON version history; cascade can remove history if group deleted. |
| `objects` | `id`; `board_id` is stored but no declared FK in schema. | Objects/cards and soft-delete state; current content and version paths are separate. |
| `object_versions` | `id`; FK `object_id → objects`, `parent_version_id → object_versions`. | Immutable-like version content JSON, semantic version tuple; application lifecycle controls changes. |
| `mind_snapshots` | `id`; self-FK `parent_snapshot_id`; `board_id` stored without declared FK in inventory. | Checkpoint/delta snapshots; indexes by parent/checkpoint/depth/board/time. |
| `snapshot_object_changes` | Composite PK `(snapshot_id, object_id)`; FKs to snapshot, object and optional object version. | Delta operation string; operation vocabulary enforcement is application-level. |
| `snapshot_placement_changes` | Composite PK `(snapshot_id, object_id)`; FKs to snapshot/object. | Layout delta state, not scientific relation. |
| `snapshot_link_changes` | Composite PK `(snapshot_id, link_id)`; FK snapshot; endpoint IDs are nullable text, no endpoint FKs. | Canvas links; visual graph state. |
| `checkpoint_objects` | Composite PK `(checkpoint_id, object_id)`; FK checkpoint to snapshots; object-version ID lacks FK in inventory. | Materialized checkpoint. |
| `checkpoint_placements` | Composite PK `(checkpoint_id, object_id)`; FK checkpoint. | Materialized layout state. |
| `checkpoint_links` | Composite PK `(checkpoint_id, link_id)`; FK checkpoint; endpoint IDs lack endpoint FKs. | Materialized canvas edges. |
| `mind_text_links` | `id`; source/target board/card/field and offsets; no declared board/card FKs; checks constrain field and offsets. | Text-range cross-links; not a universal scientific relation. |

Indexes include board/time, parent/checkpoint/depth, object board, version histories, library version lookup, and text-link source/target lookup. Exact details: `docs/audit/RECON.md` §Runtime tables and fields.

### 3.2 Main workspace: source provenance (5 tables)

Schema: `src-tauri/schema_source.sql`; hierarchy migration/indexes: `src-tauri/src/source/db.rs`.

| Table | PK / FK and indexes | Delete/version semantics and limitation |
|---|---|---|
| `sources` | `id`; no FK children directly represented in this table. | Mutable source identity fields and active/archive status; no separate source edition/version table. |
| `segments` | `id`; FK `source_id → sources`, optional self-FK `parent_segment_id`; indexes source, parent, parent/order. | Mutable segment location/notes/status; no source-version FK. |
| `extracts` | `id`; FK `segment_id → segments`, self-FKs `parent_extract_id`, `superseded_by`; indexes segment, parent, superseded. | Transformation kind/content/confidence/review; no general immutable version object. |
| `links` | `id`; FK `extract_id → extracts`; target is `(target_kind,target_id)` text, no FK to heterogeneous target; index extract and target kind/id. | Some relation kinds (`supports`, `contradicts`, evidence/reference) but target integrity is application-level. |
| `source_revisions` | `id`; entity kind/id are untyped text, not FK; entity/time index. | BEFORE UPDATE/DELETE triggers reject changes. Append-only revision log, but not a universal scientific event log. |

### 3.3 Main workspace: task, journal, questions and portable records (7 tables)

| Table | PK / FK and indexes | Semantics / limitation |
|---|---|---|
| `task_flow_items` | `id`; indexes by position and type/position. | Task/AI checklist, not scientific lifecycle. |
| `journal_entries` | `entry_date`; no FK. | Date-keyed text entry. |
| `questions` | `id`; indexes update/create time and status/score. `promoted_to_hypothesis_id` is soft reference. | Triage columns are added idempotently; status CHECK. Delete is supported, including for a question that may have promoted history unless application constraints prevent it. |
| `vora_items` | `id`; type/update index. | Portable generic item data. |
| `vora_snapshots` | `id`; FK `chart_id → vora_items(id) ON DELETE CASCADE`; chart/time index. | Generic chart snapshot JSON; cascade deletion. |
| `vora_imported_items` | Composite PK `(entity_type, entity_id)`; package/time index. | Import bookkeeping; entity ID is polymorphic, no FK. |
| `vora_imported_packages` | `package_id`. | Imported package metadata. |

### 3.4 Main workspace: hypotheses (5 tables)

Schema/migrations: `src-tauri/src/mind/hypothesis.rs`.

| Table | PK / FK and indexes | Semantics / limitation |
|---|---|---|
| `hypotheses` | `id`; `inquiry_id`, `parent_hypothesis_id`, `refined_from_id`, source card/highlight, promotion destination are columns without declared FKs in inspected initializer; indexes inquiry/status+updated. | H0/H1, IV/DV/metric/threshold are text; status CHECK. Several fields are idempotently added by column-inspection migrations. |
| `hypothesis_definitions` | Composite PK `(hypothesis_id, library_group_id)`; FK hypothesis cascade, library group, library version. | Pins linked definition version; exact referential behavior depends on parent table delete action. |
| `hypothesis_insights` | `id`; FK hypothesis with cascade. | Scratchpad records, delete allowed. |
| `hypothesis_data_entries` | `id`; FK hypothesis with cascade; hypothesis/time/id index. | JSON entry records; not typed dataset/sample records and cascade-delete risks historical retention. |
| `hypothesis_snapshots` | Composite PK `(hypothesis_id,snapshot_id)`; no declared FKs in initializer. | Snapshot IDs link to another store; no database-enforced existence or cross-store integrity. |

### 3.5 Main workspace: market tables (7 tables)

Schema: `src-tauri/schema_market.sql`, migration logic in `src-tauri/src/market/db.rs`.

| Table | PK / FK and indexes | Semantics / limitation |
|---|---|---|
| `market_candles` | Composite PK `(symbol,timeframe,ts)`; indexes `(symbol,ts)`, `(timeframe,ts)`; no FKs. | Derived OHLCV data. |
| `market_boxes` | `address`; indexes `(symbol,step)`, `chain_id`; no FKs. | Derived market structures; migration code includes historical column changes and one-time wipes. |
| `_vora_meta` | `key`; no FK. | Migration markers/config metadata. |
| `symbol_origin` | Composite PK `(symbol,step)`; no FK. | Market coordinate/reference data. |
| `scale_registry` | `symbol`; no FK. | JSON registry blob per symbol. |
| `chart_drawing_templates` | `id`, unique case-insensitive name; no FK. | JSON drawing template. |
| `chart_drawing_alerts` | `id`; index symbol/enabled/time; no FK to symbol/drawing. | Alert config/evaluation time, not experiment results. |

### 3.6 Main workspace: Forge tables (16)

Schema in `src-tauri/src/forge/index.rs`; database path is supplied to `CodeIndex`.

| Table | PK / FK and notable indexes | Semantics / limitation |
|---|---|---|
| `workspaces` | `workspace_id`; root path. | Code-index workspace, not research project. |
| `snapshots` | `snapshot_id`; FK workspace and optional self-parent. | Code index snapshots. |
| `files` | `file_id`; FK workspace. | Indexed files. |
| `file_versions` | `file_version_id`; FK file and optional self-parent. | Content hash and lineage. |
| `snapshot_files` | Composite PK `(snapshot_id,file_id)`; unique snapshot/path; FKs snapshot/file/version. | File set at code snapshot. |
| `symbol_lineages` | `lineage_id`; FKs workspace and snapshots. | Code symbol lineage. |
| `symbol_lineage_versions` | Composite PK `(lineage_id,snapshot_id)`; FKs lineage/snapshot. | Versioned symbol identity. |
| `canonical_entities` | `entity_id`; FKs workspace/lineage. | Code intelligence canonical symbol. |
| `canonical_entity_versions` | Composite PK `(entity_id,snapshot_id)`; FKs entity/snapshot/lineage. | Canonical symbol history. |
| `forge_audit_trail` | Autoincrement ID; FK workspace; task index. | Agent execution/change audit. |
| `code_files` | `file_id`; unique file path; no FK to spine files in schema. | Semantic index. |
| `code_symbols` | `id`; FK `file_id ON DELETE CASCADE`; indexes file/namespace/name/lineage. | Parsed symbol index. |
| `code_scopes` | `scope_id`; FK file with cascade; file index. | Parsed scopes. |
| `code_imports` | Autoincrement ID; optional file reference via path FK where declared. | Imports. |
| `code_references` | Autoincrement ID; path FK with cascade where declared. | Resolved references. |
| `code_raw_references` | Autoincrement ID; FK file with cascade. | Unresolved/raw refs. |

Exact field lists and indexes are recorded in [RECON.md](../../docs/audit/RECON.md). These tables are not part of SKDDA research ownership.

### 3.7 Worker database tables (5)

Defined in `src-tauri/src/bin/vora_worker.rs`; separate `vora_worker_queue.db`.

| Table | PK / FK / index | Semantics |
|---|---|---|
| `worker_tasks` | Autoincrement ID; unique `task_id`; status/priority/time index; no declared FK. | Work queue, status strings. |
| `worker_heartbeats` | `worker_id`; no FK. | Worker liveness and concurrency. |
| `cluster_ast_events` | Autoincrement ID; unique event ID; no FK. | Worker events. |
| `cluster_file_locks` | `file_path`; no FK. | Coordination locks. |
| `cluster_logs` | Autoincrement ID; descending time index; no FK. | Operational logs. |

## 4. Relationships and deletion safety

Strong relational constraints exist in selected source, canvas, library and Forge tables, but are not universal. Examples of weak or soft references relevant to future SKDDA:

- `links.target_kind/target_id` has no heterogeneous target FK.
- Question-to-hypothesis and hypothesis-to-inquiry/source/canvas snapshot links are partly application-level or unreferenced columns.
- Canvas snapshot edges and checkpoint edges persist endpoint IDs without endpoint FK constraints.
- Multiple board/object parent or owner identifiers are columns without SQL FK enforcement in the inspected schema.
- `hypothesis_data_entries` and `hypothesis_insights` cascade on hypothesis delete; this is acceptable for current subordinate UI data semantics but unsafe for immutable scientific history without a policy change.
- Library group version history cascades when a group is deleted.
- Market tables mostly have no FKs.

`PRAGMA foreign_keys = ON` is connection-scoped; setting it on the primary AppState connection does not automatically configure newly opened connections. The default helper’s omission is a concrete audit finding. The future database layer must centralize open/configure behavior and test `PRAGMA foreign_keys` on all durable command paths.

## 5. Migration, backup and rollback

### Current implementation

- No common numbered migration registry or `PRAGMA user_version` convention was established.
- Modules create tables with `CREATE TABLE IF NOT EXISTS`, inspect `PRAGMA table_info`, and apply `ALTER TABLE` statements.
- Market uses `_vora_meta` markers and includes historical transformations; several operations intentionally swallow errors (`let _ = ...`) including compatibility ALTER attempts. A one-time marker can delete derived `market_boxes`.
- Questions and hypotheses have idempotent additive-column migration loops.
- Source hierarchy adds missing columns/indexes idempotently.
- Workspace legacy DB selector migration uses SQLite backup API into a staged sibling then rename; conflict detection avoids overwriting an existing fixed database.
- Export uses SQLite backup API to a temporary file and atomic-ish rename/restore of an existing destination.
- Import stages a copied backup, validates `PRAGMA quick_check` and workspace-table presence, initializes schema, then replaces active files with rollback logic. `PRAGMA foreign_key_check` on imported data was not established.
- Settings are a separate `.settings.json` file associated with the DB path.

### Assessment

Idempotency exists in multiple module initializers but cannot be treated as a coordinated migration guarantee. There is no single ordered migration ledger, explicit supported-from version matrix, or uniform all-module transaction. A failure during startup can occur after earlier modules have already applied changes. Existing backup/import paths are valuable but do not establish inclusion of external assets or full end-to-end recovery. Do not execute migration/backup/import during this audit.

## 6. SKDDA database impact

Preserve all existing workspace tables and IDs. Extend source, question, hypothesis and framework integration only through explicit compatibility migrations. Add new tables for project/domain/gap, source version/citation where needed, claims/concepts/principles/framework versions, learning, phenomenon/observations, experiment designs/runs, datasets/versions/samples, analyses/evidence/findings, model versions/evaluations, validation/replication, typed relations, transfers, and provenance events. The final grouping must be settled in Phase 0 schema design; this is a logical list, not production SQL.

Do not treat board snapshots, Trace JSON, chart snapshots, or a duplicate scientific database as the durable record. Store canonical records in the workspace DB; large immutable data can live in content-addressed workspace assets with a manifest/checksum.

## 7. References and limitations

Primary evidence: `src-tauri/src/lib.rs`, `mind/portable_db.rs`, `mind/canvas_db.rs`, `mind/questions.rs`, `mind/hypothesis.rs`, `source/db.rs`, `schema_source.sql`, `market/db.rs`, `schema_market.sql`, `forge/index.rs`, `bin/vora_worker.rs`, and [RECON.md](../../docs/audit/RECON.md).

**UNVERIFIED:** exact real-world DB count/location for installed workspaces; file integrity; all connection entrypoints beyond inspected application and CLI paths; backup of external files; and every existing workspace schema shape. No live database was inspected or modified.

## Phase 8 verified addendum — 2026-11-12

The preceding inventory is the Phase 0 baseline, not a current table-by-table
inventory of the later scientific modules. Current runtime initialization was
rechecked at `mind::portable_db::initialize_workspace_schema`: it includes
Source, Research, Experiment, Analysis, Model, Learning, and Knowledge
Transfer in a fixed sequence alongside the pre-existing subsystems. Each
scientific module retains its own migration code; this does not establish a
single atomic all-domain migration.

An isolated fresh-workspace test now verifies repeated initialization,
`PRAGMA foreign_keys = 1`, `PRAGMA integrity_check = 'ok'`, and an empty
`PRAGMA foreign_key_check`. It does not certify older deployed schemas,
interrupted startup, user data, or external asset recovery. See the
[Phase 8 integration status](../../docs/audit/PHASE_8_INTEGRATION_STATUS.md)
for the commands, full test results, and remaining gaps.
