# Vora Mind repository architecture audit

**Scope:** read-only inspection of the repository at the time of this report. Facts are marked **FACT** and cited to source files; judgments or recommendations are marked **INFERENCE**. `UNKNOWN` means repository evidence did not establish the answer.

## 1. SNAPSHOT

### What the app does

- **FACT:** Vora Mind is described as a local-first Tauri desktop workspace; its UI combines a visual board/canvas, charting and snapshots, source/provenance management, task flow, journaling, questions, Vora Trace, and code-analysis/Forge operations. See [README.md](../../README.md), [src/main.tsx](../../src/main.tsx#L1-L65), and [src/App.tsx](../../src/App.tsx#L2490-L2528).
- **INFERENCE:** The product is a single-user knowledge workspace with market-research and code-workspace capabilities, backed primarily by a local SQLite database. It also has an independently runnable worker service; this is not a frontend-only or backend-free repository.

### Stack and versions

| Area | FACT |
|---|---|
| Frontend | React `^19.1.0`, TypeScript `~5.8.3`, Vite `^7.0.4`, React Router `^7.18.4`, Zustand `^5.0.15`; package versions are manifest constraints, not necessarily exact installed versions. [package.json](../../package.json#L1-L70) |
| Chart/canvas | `lightweight-charts ^5.2.1`, `@xyflow/react ^12.11.3`. [package.json](../../package.json#L25-L56) |
| Desktop/backend | Tauri v2 (`tauri = "2"`), Rust edition 2021, SQLite through `rusqlite = "0.32"`, DuckDB `1.0` with bundled/parquet features. [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L1-L55) |
| Package manager | pnpm `12.6.0` is pinned in `package.json`; `pnpm-workspace.yaml` contains build allowlisting but no package globs. [package.json](../../package.json#L67-L71), [pnpm-workspace.yaml](../../pnpm-workspace.yaml) |
| Repository shape | One frontend package and one Cargo package/workspace, plus a Rust worker binary and source CLI. No JS multi-package workspace is declared. [package.json](../../package.json#L1-L5), [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L1-L8), [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1774-L1787) |
| Locking | `pnpm-lock.yaml` exists; Cargo lock files are ignored by [.gitignore](../../.gitignore#L1-L11). A deterministic Rust dependency lock is therefore not guaranteed by the tracked repository. |

### Commands

| Purpose | Command / finding |
|---|---|
| Install | `pnpm install --frozen-lockfile` (also used in CI). [package.json](../../package.json#L1-L24), [release.yml](../../.github/workflows/release.yml#L25-L40) |
| Desktop development | `pnpm dev` or `pnpm tauri:dev`; Vite-only: `pnpm dev:frontend`. [package.json](../../package.json#L6-L16) |
| Build | `pnpm build` (runs `tsc && vite build`); desktop bundle: `pnpm tauri:build`. [package.json](../../package.json#L6-L18), [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L1-L15) |
| Preview | `pnpm preview`. [package.json](../../package.json#L10-L14) |
| Tests | `cargo test --manifest-path src-tauri/Cargo.toml`; a focused DuckDB test can be run with `cargo test --manifest-path src-tauri/Cargo.toml --lib market::local_db::tests`. `package.json` has no frontend test script/framework. [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L58-L100), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L122-L220) |
| Type-check | `pnpm exec tsc --noEmit` (the build script also type-checks). [tsconfig.json](../../tsconfig.json#L1-L31), [package.json](../../package.json#L6-L14) |
| Lint / format | No package lint/format script or linter config was found. Rust formatting can be run with `cargo fmt --manifest-path src-tauri/Cargo.toml`; this is not wired to CI. **UNKNOWN:** whether developers use additional untracked local tooling. |
| Migrate | No standalone migration command or migration framework is declared. Schema setup/migrations execute during startup. [src-tauri/src/mind/portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891), [src-tauri/src/market/db.rs](../../src-tauri/src/market/db.rs#L1-L120), [src-tauri/src/source/db.rs](../../src-tauri/src/source/db.rs#L8-L40) |
| Worker | `cargo run --manifest-path src-tauri/Cargo.toml --bin vora_worker -- serve 4040`; supported CLI modes include `health`, `doctor`, `status`, `submit`, `once`, and `run`. [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1774-L1810) |

### Environment variables

| Name | Where read / configured | Meaning evidenced in repo |
|---|---|---|
| `APPDATA`, `LOCALAPPDATA`, `XDG_DATA_HOME`, `HOME` | [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1438-L1465) | Fallback bases for standalone/default SQLite path resolution. |
| `TAURI_DEV_HOST` | [vite.config.ts](../../vite.config.ts#L1-L33) | Optional dev server host/HMR setting. |
| `WORKER_NODE_ID`, `WORKER_MAX_CONCURRENCY`, `WORKER_MODE`, `WORKER_AUTH_TOKEN` | [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L180-L210) | Worker identity, concurrency, local/remote mode, and bearer-token auth. Remote mode requires the token. |
| `GIT_TOKEN`, `GITHUB_TOKEN`, `LLM_API_KEY`, `OPENAI_API_KEY` | [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L638-L650) | Worker doctor checks whether these optional integrations are configured; exact runtime usage is not established by that check alone. |
| `VORA_DEBUG_BOXES`, `VORA_DEBUG_PIVOTS` | [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L198-L220), [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L493-L530) | Enable market diagnostic logging. |
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `GITHUB_TOKEN` | [release.yml](../../.github/workflows/release.yml#L40-L68) | Release workflow secrets; signing secrets are required by the release job. |
| `WORKER_PORT`, `RUST_LOG`, `RUST_BACKTRACE`, `WORKER_MODE`, `WORKER_NODE_ID`, `WORKER_MAX_CONCURRENCY` | [Dockerfile.worker](../../Dockerfile.worker#L30-L48), [vora-worker.service](../../deploy/systemd/vora-worker.service#L1-L20) | Deployment environment defaults/configuration; the worker's `serve` CLI obtains its port from an argument. |

## 2. ARCHITECTURE MAP

### Top-level folders

| Path | Responsibility (FACT) |
|---|---|
| [src/](../../src) | React application, feature modules, IPC wrappers, shared stores/components. [src/main.tsx](../../src/main.tsx#L1-L65) |
| [src-tauri/](../../src-tauri) | Tauri host, Rust commands, SQLite/DuckDB logic, schemas, worker and CLI binaries. [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1263-L1437), [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L1-L100) |
| [data/](../../data) | Local tick-data location; XAUUSD README specifies nested Parquet inputs. [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md) |
| [docs/](../../docs) | Product/runtime/market documentation; no audit report existed at inspection time. [docs/](../../docs) |
| [specs/](../../specs) | Feature/specification documents; repository guidance says major feature work should be spec-first. [AGENTS.md](../../AGENTS.md) |
| [scripts/](../../scripts) | Release/version synchronization utilities, invoked by package scripts. [package.json](../../package.json#L17-L24) |
| [deploy/](../../deploy) | Worker deployment configuration, including systemd. [deploy/systemd/vora-worker.service](../../deploy/systemd/vora-worker.service) |
| [.github/workflows/](../../.github/workflows) | Release workflow; no test/CI workflow found. [release.yml](../../.github/workflows/release.yml) |
| [public/](../../public) | Static frontend assets. [vite.config.ts](../../vite.config.ts#L7-L24) |

### Layers and communication

- **FACT:** React routes/components call typed service wrappers; those wrappers call Tauri `invoke`, and Rust commands delegate into SQLite/market/source/Forge modules. Examples: [src/source/services/sourceService.ts](../../src/source/services/sourceService.ts#L1-L15), [src-tauri/src/source/ipc.rs](../../src-tauri/src/source/ipc.rs#L1-L12), [src-tauri/src/source/db.rs](../../src-tauri/src/source/db.rs#L8-L20).
- **FACT:** Chart ticks are read by a separate in-memory DuckDB connection over local Parquet; durable chart analysis/configuration uses the app's SQLite database. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L12-L119), [src-tauri/schema_market.sql](../../src-tauri/schema_market.sql#L1-L105).
- **FACT:** The worker is an independent Rust CLI process with its own SQLite queue and handwritten HTTP/JSON/SSE server. [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1-L90), [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1500-L1530).
- **INFERENCE:** There are two operational backends: the embedded Tauri command/database backend and the optional worker service. The worker's queue/status model should not be treated as the app's general domain workflow engine without deliberate adaptation.

### Entry points

| Entry | Role |
|---|---|
| [src/main.tsx](../../src/main.tsx#L1-L65) | Frontend bootstrap, settings initialization and HashRouter routes. |
| [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1263-L1437) | Tauri setup, app-data SQLite initialization, plugin setup, IPC command registry. |
| [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1774-L1810) | Worker CLI and optional HTTP service. |
| [src-tauri/src/bin/vora_source.rs](../../src-tauri/src/bin/vora_source.rs) | Separate source/provenance CLI binary. |
| **Workers/cron:** | No browser worker, cron scheduler, or frontend service-worker registration found. **UNKNOWN:** manually orchestrated external jobs. |

### Module boundaries

- **FACT:** TypeScript uses strict checking, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, and `@/* -> src/*`. [tsconfig.json](../../tsconfig.json#L1-L31)
- **FACT:** `invoke` appears in domain services (`src/source/services`, `src/lib/market`, `src/services`) and also directly in [UpdaterProvider.tsx](../../src/shared/components/UpdaterProvider.tsx#L1-L18). No import-boundary ESLint/plugin rule or linter configuration was found.
- **FACT:** `src-tauri/src/resolver.rs` is not present; Tauri command handlers are in `lib.rs` and domain `ipc.rs` modules. `ipc.rs` delegates to domain DB modules. [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1308-L1433), [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L1-L15), [src-tauri/src/source/ipc.rs](../../src-tauri/src/source/ipc.rs#L1-L12)
- **INFERENCE:** TypeScript aliases/types provide compile-time organization, but frontend/backend layer boundaries are conventions rather than enforced architectural rules.

## 3. DATA LAYER

### Engines and connection handling

- **FACT:** SQLite is the durable workspace store (`vora_data.vora` under the Tauri app-data directory); the Tauri process holds a shared `rusqlite::Connection` behind a mutex. [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1268-L1301), [src-tauri/src/mind/canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L1-L14)
- **FACT:** Startup initializes canvas, task-flow, journal, questions, market, source, Forge index, and portable schemas against that connection. [src-tauri/src/mind/portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891)
- **FACT:** DuckDB is opened in-memory for each candle fetch and reads Parquet files directly; no DuckDB database file/ORM is used. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L12-L119)
- **FACT:** Worker uses a separate `vora_worker_queue.db` SQLite file, and Forge's CLI/index operation also has standalone database/test paths. [src-tauri/src/bin/vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L18-L20), [src-tauri/src/forge/index.rs](../../src-tauri/src/forge/index.rs#L1-L12)
- **FACT:** Query access is direct parameterized SQL with `rusqlite`; no ORM or migration framework is declared. [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L24-L50)

### Runtime tables and fields

Notation: `!` = `NOT NULL`; `?` = nullable; `PK` = primary key; `FK` = foreign key; omitted defaults mean no SQL default was declared. SQLite affinity types are shown as declared. The runtime workspace schemas are initialized from Rust; [src-tauri/src/schema.sql](../../src-tauri/src/schema.sql) is an additional older/duplicated SQL schema and is not the canvas initializer.

#### Canvas/board database (13 tables)

| Table | Fields (declared type, nullability/default) and relations |
|---|---|
| `mind_boards` | `id TEXT PK`, `title TEXT!`, `created_at INTEGER!`, `updated_at INTEGER!`, `active_version_id TEXT?`, `branch_origin_version_id TEXT?`, `library_item_group_id TEXT?`. |
| `mind_board_versions` | `id TEXT PK`, `board_id TEXT! FK mind_boards`, `version_major/minor/patch INTEGER!`, `parent_version_id TEXT? FK self`, `forked_from_version_id TEXT? FK self`, `snapshot_json TEXT!`, `created_at INTEGER!`; unique history index is non-unique. |
| `library_item_groups` | `id TEXT PK`, `name/version TEXT!`, `snapshot_json TEXT!`, `created_at/updated_at INTEGER!`, `deleted_at INTEGER?`. |
| `library_item_group_versions` | `id TEXT PK`, `group_id TEXT! FK library_item_groups ON DELETE CASCADE`, `version TEXT!`, `snapshot_json TEXT!`, `created_at INTEGER!`; unique `(group_id, version)`. |
| `objects` | `id TEXT PK`, `type TEXT!`, `board_id TEXT!`, `created_at/updated_at INTEGER!`, `deleted_at INTEGER?`, `metadata JSON?`; board relationship is not declared as an FK in this table. |
| `object_versions` | `id TEXT PK`, `object_id TEXT! FK objects`, `version_major/minor/patch INTEGER!`, `parent_version_id TEXT? FK self`, `reason/summary TEXT?`, `content JSON!`, `created_at INTEGER!`. |
| `mind_snapshots` | `id TEXT PK`, `name TEXT!`, `parent_snapshot_id TEXT? FK self`, `board_id TEXT!`, `depth INTEGER!`, `replay_cost INTEGER! DEFAULT 0`, `is_checkpoint INTEGER! DEFAULT 0`, `created_at INTEGER!`. |
| `snapshot_object_changes` | `snapshot_id/object_id TEXT!` composite PK; `operation TEXT!`, `object_version_id TEXT?`; FKs to `mind_snapshots` and `objects`. |
| `snapshot_placement_changes` | `snapshot_id/object_id TEXT!` composite PK; `operation TEXT!`, `x/y/width/height REAL?`, `rotation REAL DEFAULT 0`, `z_index INTEGER DEFAULT 0`, `style JSON?`; FKs to `mind_snapshots` and `objects`. |
| `snapshot_link_changes` | `snapshot_id/link_id TEXT!` composite PK, `operation TEXT!`, `source_object_id/target_object_id/link_type TEXT?`; FK to `mind_snapshots`. |
| `checkpoint_objects` | `checkpoint_id/object_id TEXT!` composite PK, `object_version_id TEXT!`; FK `checkpoint_id -> mind_snapshots`. |
| `checkpoint_placements` | `checkpoint_id/object_id TEXT!` composite PK, `x/y/width/height REAL!`, `rotation REAL DEFAULT 0`, `z_index INTEGER DEFAULT 0`, `style JSON?`; FK to `mind_snapshots`. |
| `checkpoint_links` | `checkpoint_id/link_id/source_object_id/target_object_id/link_type TEXT!`; composite PK `(checkpoint_id, link_id)`, FK `checkpoint_id -> mind_snapshots`. |
| `mind_text_links` | `id TEXT PK`; source/target board/card/field `TEXT!`, start/end offsets `INTEGER!`, `created_at/updated_at INTEGER!`; field CHECK is `title|content`, offsets CHECK nonnegative and end greater than start; no declared board/card FKs. |

Schema and indexes: [src-tauri/src/mind/canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L145-L340). There are indexes on snapshots by parent/checkpoint/depth, objects by board, snapshots by board/time, versions/history, library groups/versions, and text-link source/target.

#### Source/provenance tables (5 tables)

| Table | Fields and relations |
|---|---|
| `sources` | `id TEXT PK`, `kind TEXT!`, `path TEXT?`, `title TEXT!`, `author/language TEXT?`, `duration_sec/page_count INTEGER?`, `metadata_json TEXT! DEFAULT '{}'`, `notes TEXT! DEFAULT ''`, `status TEXT! DEFAULT 'active'`, `created_at/updated_at INTEGER!`. |
| `segments` | `id TEXT PK`, `source_id TEXT! FK sources`, `label TEXT!`, `start_sec/end_sec/page_start/page_end/line_start/line_end INTEGER?`, `notes TEXT! DEFAULT ''`, `status TEXT! DEFAULT 'active'`, `created_at/updated_at INTEGER!`, `parent_segment_id TEXT? FK self`, `order_index INTEGER! DEFAULT 0`. |
| `extracts` | `id TEXT PK`, `segment_id TEXT! FK segments`, `transform_kind/content TEXT!`, `language TEXT?`, `confidence TEXT! DEFAULT 'certain'`, `created_by TEXT! DEFAULT 'user'`, `reviewed_by_user INTEGER! DEFAULT 1`, `parent_extract_id/superseded_by TEXT? FK extracts`, `notes TEXT! DEFAULT ''`, `status TEXT! DEFAULT 'active'`, `created_at/updated_at INTEGER!`. |
| `links` | `id TEXT PK`, `extract_id TEXT! FK extracts`, `target_kind/target_id TEXT!`, `link_kind TEXT! DEFAULT 'reference'`, `note TEXT! DEFAULT ''`, `status TEXT! DEFAULT 'active'`, `created_at/updated_at INTEGER!`. |
| `source_revisions` | `id TEXT PK`, `entity_kind/entity_id/revision_type TEXT!`, `before_json TEXT?`, `after_json TEXT!`, `reason TEXT! DEFAULT ''`, `created_by TEXT! DEFAULT 'user'`, `created_at INTEGER!`; SQLite triggers reject UPDATE and DELETE. |

Enums/comments and columns are defined in [schema_source.sql](../../src-tauri/schema_source.sql#L1-L128); hierarchy columns and their indexes are added idempotently in [source/db.rs](../../src-tauri/src/source/db.rs#L8-L40). Important indexes cover segment parent/source, extract segment/parent/supersession, links, and revision entity/time. The older duplicate source declarations in [src-tauri/src/schema.sql](../../src-tauri/src/schema.sql#L125-L250) lack the segment hierarchy fields; startup's source initializer runs a migration.

#### Task, journal, questions, portable data (7 tables)

| Table | Fields and relations |
|---|---|
| `task_flow_items` | `id TEXT PK`, `title TEXT! CHECK length 1..200`, `description TEXT! DEFAULT ''`, `task_type TEXT! DEFAULT 'task' CHECK task/ai`, `completed INTEGER! DEFAULT 0 CHECK 0/1`, `position INTEGER!`, `created_at/updated_at INTEGER!`; indexes by position and type/position. |
| `journal_entries` | `entry_date TEXT PK`, `content TEXT!`, `created_at/updated_at INTEGER!`; date validated as `YYYY-MM-DD` in Rust. |
| `questions` | `id TEXT PK`, `title TEXT! CHECK trimmed length 1..200`, `description TEXT!`, `created_at/updated_at INTEGER!`; index by descending update/create time. |
| `vora_items` | `id/type/name/data TEXT!`, `created_at/updated_at INTEGER!`; index `(type, updated_at DESC)`. |
| `vora_snapshots` | `id/chart_id/name/data TEXT!`, `created_at INTEGER!`; FK `chart_id -> vora_items(id) ON DELETE CASCADE`; index `(chart_id, created_at DESC)`. |
| `vora_imported_items` | `entity_type/entity_id/display_name/source_package_id/source_package_name TEXT!`, `imported_at INTEGER!`; composite PK `(entity_type, entity_id)`, index by package/time. |
| `vora_imported_packages` | `package_id TEXT PK`, `package_name TEXT!`, `imported_at INTEGER!`. |

Sources: [task_flow.rs](../../src-tauri/src/mind/task_flow.rs#L1-L80), [journal.rs](../../src-tauri/src/mind/journal.rs#L1-L38), [questions.rs](../../src-tauri/src/mind/questions.rs#L1-L55), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L10-L50).

#### Market tables (7 tables)

| Table | Fields and relations |
|---|---|
| `market_candles` | `symbol/timeframe TEXT!`, `ts INTEGER!`, `open/high/low/close/volume REAL!`, `tick_count INTEGER!`, `first_extreme TEXT! DEFAULT 'same'`; PK `(symbol,timeframe,ts)`. |
| `market_boxes` | `address TEXT PK`, `symbol/pattern/zone TEXT!`, `step/pivot_step/scale/x/y_abs/z/y_band/x_idx/ts_start/ts_end INTEGER!`, `high/low/origin_price REAL!`, `engulf_ts/chain_id/chain_pos INTEGER?`, `duration_sec/scale_sec INTEGER! DEFAULT 0` (migration paths account for older schemas). |
| `_vora_meta` | `key TEXT PK`, `value TEXT?`. |
| `symbol_origin` | `symbol TEXT!`, `step INTEGER!`, `price_ref REAL!`, `price_ref_ts/computed_at INTEGER!`; composite PK `(symbol,step)`. |
| `scale_registry` | `symbol TEXT PK`, `registry_json TEXT!`, `updated_at INTEGER!`. |
| `chart_drawing_templates` | `id TEXT PK`, `name TEXT! UNIQUE COLLATE NOCASE`, `drawing_type/template_json TEXT!`, `created_at INTEGER!`. |
| `chart_drawing_alerts` | `id TEXT PK`, `symbol/drawing_id/condition/message TEXT!`, `timeframe INTEGER!`, `trigger_once/notify_app/notify_toast/notify_email/notify_sound/enabled INTEGER!` (notification defaults 1,1,0,0; enabled 1), `expiration_at/last_evaluated_at INTEGER?`, `created_at INTEGER!`. |

No foreign keys are declared in these market tables. Indexes are on candle symbol/time and timeframe/time, box symbol/step and chain, and alert symbol/enabled/time. [schema_market.sql](../../src-tauri/schema_market.sql#L1-L105), [market/db.rs](../../src-tauri/src/market/db.rs#L1-L120).

#### Forge index tables (17 tables)

| Table | Fields (all not-null unless `?`) and relations |
|---|---|
| `workspaces` | `workspace_id TEXT PK`, `root_path TEXT!`, `created_at INTEGER!`, `current_snapshot_id TEXT?`. |
| `snapshots` | `snapshot_id TEXT PK`, `workspace_id TEXT! FK workspaces`, `parent_snapshot_id TEXT? FK self`, `created_at INTEGER!`. |
| `files` | `file_id TEXT PK`, `workspace_id TEXT! FK workspaces`, `created_at INTEGER!`, `deleted_at INTEGER?`. |
| `file_versions` | `file_version_id TEXT PK`, `file_id TEXT! FK files`, `content_hash TEXT!`, `byte_size/created_at INTEGER!`, `parent_file_version_id TEXT? FK self`. |
| `snapshot_files` | `snapshot_id/file_id/path_at_snapshot/file_version_id TEXT!`; composite PK `(snapshot_id,file_id)`, unique `(snapshot_id,path_at_snapshot)`, FKs to snapshot/file/version. |
| `symbol_lineages` | `lineage_id TEXT PK`, `workspace_id/created_snapshot_id TEXT!`, `retired_snapshot_id TEXT?`; FKs to workspace/snapshots. |
| `symbol_lineage_versions` | `lineage_id/snapshot_id/symbol_id TEXT!`, `confidence REAL?`; composite PK `(lineage_id,snapshot_id)`, FKs to lineage/snapshot. |
| `canonical_entities` | `entity_id TEXT PK`, `workspace_id/lineage_id/initial_name TEXT!`, `created_at INTEGER!`; FKs workspace/lineage. |
| `canonical_entity_versions` | `entity_id/snapshot_id/symbol_id/lineage_id/canonical_signature/canonical_namespace/canonical_name TEXT!`, `created_at INTEGER!`; composite PK `(entity_id,snapshot_id)`, FKs entity/snapshot/lineage. |
| `forge_audit_trail` | `id INTEGER PK AUTOINCREMENT`, `task_id/workspace_id/intent/target_symbol/target_file/status TEXT!`, `base_snapshot_id/committed_snapshot_id TEXT?`, `healing_attempts INTEGER! DEFAULT 0`, `error_payload TEXT?`, `created_at INTEGER!`; FK workspace. |
| `code_files` | `file_id TEXT PK`, `file_path TEXT! UNIQUE`, `content_hash/lineage_parent_id TEXT!/?`, `last_indexed_at INTEGER!`. |
| `code_symbols` | `id TEXT PK`, `file_id/module_namespace/file_path/name/kind/scope_path/qualified_scope/signature/canonical_signature/body_hash/visibility TEXT!`, `lineage_id TEXT?`, `start_line/end_line/start_byte/end_byte INTEGER!`; FK file with cascade delete. |
| `code_scopes` | `scope_id TEXT PK`, `file_id TEXT!`, `parent_scope_id TEXT?`, `visible_bindings_json TEXT!`; FK file with cascade delete. |
| `code_imports` | `id INTEGER PK AUTOINCREMENT`, `file_id TEXT?`, `file_path/source_module/imported_name TEXT!`, `alias TEXT?`, `line INTEGER!`; FK file path with cascade delete. |
| `code_references` | `id INTEGER PK AUTOINCREMENT`, `file_path/caller_scope/callee_name TEXT!`, `line/byte_offset INTEGER!`; FK file path with cascade delete. |
| `code_raw_references` | `id INTEGER PK AUTOINCREMENT`, `file_id/scope_id TEXT!`, `caller_symbol_id/receiver_name TEXT?`, `callee_raw_name/ref_kind TEXT!`, `is_member_access INTEGER! DEFAULT 0`, `access_chain_json TEXT?`, `byte_start/byte_end/line INTEGER!`; FK file with cascade delete. |

Indexes cover file path/hash, symbols (file/namespace/name/lineage), scopes, references, snapshots/files/versions, lineages, canonical entities/versions, and audit task. [forge/index.rs](../../src-tauri/src/forge/index.rs#L1-L219).

#### Worker database (separate `vora_worker_queue.db`, 5 tables)

| Table | Fields and relations |
|---|---|
| `worker_tasks` | `id INTEGER PK AUTOINCREMENT`, `task_id TEXT! UNIQUE`, `project_root/intent TEXT!`, `target_file/target_symbol/patch_code TEXT?`, `priority INTEGER! DEFAULT 10`, `status TEXT! DEFAULT 'PENDING'`, `claimed_by TEXT?`, `healing_attempts INTEGER! DEFAULT 0`, `last_error_stage/error_payload TEXT?`, `created_at INTEGER!`, `started_at/finished_at INTEGER?`. |
| `worker_heartbeats` | `worker_id TEXT PK`, `last_seen INTEGER!`, `current_task_id TEXT?`, `status TEXT!`, `version TEXT! DEFAULT '0.55.0'`, `port INTEGER! DEFAULT 4040`, `active_tasks INTEGER! DEFAULT 0`, `max_concurrency INTEGER! DEFAULT 2`. |
| `cluster_ast_events` | `id INTEGER PK AUTOINCREMENT`, `event_id TEXT! UNIQUE`, `source_node/file_path/action TEXT!`, `symbol_id TEXT?`, `timestamp INTEGER!`. |
| `cluster_file_locks` | `file_path TEXT PK`, `locked_by_task/locked_by_node TEXT!`, `locked_at INTEGER!`. |
| `cluster_logs` | `id INTEGER PK AUTOINCREMENT`, `node_id/level/message TEXT!`, `task_id TEXT?`, `timestamp INTEGER!`. |

Indexes are task status/priority/time and descending cluster-log timestamp. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L25-L90).

### Migration system and fixtures

- **FACT:** No migration tool, numbered migration folder, or `PRAGMA user_version` convention was found. Startup uses `CREATE TABLE IF NOT EXISTS`, `PRAGMA table_info`, explicit `ALTER TABLE`, and `_vora_meta` markers; some historical alterations intentionally ignore errors. [source/db.rs](../../src-tauri/src/source/db.rs#L8-L40), [market/db.rs](../../src-tauri/src/market/db.rs#L1-L120)
- **FACT:** Legacy DB/settings selector migration, backup import/export, and schema bootstrap are implemented in `portable_db.rs`. [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L65-L140), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L650-L755), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891)
- **FACT:** Source revisions are append-only by SQLite trigger; test fixtures live in Rust module tests and Cargo integration tests, not in a general seed directory. [schema_source.sql](../../src-tauri/schema_source.sql#L100-L128), [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L58-L100)
- **FACT:** Tick input is expected at `data/ticks/<SYMBOL>/**/*.parquet`, with numeric `ts` (Unix seconds), `price`, and `volume`; `data/ticks/XAUUSD/README.md` documents the format. [local_db.rs](../../src-tauri/src/market/local_db.rs#L48-L118), [XAUUSD README](../../data/ticks/XAUUSD/README.md)

## 4. DOMAIN MODEL & STATE MACHINES

### Core entities and lifecycle

- **FACT:** Boards contain versioned cards/canvas objects and snapshot/checkpoint deltas. [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L153-L340)
- **FACT:** Source provenance is modeled as source → hierarchical segment → extract(s) → links, with revision rows recording mutations. [schema_source.sql](../../src-tauri/schema_source.sql#L1-L128), [source/db.rs](../../src-tauri/src/source/db.rs#L70-L120)
- **FACT:** Market charts derive candles from local tick Parquet; drawings/templates/alerts, detected boxes, scale config, and symbol origins are persisted in SQLite. [local_db.rs](../../src-tauri/src/market/local_db.rs#L12-L119), [schema_market.sql](../../src-tauri/schema_market.sql#L1-L105)
- **FACT:** Worker tasks progress through string statuses; Forge maintains code-symbol identities/lineages and an audit trail. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L25-L90), [forge/index.rs](../../src-tauri/src/forge/index.rs#L1-L120)
- **Reuse candidates:** Task Flow is a simple ordered task list with completion; the worker queue is a processing pipeline; source revisions provide an append-only history pattern. [task_flow.rs](../../src-tauri/src/mind/task_flow.rs#L1-L105), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L801-L870), [schema_source.sql](../../src-tauri/schema_source.sql#L70-L128)

### Status/state enums and transitions

| State family | Values | Enforcement / observed transitions |
|---|---|---|
| Trace node status | `accepted`, `rejected`, `pending`, `current`, `abandoned`; node kinds `state`, `decision`, `experiment`, `rejected`; edge kinds `main`, `branch`, `rejected`. [vora_trace.rs](../../src-tauri/src/vora_trace.rs#L1-L55), [src/types/index.ts](../../src/types/index.ts#L14-L16) | `addNode` defaults rejected nodes to `rejected`, other nodes to `pending`; editor/store can update node status. No centralized allowed-transition graph found. [traceStore.ts](../../src/features/vora-trace/store/traceStore.ts#L215-L245) |
| Worker task status | Observed `PENDING`, `CLAIMED`, `HEALING`, `COMPLETED`, `FAILED`. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L25-L45), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L801-L870), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1300-L1320) | Claim sets `PENDING → CLAIMED`; execution may set `HEALING`, then terminal `COMPLETED`/`FAILED`; stale recovery resets to `PENDING`. Status is a free-form SQL string, not an enum/CHECK. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L380-L400), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L801-L870) |
| Task Flow | `TaskFlowType`: `task` / `ai`; `completed: boolean`. [task_flow.rs](../../src-tauri/src/mind/task_flow.rs#L1-L35), [src/types/index.ts](../../src/types/index.ts#L1-L12) | Completion toggles a boolean; no multi-stage status machine. [lib.rs](../../src-tauri/src/lib.rs#L189-L257) |
| Source status | SQL defaults to `active`; comments describe `active|archived`; model property is `String`. [schema_source.sql](../../src-tauri/schema_source.sql#L8-L23), [source/model.rs](../../src-tauri/src/source/model.rs#L180-L245) | Creation normalizes an empty status to `active`; no typed transition enforcement found. Revision event types are `create|update|correct|retract|review`, but these are audit-event kinds, not current statuses. [source/db.rs](../../src-tauri/src/source/db.rs#L70-L120), [source/model.rs](../../src-tauri/src/source/model.rs#L155-L178) |
| Settings save state | `saving`, `saved`, `error`. [useAppStore.ts](../../src/shared/store/useAppStore.ts#L85-L105) | Per-entry debounce/queued saves; status does not represent a domain entity lifecycle. [useAppStore.ts](../../src/shared/store/useAppStore.ts#L1-L32) |
| Updater | `idle`, `checking`, `downloading`, `ready`, `installing`, `error`. [UpdaterProvider.tsx](../../src/shared/components/UpdaterProvider.tsx#L1-L32) | UI/update flow state; not a persistent workflow. |
| Forge semantic resolution | `Resolved`, `Ambiguous`, `Unresolved`, `DynamicFallback`. [forge/model.rs](../../src-tauri/src/forge/model.rs#L518-L565) | Resolution outcome, not a workflow status; resolver chooses the variant. [forge/resolver.rs](../../src-tauri/src/forge/resolver.rs#L210-L260) |

Other persisted domain enums include source kind/transform/confidence/creator/link/revision types and market timeframe/drawing/alert/box types; these are detailed in [source/model.rs](../../src-tauri/src/source/model.rs#L1-L178) and [market/model.rs](../../src-tauri/src/market/model.rs#L1-L190). They do not form state machines.

## 5. API SURFACE

### Embedded Tauri RPC

- **FACT:** Tauri exposes named `invoke` commands; Rust return values are serialized as JSON values, and `Result<T, String>` errors reject the invoke. No custom binary IPC transport is used. [marketApi.ts](../../src/lib/market/marketApi.ts#L56-L75), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L41-L66)
- **FACT:** The complete registration list is in [lib.rs](../../src-tauri/src/lib.rs#L1308-L1433). The following table enumerates the registered methods grouped by owning domain. Individual Rust signatures are the input/output contract; no separate OpenAPI/JSON Schema registry exists.

| Domain | Registered RPC methods | Input/output shape source |
|---|---|---|
| Update | `install_update`, `get_last_update_failure` | Update request/state and result/error in [lib.rs](../../src-tauri/src/lib.rs#L32-L88). |
| Boards/tasks | `get_all_boards`, `create_board`, `get_task_flow_items`, `create_task_flow_item`, `update_task_flow_item`, `set_task_flow_item_completed`, `reorder_task_flow_items`, `delete_task_flow_item`, `rename_board`, `delete_board` | Rust params and result DTOs in [lib.rs](../../src-tauri/src/lib.rs#L150-L257), [lib.rs](../../src-tauri/src/lib.rs#L425-L444); board DTOs in [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L1-L110). |
| Trace | `trace_load_graph`, `trace_save_graph`, `trace_save_decision_md`, `trace_list_decisions`, `trace_scan_ecosystem`, `trace_git_log_for_project`, `trace_read_all_decisions`, `trace_find_commits_for_file`, `trace_export_graph_json`, `trace_import_graph_json`, `trace_app_version`, `trace_recent_decisions` | Graph/project/decision DTOs and command signatures in [vora_trace.rs](../../src-tauri/src/vora_trace.rs#L1-L115), [vora_trace.rs](../../src-tauri/src/vora_trace.rs#L390-L670). |
| Journal/questions | `get_journal_entries`, `save_journal_entry`, `get_questions`, `create_question`, `save_question`, `delete_question` | Inputs and typed results in [lib.rs](../../src-tauri/src/lib.rs#L258-L316); DTOs in [journal.rs](../../src-tauri/src/mind/journal.rs#L1-L13), [questions.rs](../../src-tauri/src/mind/questions.rs#L1-L15). |
| Board snapshots/library | `get_board_versions`, `create_board_version`, `checkout_board_version`, `get_library_item_groups`, `get_library_item_group_versions`, `get_library_item_group_usages`, `create_library_item_group`, `commit_library_item_group_version`, `move_library_item_group_to_trash`, `restore_library_item_group`, `commit_snapshot`, `resolve_snapshot`, `get_all_snapshots` | Signatures and result DTOs in [lib.rs](../../src-tauri/src/lib.rs#L317-L424), [lib.rs](../../src-tauri/src/lib.rs#L664-L728). |
| Cards/links | `get_all_cards`, `get_text_links_for_card`, `create_text_link`, `delete_text_link`, `save_card_direct`, `move_card_to_bin`, `restore_card_from_bin` | Card/text-link types and signatures in [lib.rs](../../src-tauri/src/lib.rs#L445-L530), [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L1-L145). |
| Forge | `parse_code_ast`, `read_project_file`, `get_forge_tools`, `execute_forge_tool`, `apply_patch_and_validate`, `create_git_commit`, `scan_workspace`, `get_git_commit_history`, `run_project_tests`, `forge_test_candidate_patch`, `forge_test_multi_candidate_patches`, `forge_rollback_to_snapshot`, `resolve_strict_edges`, `simulate_semantic_change`, `create_semantic_plan`, `forge_compute_ast_semantic_diff`, `forge_repair_and_expand_plan`, `forge_find_symbol`, `forge_get_impact_radius`, `forge_execute_patch_transaction`, `forge_execute_multi_patch_transaction`, `forge_get_lineage_history`, `forge_get_canonical_entities`, `forge_record_audit_entry`, `forge_get_task_audit_history` | Rust signatures in [lib.rs](../../src-tauri/src/lib.rs#L729-L1260); domain DTOs under [src-tauri/src/forge/](../../src-tauri/src/forge). |
| Portable DB/settings | `get_settings_file_path`, `get_current_db_path`, `export_database_backup`, `export_settings_json`, `import_settings_json`, `import_database_backup`, `save_setting`, `get_setting`, `get_all_settings`, `delete_setting`, `replace_all_settings`, `save_vora_item`, `get_vora_items`, `save_vora_snapshot`, `get_vora_snapshots`, `get_all_vora_snapshots`, `replace_vora_snapshots`, `delete_vora_snapshot`, `clear_vora_snapshots` | Signatures in [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L309-L390), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L444-L620), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L650-L755). |
| Sources | `src_list_sources`, `src_get_source_bundle`, `src_list_extracts`, `src_get_extract`, `src_create_source`, `src_create_segment`, `src_create_extract`, `src_create_link`, `src_mark_extract_reviewed` | Request/response DTOs and command signatures in [source/ipc.rs](../../src-tauri/src/source/ipc.rs#L20-L130), [source/ipc.rs](../../src-tauri/src/source/ipc.rs#L387-L445). |
| Market | `mkt_fetch_local_candles`, `mkt_detect_boxes`, `mkt_cancel_request`, `mkt_get_symbol_origin`, `mkt_refresh_symbol_origin`, `mkt_get_scale_registry`, `mkt_set_scale_registry`, `mkt_list_scale_registries`, `mkt_list_drawing_templates`, `mkt_save_drawing_template`, `mkt_delete_drawing_template`, `mkt_list_drawing_alerts`, `mkt_save_drawing_alert`, `mkt_update_drawing_alert`, `mkt_delete_drawing_alert`, `mkt_delete_drawing_alerts_for_drawings`, `mkt_list_pivots` | DTOs/signatures in [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L10-L35), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L41-L83), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L324-L510). |

**Auth:** Embedded commands use Tauri's app/webview boundary and have no per-user/role authorization layer found. Tauri capability `default` grants core/dialog/opener/process/updater permissions; it does not list frontend filesystem read permissions. [default.json](../../src-tauri/capabilities/default.json#L1-L14)

### Worker HTTP/SSE API

All non-OPTIONS requests pass through worker auth: `WORKER_AUTH_TOKEN` requires an `Authorization: Bearer ...` header; absent token is allowed only outside remote mode. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1381-L1395), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1500-L1530)

| Method/path | Input | Output |
|---|---|---|
| `GET /api/health` | None | Health/version/current node/database/auth flag, cluster counts, queue counts and nodes. |
| `GET /api/nodes` | None | `{ nodes: [...] }`. |
| `GET /api/cluster/load` | None | Cluster load JSON. |
| `GET /api/cluster/locks` | None | `{ locks: [{file_path, locked_by_task, locked_by_node, locked_at}] }`. |
| `GET /api/cluster/logs` | None | Up to 50 `{node_id, task_id, level, message, timestamp}` entries. |
| `GET /api/tasks` | None | `{ tasks: [{task_id,status,intent,claimed_by,error}] }`, latest 10. |
| `POST /api/tasks` | JSON fields `intent`, `project_root?`, `target_file?`, `target_symbol?`, `patch_code?` | `201 {status:"enqueued",task_id}`; invalid JSON returns a JSON `error` and 400. |
| `GET /api/tasks/:id` | Task ID in path | `{task_id,status,intent,claimed_by,error}` or JSON error/404. |
| `GET /api/tasks/:id/diagnostics` | Task ID in path | Task status, healing attempts, last error stage/payload or JSON error/404. |
| `GET /api/logs/stream`, `GET /api/tasks/:id/stream` | Optional task ID | Server-sent event stream. |

Paths, method handling, response bodies and status codes: [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1500-L1690), stream routes [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1510-L1535), route list [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1710-L1740). Worker HTTP uses handwritten request parsing and `serde_json::Value`, not a schema validator.

### Validation and errors

- **FACT:** No Zod/Joi/Yup/Valibot dependency was found. Validation is handwritten in Rust/UI (for example task titles, dates, scale registries, alert fields, and source requests). [task_flow.rs](../../src-tauri/src/mind/task_flow.rs#L90-L110), [journal.rs](../../src-tauri/src/mind/journal.rs#L20-L55), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L420-L460)
- **FACT:** Tauri domain commands generally convert errors to `String`; worker HTTP returns JSON `{error: ...}` with HTTP status codes. There is no shared application error-code enum or documented versioned API convention. [source/ipc.rs](../../src-tauri/src/source/ipc.rs#L1-L12), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1510-L1525)

## 6. FRONTEND STATE & ROUTING

### Server state and stores

- **FACT:** No React Query, SWR, or RTK Query package is declared. API reads are imperative `invoke` calls in services/components and local hooks/state. [package.json](../../package.json#L25-L70)
- **FACT:** `useAppStore` is Zustand-persisted under `vora-app-store`; `partialize` persists `cards` and `executedTasks`, not the complete store. [useAppStore.ts](../../src/shared/store/useAppStore.ts#L38-L110), [useAppStore.ts](../../src/shared/store/useAppStore.ts#L545-L565)
- **FACT:** `useSettingsStore` is in-memory Zustand state, with settings persisted to Tauri-backed JSON through `voraDb`; legacy settings migrate from `localStorage`. [useSettingsStore.ts](../../src/shared/store/useSettingsStore.ts#L1-L120)
- **FACT:** `useDocumentStore` holds one active document, dirty/loading/error state, and uses a 1-second debounced save. [useDocumentStore.ts](../../src/features/file-store/useDocumentStore.ts#L1-L75)
- **FACT:** `useTraceStore` contains graph, selection/filter/sync state and starts with `mockGraph`; outside Tauri it remains on mock data. [traceStore.ts](../../src/features/vora-trace/store/traceStore.ts#L1-L100)
- **FACT:** Chart candle data is page-local React state (`CandleDto[]`), fetched via `mktFetchLocalCandles`, rather than the main Zustand store. [ChartPage.tsx](../../src/pages/ChartPage.tsx#L410-L430), [ChartPage.tsx](../../src/pages/ChartPage.tsx#L1965-L2005)

### Route/screen map

| Route / surface | Component |
|---|---|
| `/` | Redirects to chart or renders `App`, depending on default route. |
| `/home` | `App` canvas shell. |
| `/market` | Redirect to `/chart`. |
| `/chart` | `ChartPage`, mounted persistently while chart or snapshots route is active. |
| `/sources` | `SourcesPage`. |
| `/scale-lab` | Redirect to `/chart`. |
| `/snapshots` | `SnapshotsPage`, with chart retained underneath. |
| In-canvas panels | `VoraTraceApp`, `TaskFlowPage` (`task`/`ai`), `JournalPage`, `QuestionsBoardPage`; selected through `App` panel state, not independent URL routes. |

Routes: [src/main.tsx](../../src/main.tsx#L28-L65); panels: [src/App.tsx](../../src/App.tsx#L2490-L2528).

## 7. UI SYSTEM

- **FACT:** Styling uses Tailwind CSS v4 through Vite, `@theme` design tokens in `App.css`, and feature-specific CSS (notably chart/drawing styles); React Flow stylesheet is imported globally. [vite.config.ts](../../vite.config.ts#L1-L12), [App.css](../../src/App.css#L1-L40), [src/main.tsx](../../src/main.tsx#L10-L17)
- **FACT:** No single external component library is declared; Lucide icons, Tailwind utility classes, and custom components are used. [package.json](../../package.json#L25-L70)

| Pattern | Best existing example | Notes |
|---|---|---|
| List/master view | [SourceMasterList.tsx](../../src/pages/sources/SourceMasterList.tsx#L1-L115) | Search/filter, selected row, empty/no-match states. |
| Detail view | [SourceDetailView.tsx](../../src/pages/sources/SourceDetailView.tsx#L1-L190) | Metadata header, related segment tree, empty detail state. |
| Create/edit form | [AddSourceModal.tsx](../../src/pages/sources/AddSourceModal.tsx#L1-L165) | Hand-validated submit, loading/error, DTO conversion. |
| Status chip/badge | [SourceMasterList.tsx](../../src/pages/sources/SourceMasterList.tsx#L65-L100) | Kind badge; actual source lifecycle status is not consistently rendered as a chip. |
| Transition action | [TaskFlowPage.tsx](../../src/shared/layout/TaskFlowPage.tsx#L155-L230) | Completion actions and priority ordering; workflow is only a completed boolean plus order. |
| History/timeline | [TaskFlowPage.tsx](../../src/shared/layout/TaskFlowPage.tsx#L170-L230) | Priority timeline; for causal history use [WhyAmIHere.tsx](../../src/features/vora-trace/components/timeline/WhyAmIHere.tsx#L1-L45). |
| Permission check | **UNKNOWN / none found** | No role/permission UI pattern or permission map exists. Worker token enforcement is backend-side. |
| Validation schema | [source/ipc.rs](../../src-tauri/src/source/ipc.rs#L80-L130) | Typed request structs plus Rust/UI checks; no shared schema library. |
| Migration template | [source/db.rs](../../src-tauri/src/source/db.rs#L8-L40) | Idempotent schema bootstrap and `PRAGMA table_info`/`ALTER TABLE` pattern. |
| Test template | [journal.rs](../../src-tauri/src/mind/journal.rs#L65-L110) | Rust unit test using in-memory SQLite; Cargo integration examples under `src-tauri/tests/`. |
| Modal/dialog | [CreateDrawingAlertDialog.tsx](../../src/components/market/CreateDrawingAlertDialog.tsx#L1-L110) | Accessible dialog role, submit state, inline error. |
| Empty/loading/error state | [TaskFlowPage.tsx](../../src/shared/layout/TaskFlowPage.tsx#L200-L230) | Has all three states; errors include reload action. |
| Toast | [SnapshotsPage.tsx](../../src/pages/SnapshotsPage.tsx#L23-L50), [SnapshotsPage.tsx](../../src/pages/SnapshotsPage.tsx#L125-L155) | Local timed notice; no shared toast framework found. |
| Confirmation | **UNKNOWN / no canonical reusable example found** | Deletion actions exist, but no common confirmation component/pattern was identified. |

## 8. CROSS-CUTTING

| Concern | Findings |
|---|---|
| Auth/roles | The embedded local desktop has no user/role model found. Worker may enforce bearer-token authentication in remote mode and refuses startup without a token; local mode can run without one. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L200-L210), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1381-L1395) |
| Logging | Rust uses standard output/`eprintln!` and frontend uses `console`; no tracing/logging SDK dependency or structured centralized logger found. [Cargo.toml](../../src-tauri/Cargo.toml#L24-L50), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L200-L220) |
| Analytics | No analytics/event tracking integration found. |
| Notifications | Market drawing alerts support app/browser notification, toast, sound flags; email is disabled/unconfigured in UI. [CreateDrawingAlertDialog.tsx](../../src/components/market/CreateDrawingAlertDialog.tsx#L30-L90) |
| i18n | No i18n framework/dependency found; UI strings are inline English, while app state supports RTL/LTR text direction. [package.json](../../package.json#L25-L70), [useAppStore.ts](../../src/shared/store/useAppStore.ts#L38-L108) |
| File access | Frontend has dialog permissions but no `fs:read-file` capability. Native Rust reads local project/source files and tick Parquet; source records can store file paths. [default.json](../../src-tauri/capabilities/default.json#L1-L14), [local_db.rs](../../src-tauri/src/market/local_db.rs#L48-L119) |
| Background jobs | Worker process polls a SQLite task queue and runs tasks; no in-app scheduler/cron was found. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1734-L1772) |
| Audit/history | Source revisions are append-only and trigger-protected; Forge has `forge_audit_trail`; board snapshots/versions and worker logs also retain history. [schema_source.sql](../../src-tauri/schema_source.sql#L70-L128), [forge/index.rs](../../src-tauri/src/forge/index.rs#L78-L118), [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L153-L340) |
| File upload | No upload service was found; paths are selected/stored locally, and worker tasks may carry source code/patch content. **UNKNOWN:** external sync tooling. |

## 9. QUALITY

- **FACT:** Frontend has strict TypeScript compiler checks; no frontend test framework, test files, or test script were found. [tsconfig.json](../../tsconfig.json#L1-L31), [package.json](../../package.json#L1-L24)
- **FACT:** Rust has module unit tests and named integration tests covering contracts, semantic lineage, worker lifecycle, cluster, load balancing, self-healing and diagnostics. See [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L58-L100) and [src-tauri/tests/](../../src-tauri/tests).
- **FACT:** Coverage is not configured or reported; **UNKNOWN** current percentage.
- **FACT:** No linter is configured in package scripts. Vite build includes TypeScript type-check through `pnpm build`; release CI installs dependencies and builds a Windows NSIS package but does not run test/typecheck steps explicitly before the Tauri build. [package.json](../../package.json#L6-L24), [release.yml](../../.github/workflows/release.yml#L25-L75)
- **FACT:** CI/release is triggered on `v*` tags, targets `windows-latest` and NSIS; deployment support also includes a Docker worker image and systemd service. [release.yml](../../.github/workflows/release.yml#L1-L20), [Dockerfile.worker](../../Dockerfile.worker#L1-L48), [vora-worker.service](../../deploy/systemd/vora-worker.service#L1-L20)
- **FACT:** No lint/type-check/test workflow, code coverage, or frontend test command was found. **UNKNOWN:** private/local CI outside tracked GitHub workflows.

## 10. EXTENSION POINTS & CONVENTIONS I MUST FOLLOW

| Need | Existing convention / registration point |
|---|---|
| New feature specification | Repository guidance requests a verified `specs/` document before major features; see [AGENTS.md](../../AGENTS.md) and [specs/](../../specs). |
| Frontend domain | Use feature/domain folders under `src/` (examples: `src/source/`, `src/features/vora-trace/`, `src/lib/market/`); UI pages are in `src/pages/`. [src/](../../src) |
| Shared types | Canonical shared types are in [src/types/index.ts](../../src/types/index.ts); several domains also keep local DTO/model types (e.g. [marketApi.ts](../../src/lib/market/marketApi.ts#L56-L75)). |
| Persistence/backend | App SQLite bootstrap in `mind/portable_db.rs`; domain DB module, IPC wrapper, then registration in `lib.rs`'s `generate_handler!`. [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891), [lib.rs](../../src-tauri/src/lib.rs#L1308-L1433) |
| New route | Register in `src/main.tsx`; navigation links/sidebar are separate UI surfaces and may also need updates. [main.tsx](../../src/main.tsx#L28-L65), [LeftSidebar.tsx](../../src/shared/layout/LeftSidebar.tsx) |
| In-canvas panel | Add panel state/navigation and render branch in `App.tsx`; panel state currently lives in component state. [App.tsx](../../src/App.tsx#L360-L380), [App.tsx](../../src/App.tsx#L2490-L2528) |
| New table | Add idempotent domain schema/init/migration in the owning Rust DB module and call it from workspace initialization; no separate migration registry exists. [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891) |
| Feature flag | No central feature-flag service/map found. Existing toggles use persisted settings keys and UI state; debug flags use environment variables. [useSettingsStore.ts](../../src/shared/store/useSettingsStore.ts#L25-L65), [market/ipc.rs](../../src-tauri/src/market/ipc.rs#L200-L220) |
| Auth/permissions | No app permission map/role registry. Capability changes belong in `src-tauri/capabilities/default.json`; worker remote auth is independent. [default.json](../../src-tauri/capabilities/default.json#L1-L14), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L200-L210) |
| Code style/type safety | TypeScript strict/no-unused checks; Rust DTOs use Serde. Repo guidance requires domain interfaces mirrored between Rust and TS and IPC through services. [tsconfig.json](../../tsconfig.json#L1-L31), [AGENTS.md](../../AGENTS.md) |

## 11. RISKS & DEBT

| # | Risk | Evidence / why it matters |
|---|---|---|
| 1 | No standardized migrations | Startup performs multiple ad hoc idempotent alterations, with some ignored errors; adding state increases upgrade/partial-migration risk. [market/db.rs](../../src-tauri/src/market/db.rs#L1-L120) |
| 2 | Duplicate/stale schema definitions | Canvas runtime schema is inline while `src-tauri/src/schema.sql` carries older snapshots/source definitions; source schema also exists separately. Risk of editing a schema that runtime does not use or having `IF NOT EXISTS` preserve an incompatible older shape. [canvas_db.rs](../../src-tauri/src/mind/canvas_db.rs#L145-L340), [schema.sql](../../src-tauri/src/schema.sql#L1-L250), [schema_source.sql](../../src-tauri/schema_source.sql#L1-L128) |
| 3 | Statuses are inconsistently typed | Source/worker/Forge audit statuses are free-form `TEXT`/`String`; transition rules are not represented in SQL constraints. [schema_source.sql](../../src-tauri/schema_source.sql#L8-L23), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L25-L45) |
| 4 | Frontend test gap | No JS test framework/script found, so UI state transitions and persistence are not covered by automated frontend tests. [package.json](../../package.json#L1-L24) |
| 5 | No CI test/lint gates | Release CI builds but does not explicitly run Rust tests, lint, or standalone TypeScript checks. [release.yml](../../.github/workflows/release.yml#L25-L75) |
| 6 | API has no machine-readable schema/versioning | Tauri commands are a large hand-maintained registry and worker endpoints use handwritten JSON parsing; adding fields requires coordinated edits. [lib.rs](../../src-tauri/src/lib.rs#L1308-L1433), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1625-L1665) |
| 7 | State is split across DB, localStorage/Zustand and files | Cards/executed task records are persisted by Zustand while domain data also resides in SQLite; trace graph uses files and has mock fallback. This creates reconciliation and migration surfaces. [useAppStore.ts](../../src/shared/store/useAppStore.ts#L545-L565), [traceStore.ts](../../src/features/vora-trace/store/traceStore.ts#L70-L110) |
| 8 | Import boundaries are not enforced | `invoke` is used directly from a shared UI component, and no dependency-boundary lint rule was found. [UpdaterProvider.tsx](../../src/shared/components/UpdaterProvider.tsx#L1-L18), [tsconfig.json](../../tsconfig.json#L1-L31) |
| 9 | Worker auth must match all clients/transports | Auth is required for SSE when token-configured, while browser `EventSource` in `workerClient` does not attach an Authorization header; remote authenticated log streaming may fail. [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1500-L1530), [workerClient.ts](../../src/shared/services/workerClient.ts#L90-L135) |
| 10 | Bundled tick data may inflate release artifacts | `tauri.conf.json` includes `../data/ticks/**/*`; Parquet payloads are ignored except the checked-in XAUUSD README, so release build contents depend on local dataset availability. [tauri.conf.json](../../src-tauri/tauri.conf.json#L29-L43), [.gitignore](../../.gitignore#L30-L65), [XAUUSD README](../../data/ticks/XAUUSD/README.md) |

## 12. REUSE MAP

| Need | Existing thing to reuse | File path | Confidence |
|---|---|---|---|
| List view | Source master list with search, selection, empty/no-match states | [SourceMasterList.tsx](../../src/pages/sources/SourceMasterList.tsx#L1-L115) | High |
| Detail view | Source detail panel with metadata, related tree, empty state | [SourceDetailView.tsx](../../src/pages/sources/SourceDetailView.tsx#L1-L190) | High |
| Form | Add Source modal; typed request, validation, busy/error handling | [AddSourceModal.tsx](../../src/pages/sources/AddSourceModal.tsx#L1-L165) | High |
| Status chip | Reuse existing kind/type badge styling; lifecycle status chip itself is not canonical | [SourceMasterList.tsx](../../src/pages/sources/SourceMasterList.tsx#L65-L100) | Medium |
| Transition action | Task Flow complete/reorder actions; for real multi-state transitions, build on this only as a visual pattern | [TaskFlowPage.tsx](../../src/shared/layout/TaskFlowPage.tsx#L155-L230) | Medium |
| History/timeline | Source append-only revisions or Trace causal chain | [schema_source.sql](../../src-tauri/schema_source.sql#L70-L128), [WhyAmIHere.tsx](../../src/features/vora-trace/components/timeline/WhyAmIHere.tsx#L1-L45) | High |
| Permission check | No app-level permission helper/map found; worker bearer auth is the only explicit authorization pattern | [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L1381-L1395) | Low |
| Validation schema | Typed Rust request structs plus service/UI checks; no schema library | [source/ipc.rs](../../src-tauri/src/source/ipc.rs#L80-L130), [AddSourceModal.tsx](../../src/pages/sources/AddSourceModal.tsx#L30-L75) | High |
| Migration template | Idempotent `init_schema` and `PRAGMA table_info` migration helper | [source/db.rs](../../src-tauri/src/source/db.rs#L8-L40) | High |
| Test template | In-memory SQLite unit test; Cargo integration test conventions | [journal.rs](../../src-tauri/src/mind/journal.rs#L65-L110), [src-tauri/tests/](../../src-tauri/tests) | High |

## 13. RECOMMENDED INTEGRATION POINT

- **INFERENCE:** Implement “Research Workflow” as a new first-class domain feature rather than representing its lifecycle as a set of Task Flow booleans or overloading the worker queue. Task Flow only models `completed`; worker statuses are operational queue state. Neither currently models a research case with explicit domain transitions/history. [task_flow.rs](../../src-tauri/src/mind/task_flow.rs#L1-L35), [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs#L25-L45)
- **INFERENCE:** For frontend placement, follow the Sources feature's page/service/components decomposition (`src/pages`, feature-specific UI, typed invoke service). If it should behave as a main workspace destination, add a route and navigation entry; if it should be a canvas overlay, use `App.tsx`'s panel branch. The current route/panel split is visible in [src/main.tsx](../../src/main.tsx#L28-L65) and [src/App.tsx](../../src/App.tsx#L2490-L2528).
- **INFERENCE:** Persist workflow entities, status transitions, and append-only history in SQLite through a dedicated Rust domain DB/IPC module, initializing it from `initialize_workspace_schema`; add shared TS/Rust DTOs and explicit allowed-transition validation. Source revisions provide the closest existing history pattern, while Forge audit rows show task-level audit metadata. [portable_db.rs](../../src-tauri/src/mind/portable_db.rs#L883-L891), [schema_source.sql](../../src-tauri/schema_source.sql#L70-L128), [forge/index.rs](../../src-tauri/src/forge/index.rs#L100-L118)
- **FACT / repo guidance:** The root engineering instructions require a spec before major features and specify the desired frontend service → Tauri command → DB flow. Follow [AGENTS.md](../../AGENTS.md) and add a feature specification under [specs/](../../specs) before implementation.

### TL;DR for the next engineer

- Vora Mind is a local-first Tauri desktop workspace with a React/TypeScript UI, SQLite persistence, and an optional standalone Rust worker API.
- The main durable DB is `vora_data.vora`; Rust initializes multiple domain schemas at startup, while worker queue data is separate.
- No migration framework, frontend test runner, permission map, or shared validation-schema library is currently present.
- For a stateful module, follow the Sources UI/service pattern, add a dedicated Rust domain DB/IPC module, and define explicit states/history instead of reusing Task Flow's boolean.
- Write the feature spec first, register any route/panel and Tauri commands, and add focused Rust tests plus a frontend test strategy before release.
