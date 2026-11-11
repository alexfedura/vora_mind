# Vora Existing Data and Schema Inventory

**Baseline:** Repository revision `b36fbd4`, audited 2026-11-11.  
**Scope:** Schema definitions and persistence paths found in source. This inventory does not inspect a live user database, filesystem assets, or deployed schema variants.

## 1. Persistence roots and ownership

| Store | Location/owner | Contents and limits |
|---|---|---|
| Main workspace SQLite | Tauri AppData `vora_data.vora`; shared `rusqlite::Connection` in app state. A legacy `vora_workspace.db` is copied into the fixed location if needed. | Current workspace domain tables. Schema is composed by Rust module initializers and included SQL files. |
| Workspace settings | Sibling `.settings.json`; older `vora_settings` table also appears in compatibility handling. | Settings are separate from regular workspace rows and can be separately imported/exported. |
| Legacy workspace selector | App config `database.json`. | Read for migration to the fixed DB path; not evidence that current runtime supports arbitrary active workspace DB selection. |
| Worker queue SQLite | Relative `vora_worker_queue.db` resolved by the worker process. | Operational task/worker coordination; separate connection, process and schema. Actual deployed working directory is unknown. |
| Vora Trace | AppData filesystem files: project registry YAML, graph JSON and decision Markdown, with legacy-copy handling. | Not a SQLite domain. Trace graph “project,” “experiment,” and “evidence” fields are not target scientific entities. |
| Media/source assets | Filesystem paths referenced by DB rows; file metadata and hashes stored in SQLite. | A database backup alone does not establish that referenced media/document files are included or recoverable. |
| Market tick data | Parquet files under a resolved `data/ticks/<symbol>` directory; read by DuckDB. | Separate from SQLite `market_candles`; tick-file inventory, licensing, packaging and backup coverage are unknown. |

## 2. Runtime main-database table inventory

The current initializer composition yields **58 named main-workspace tables** in these groups: canvas/document/media 18; hypothesis 5; task flow/journal/questions/portable package records 7; source 5; market 7; Forge 16. The count excludes Trace files, worker DB tables, settings sidecars, temporary/staging tables, and SQL tables that are dropped by initialization. It describes current source initializers, not every deployed user database.

### 2.1 Canvas, documents and media — 18 tables

Defined primarily by [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs) with incremental column checks. Columns below are declared by the current initializer; columns explicitly noted as migration-added are also part of the current runtime schema.

| Table | Declared columns / purpose | Relationships and notable indexes |
|---|---|---|
| `mind_boards` | `id`, `title`, `created_at`, `updated_at`, `active_version_id`, `branch_origin_version_id`, `library_item_group_id`, `parent_board_id` | Active/fork/library IDs are not declared FKs. Partial unique index on non-null `library_item_group_id`. |
| `mind_board_versions` | `id`, `board_id`, `version_major`, `version_minor`, `version_patch`, `parent_version_id`, `forked_from_version_id`, `snapshot_json`, `reason`, `created_at` | FKs to board and parent/fork version; history index `(board_id, created_at, id)`. |
| `library_item_groups` | `id`, `name`, `version`, `snapshot_json`, `item_type`, `created_at`, `updated_at`, `deleted_at` | `item_type` is checked as `item` or `group`; runtime migration adds it for older DBs. |
| `library_item_group_versions` | `id`, `group_id`, `version`, `snapshot_json`, `created_at` | FK to group with `ON DELETE CASCADE`; unique `(group_id, version)`. |
| `objects` | `id`, `type`, `board_id`, `created_at`, `updated_at`, `deleted_at`, `metadata`, `review_status`; migration adds `working_content` | `review_status` checked against `active`, `completed`, `not_needed`; FK to board is not declared. Indexes on board and review status. |
| `vora_files` | `id`, `original_name`, `current_path`, `file_type`, `file_hash`, `size_bytes`, `modified_time_ms`, `created_at`, `duration_seconds`, `last_position_seconds`, `last_used_at` | File identity/hash/path data; references external file system. |
| `documents` | `id`, `title`, `file_path`, `file_id`, `file_type`, `page_count`, `created_at`, `updated_at` | Unique `file_path`; FK `file_id` to `vora_files`; file type check `pdf`/`docx`/`md`. |
| `document_highlights` | `id`, `document_id`, `content`, `page_number`, `position_data`, `color`, `note`, `library_card_id`, `created_at` | FK to document with cascade; index `(document_id, created_at)`. `library_card_id` is soft. |
| `object_versions` | `id`, `object_id`, `version_major`, `version_minor`, `version_patch`, `parent_version_id`, `reason`, `summary`, `content`, `created_at` | FKs to object and parent object version. |
| `mind_snapshots` | `id`, `name`, `parent_snapshot_id`, `board_id`, `depth`, `replay_cost`, `is_checkpoint`, `created_at` | `board_id` is added for old DBs by `ensure_board_column`; parent FK. Indexes on parent, checkpoint, depth, and `(board_id, created_at)`. |
| `snapshot_object_changes` | `snapshot_id`, `object_id`, `operation`, `object_version_id` | Composite PK `(snapshot_id, object_id)`; FKs to snapshot and object; `object_version_id` is soft. |
| `snapshot_placement_changes` | `snapshot_id`, `object_id`, `operation`, `x`, `y`, `width`, `height`, `rotation`, `z_index`, `style` | Composite PK `(snapshot_id, object_id)`; FKs to snapshot and object. |
| `snapshot_link_changes` | `snapshot_id`, `link_id`, `operation`, `source_object_id`, `target_object_id`, `link_type` | Composite PK `(snapshot_id, link_id)`; FK only to snapshot; endpoint IDs are soft. |
| `checkpoint_objects` | `checkpoint_id`, `object_id`, `object_version_id` | Composite PK `(checkpoint_id, object_id)`; FK only to checkpoint. |
| `checkpoint_placements` | `checkpoint_id`, `object_id`, `x`, `y`, `width`, `height`, `rotation`, `z_index`, `style` | Composite PK `(checkpoint_id, object_id)`; FK only to checkpoint. |
| `checkpoint_links` | `checkpoint_id`, `link_id`, `source_object_id`, `target_object_id`, `link_type` | Composite PK `(checkpoint_id, link_id)`; FK only to checkpoint. |
| `mind_text_links` | `id`, source board/card/field/start/end, target board/card/field/start/end, `created_at`, `updated_at` | Range/field checks; indexes for source and target positions. Board/card endpoints are soft. |
| `mind_media_slices` | `id`, `file_id`, `title`, `start_seconds`, `end_seconds`, `note`, `created_at`, `updated_at` | FK to `vora_files` with cascade; nonnegative ordered time range checks. |

The initializer also inserts initial Library versions and can create boards/versions for legacy Library groups inside a savepoint. This is a compatibility transform, not a scientific entity conversion.

### 2.2 Research-adjacent Mind tables

Defined by [task_flow.rs](../../src-tauri/src/mind/task_flow.rs), [journal.rs](../../src-tauri/src/mind/journal.rs), [questions.rs](../../src-tauri/src/mind/questions.rs), and [hypothesis.rs](../../src-tauri/src/mind/hypothesis.rs).

| Table | Columns | Relationships/indexes and interpretation |
|---|---|---|
| `task_flow_items` | `id`, `title`, `description`, `task_type`, `completed`, `position`, `created_at`, `updated_at` | `task_type` checked as `task`/`ai`; completed is 0/1. Position/type indexes. `description` and `task_type` are migration-added for old DBs. |
| `journal_entries` | `entry_date`, `content`, `created_at`, `updated_at` | `entry_date` PK in ISO date format; no scientific event/entity link. |
| `questions` | `id`, `title`, `description`, `status`, `impact`, `feasibility`, `urgency`, `priority_score`, `triage_notes`, `triaged_at`, `is_selected_for_promotion`, `promoted_to_hypothesis_id`, `created_at`, `updated_at` | `status` checked against `parked`/`triaged`/`archived`/`promoted`; index by update and `(status, priority_score, updated_at)`. Promoted ID is not an FK. Triage columns are added to the earlier five-column table by initializer. |
| `hypotheses` | `id`, `inquiry_id`, `statement_h1`, `statement_h0`, `independent_var`, `dependent_var`, `metric`, `success_threshold`, `status`, `created_at`, `updated_at`, `evidence_notes`, `verdict`, `evaluated_at`, `promoted_to_library_id`, `parent_hypothesis_id`, `protocol_steps`, `protocol_locked_at`, `raw_data`, `data_analysis`, `executed_at`, `boundary_conditions`, `refined_from_id`, `checklist_schema`, `protocol_type`, `protocol_flowchart_json`, `protocol_completed_steps`, `source_card_id`, `source_highlight_id` | Indexes on `inquiry_id` and status/update. Status check is `draft`/`preregistered`/`testing`/`supported`/`rejected`. Most lineage and card/library IDs are soft references. Extra columns are added incrementally to older tables. |
| `hypothesis_definitions` | `hypothesis_id`, `library_group_id`, `library_version_id`, `definition_name`, `version_string` | Composite PK `(hypothesis_id, library_group_id)`; FKs to hypothesis, Library group and Library version; hypothesis delete cascades. |
| `hypothesis_insights` | `id`, `hypothesis_id`, `content`, `created_at` | FK to hypothesis with cascade. |
| `hypothesis_data_entries` | `id`, `hypothesis_id`, `data_json`, `created_at` | FK to hypothesis with cascade; index `(hypothesis_id, created_at, id)`. JSON is not a typed observation/dataset contract. |
| `hypothesis_snapshots` | `hypothesis_id`, `snapshot_id`, `created_at` | Composite PK; no declared FKs. |

### 2.3 Portable package tables — 4 tables

Defined in the `PORTABLE_SCHEMA` of [portable_db.rs](../../src-tauri/src/mind/portable_db.rs).

| Table | Columns | Relationships/indexes |
|---|---|---|
| `vora_items` | `id`, `type`, `name`, `data`, `created_at`, `updated_at` | Index `(type, updated_at DESC)`. |
| `vora_snapshots` | `id`, `chart_id`, `name`, `data`, `created_at` | FK `chart_id` to `vora_items(id)` with cascade; index `(chart_id, created_at DESC)`. |
| `vora_imported_items` | `entity_type`, `entity_id`, `display_name`, `source_package_id`, `source_package_name`, `imported_at` | Composite PK `(entity_type, entity_id)`; source package ID is soft; source index. |
| `vora_imported_packages` | `package_id`, `package_name`, `imported_at` | Package ID PK. |

The settings compatibility code can create a legacy `vora_settings(key, value)` table, but current settings are primarily a sidecar JSON file. A test creates an `authored_data` table only in an in-memory backup test; it is not part of the production schema.

### 2.4 Source provenance — 5 tables

Defined by [schema_source.sql](../../src-tauri/schema_source.sql) and initialized/migrated in [source/db.rs](../../src-tauri/src/source/db.rs).

| Table | Columns | Relationships/indexes |
|---|---|---|
| `sources` | `id`, `kind`, `path`, `title`, `author`, `language`, `duration_sec`, `page_count`, `metadata_json`, `notes`, `status`, `created_at`, `updated_at` | PK `id`; no schema-enforced source-version identity. |
| `segments` | `id`, `source_id`, `label`, `start_sec`, `end_sec`, `page_start`, `page_end`, `line_start`, `line_end`, `notes`, `status`, `created_at`, `updated_at`, `parent_segment_id`, `order_index` | FK to source and self-parent; indexes source. |
| `extracts` | `id`, `segment_id`, `transform_kind`, `content`, `language`, `confidence`, `created_by`, `reviewed_by_user`, `parent_extract_id`, `superseded_by`, `notes`, `status`, `created_at`, `updated_at` | FK to segment and self-links; indexes segment, parent, superseded. |
| `links` | `id`, `extract_id`, `target_kind`, `target_id`, `link_kind`, `note`, `status`, `created_at`, `updated_at` | FK to extract; target is intentionally polymorphic string pair with no target FK; indexes extract and target kind/ID. |
| `source_revisions` | `id`, `entity_kind`, `entity_id`, `revision_type`, `before_json`, `after_json`, `reason`, `created_by`, `created_at` | Entity pair is soft; index `(entity_kind, entity_id, created_at)`. Triggers reject UPDATE/DELETE on revision rows. |

IDs in SQL comments show examples like `SRC-0001`; runtime/database state must be inspected to confirm formats actually present. Revision triggers protect revision rows but do not by themselves make the domain mutation plus revision insert atomic.

### 2.5 Market tables — 7 current named tables

Defined by [schema_market.sql](../../src-tauri/schema_market.sql), with migrations in [market/db.rs](../../src-tauri/src/market/db.rs).

| Table | Columns | Relationships/indexes |
|---|---|---|
| `market_candles` | `symbol`, `timeframe`, `ts`, `open`, `high`, `low`, `close`, `volume`, `tick_count`, `first_extreme` | Composite PK `(symbol, timeframe, ts)`; indexes `(symbol, ts)` and `(timeframe, ts)`. `ts` documented as Unix seconds/UTC. |
| `market_boxes` | `address`, `symbol`, `step`, `pivot_step`, `pattern`, `zone`, `scale`, `x`, `y_abs`, `z`, `y_band`, `x_idx`, `ts_start`, `ts_end`, `high`, `low`, `origin_price`, `engulf_ts`, `duration_sec`, `scale_sec`, `chain_id`, `chain_pos` | Indexes `(symbol, step)` and `chain_id`; no FKs. |
| `_vora_meta` | `key`, `value` | Migration markers; PK key. |
| `symbol_origin` | `symbol`, `step`, `price_ref`, `price_ref_ts`, `computed_at` | Composite PK `(symbol, step)`. |
| `scale_registry` | `symbol`, `registry_json`, `updated_at` | `symbol` PK. |
| `chart_drawing_templates` | `id`, `name`, `drawing_type`, `template_json`, `created_at` | Case-insensitive unique name. |
| `chart_drawing_alerts` | `id`, `symbol`, `timeframe`, `drawing_id`, `condition`, `message`, `trigger_once`, `notify_app`, `notify_toast`, `notify_email`, `notify_sound`, `expiration_at`, `enabled`, `created_at`, `last_evaluated_at` | Index `(symbol, enabled, created_at DESC)`; alert columns are migrated if absent. |

`market_sources` is not a current durable table: the runtime initializer explicitly drops it. The Market migration code also performs marker-controlled deletion of derived `market_boxes`, and some legacy column drop/add and cleanup statements suppress SQL errors. These behaviors must be reviewed against user DB fixtures before any analogous migration strategy is adopted for scientific data.

### 2.6 Forge tables — 16 current named tables

Defined by `IDENTITY_SPINE_SCHEMA` in [forge/index.rs](../../src-tauri/src/forge/index.rs). These represent code-workspace indexing and code-entity lineage, not scientific knowledge lineage.

| Table | Columns | Relationships/indexes |
|---|---|---|
| `workspaces` | `workspace_id`, `root_path`, `created_at`, `current_snapshot_id` | PK workspace; current snapshot is soft. |
| `snapshots` | `snapshot_id`, `workspace_id`, `parent_snapshot_id`, `created_at` | FKs to workspace and parent; workspace index. |
| `files` | `file_id`, `workspace_id`, `created_at`, `deleted_at` | FK workspace; workspace index. |
| `file_versions` | `file_version_id`, `file_id`, `content_hash`, `byte_size`, `created_at`, `parent_file_version_id` | FKs file and parent version; file/hash indexes. |
| `snapshot_files` | `snapshot_id`, `file_id`, `path_at_snapshot`, `file_version_id` | Composite PK snapshot/file; unique snapshot/path; FKs to all three parents; snapshot/path indexes. |
| `symbol_lineages` | `lineage_id`, `workspace_id`, `created_snapshot_id`, `retired_snapshot_id` | FKs to workspace/snapshots. |
| `symbol_lineage_versions` | `lineage_id`, `snapshot_id`, `symbol_id`, `confidence` | Composite PK lineage/snapshot; FKs lineage/snapshot; indexes symbol/snapshot. |
| `canonical_entities` | `entity_id`, `workspace_id`, `lineage_id`, `initial_name`, `created_at` | FKs workspace/lineage; indexes workspace/lineage. |
| `canonical_entity_versions` | `entity_id`, `snapshot_id`, `symbol_id`, `lineage_id`, `canonical_signature`, `canonical_namespace`, `canonical_name`, `created_at` | Composite PK entity/snapshot; FKs entity/snapshot/lineage; entity/snapshot indexes. |
| `forge_audit_trail` | `id`, `task_id`, `workspace_id`, `intent`, `target_symbol`, `target_file`, `base_snapshot_id`, `committed_snapshot_id`, `healing_attempts`, `status`, `error_payload`, `created_at` | FK workspace; task index. |
| `code_files` | `file_id`, `file_path`, `content_hash`, `lineage_parent_id`, `last_indexed_at` | Unique file path; indexes path/hash. |
| `code_symbols` | `id`, `file_id`, `module_namespace`, `file_path`, `name`, `kind`, `scope_path`, `qualified_scope`, `signature`, `canonical_signature`, `body_hash`, `visibility`, `lineage_id`, `start_line`, `end_line`, `start_byte`, `end_byte` | FK `file_id` to code files with cascade; indexes file, namespace, name, lineage. |
| `code_scopes` | `scope_id`, `file_id`, `parent_scope_id`, `visible_bindings_json` | FK file with cascade; file index. |
| `code_imports` | `id`, `file_id`, `file_path`, `source_module`, `imported_name`, `alias`, `line` | FK path to `code_files(file_path)` with cascade. |
| `code_references` | `id`, `file_path`, `caller_scope`, `callee_name`, `line`, `byte_offset` | FK path to code files with cascade; callee index. |
| `code_raw_references` | `id`, `file_id`, `scope_id`, `caller_symbol_id`, `callee_raw_name`, `receiver_name`, `is_member_access`, `access_chain_json`, `ref_kind`, `byte_start`, `byte_end`, `line` | FK file with cascade; indexes callee and scope. |

### 2.7 Tables outside the main workspace database

The worker queue schema in [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs) declares **5 tables**: `worker_tasks` (`id`, `task_id`, `project_root`, `intent`, `target_file`, `target_symbol`, `patch_code`, `priority`, `status`, `claimed_by`, `healing_attempts`, `last_error_stage`, `error_payload`, `created_at`, `started_at`, `finished_at`); `worker_heartbeats` (`worker_id`, `last_seen`, `current_task_id`, `status`, `version`, `port`, `active_tasks`, `max_concurrency`); `cluster_ast_events` (`id`, `event_id`, `source_node`, `file_path`, `symbol_id`, `action`, `timestamp`); `cluster_file_locks` (`file_path`, `locked_by_task`, `locked_by_node`, `locked_at`); and `cluster_logs` (`id`, `node_id`, `task_id`, `level`, `message`, `timestamp`). Indexes cover worker task status/priority/creation and descending log timestamp. No scientific experiment/run contract is present.

Trace DTOs in [vora_trace.rs](../../src-tauri/src/vora_trace.rs) are serialized files, not SQL rows. A graph contains a schema version, project string, nodes and edges. Node fields include ID, kind (`state`, `decision`, `experiment`, `rejected`), title/status, optional objective/reason/timestamps/parent/commit links, alternatives, evidence strings and next IDs. A project registry has root and project entries with path/role/status/description and Git metadata. Relationships remain graph-string references; they are not SQLite FKs.

## 3. Relationship, constraints, and identity observations

- FKs are declared for selected canvas, source, hypothesis child, portable snapshot, and Forge relations. Many significant IDs remain strings without FKs: question-to-hypothesis, hypothesis inquiry/parent/library promotion/source card, source revision entity, source link target, canvas cards/text-link endpoints/snapshot links, Trace graph references, and portable package source.
- SQLite enforcement is connection-local. Canvas initializer sets WAL, synchronous NORMAL, foreign keys ON and temp store MEMORY; app startup also configures the shared connection. The separate `open_default_db_connection()` path does not apply the same foreign-key pragma.
- Most main-domain timestamps are integer values, but units are not globally normalized. Canvas and some Mind code use milliseconds; source operations include seconds-based timestamps; Market schema documents Unix seconds; Trace uses ISO/string values. Asset modification time is explicitly milliseconds. Each domain must be examined before conversion.
- IDs are heterogeneous: UUID-like IDs, domain-prefixed source examples, numeric autoincrement worker/Forge records, composite natural keys, and opaque strings. Existing ID values must be preserved unless an approved mapping and alias strategy exists.
- Deletion policies vary: selected child rows cascade; many relationships have no FK; visual/history records have independent lifecycles; source revision rows are protected by triggers. No universal scientific retention/archive policy is implemented.
- JSON/text blobs include board snapshots/content, metadata, hypothesis protocol/checklist/data entries, market registries/alerts and import package payloads. Their schemas are not generally versioned at the column level.

## 4. Database initialization and schema-version evidence

`initialize_workspace_schema` invokes module initializers in sequence. Most are idempotent-ish `CREATE TABLE IF NOT EXISTS` plus ad-hoc column additions. The canvas initializer includes savepoint-backed Library-to-board compatibility insertion. Questions, hypotheses, task flow, media and Market each implement their own migration logic. No single numbered migration table or `PRAGMA user_version` ledger was found.

`src-tauri/src/schema.sql` is an additional old/duplicated SQL source and is not included by runtime workspace initialization. The actual included SQL paths are `src-tauri/schema_source.sql` and `src-tauri/schema_market.sql`. In particular, do not inventory a deployed database from the duplicate file alone. `market_sources` is explicitly dropped during market initialization, and derived `market_boxes` may be cleared once per marker.

The source establishes the initializer’s current shape, not the current contents of user databases. It cannot prove which migrations ran, whether a previous partial boot left mixed columns, whether an older binary has divergent schema, or what rows/paths exist in an installation.

## 5. Backup and external data completeness

SQLite online backup copies SQLite rows. Import validates staged DBs with `quick_check` and required-table checks and initializes the imported DB before replacing active workspace files, with rollback logic for DB/settings files. No whole-workspace manifest or evidence of complete media/source/Trace/Parquet backup is established. `foreign_key_check`, application-level ID/link validation, asset hash verification and worker DB state inclusion were not observed in the inspected validation path.

An approved backup fixture should include: all 58 main tables at realistic shape; source/media references and missing-file cases; old question/hypothesis and canvas schemas; Market marker states; settings sidecar; Trace files; and worker DB as applicable. Preserve each original fixture and restore into a separate temporary directory.

## 6. Coverage and evidence limits

Rust tests exist in source modules for questions, hypothesis, source, canvas/persistence, Market, portable import/backup, and Forge/worker scenarios. The test sources were inspected but **not executed**. No frontend test files or frontend test script were found in the inspected project manifests and source. No live SQLite database was inspected, so this is not a row-level data inventory or deployed schema certification.
