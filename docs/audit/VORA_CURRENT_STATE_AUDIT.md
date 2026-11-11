# Vora Current-State Architecture Audit

**Audit date:** 2026-11-11  
**Repository baseline:** `b36fbd4` (`chore(config): release v0.15.8`); worktree was clean at inspection start.  
**Purpose:** Evidence-based description of the implementation at this revision, compared with the [authoritative scientific architecture](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md). This is a source audit, not a runtime certification.

## 1. Executive conclusion

Vora is a local-first Tauri v2 desktop application with a React 19/TypeScript frontend, Rust domain modules, SQLite workspace persistence, a separate Rust worker queue, DuckDB-on-Parquet market-data reads, and file-backed Vora Trace state. There is meaningful persisted infrastructure for boards and versions, source/segment/extract provenance, inquiry triage, hypothesis/Lab workflows, market/chart state, and Forge code indexing.

It is **not yet a unified scientific research system**. No first-class ResearchProject, ResearchDomain, ResearchGap, SourceVersion, typed scientific Observation, ExperimentDesign/Run, DatasetVersion, AnalysisRun, Evidence, Finding, scientific Model/ModelVersion, Validation/Replication, Learning/Mastery, or cross-project transfer service was found in the implementation. Names such as Trace “experiment,” market candle, Lab data entry, board version, library group, or source link do not establish those target contracts.

The target’s most reusable foundations are the current workspace database and IPC path, source provenance, question/hypothesis transitions, board/library snapshots, selected transactional operations, and SQLite online backup. The major blockers are schema/migration ambiguity, unassigned legacy ownership, soft or polymorphic references, inconsistent connection setup, mixed timestamp conventions, and unknown coverage of deployed user database variants and external files.

No production behavior was changed, no database was opened, and no tests/builds were run for this audit. **No runtime test result is asserted.**

## 2. Evidence standard and target authority

The root [VORA_MIND_SCIENTIFIC_ARCHITECTURE.md](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md) identifies itself as the authoritative target and explicitly separates requirements from current implementation. Its target spans three pathways: acquisition/mastery; investigation/development; and first-principles discovery. The `specs/skdda/01`–`08` documents provide proposed contracts, relationship/lifecycle policies, and migration strategy; they are not proof those features exist.

The audit traced representative source and research workflows through UI, TypeScript service/store, Tauri command, Rust domain operation, and schema initializer. Findings distinguish a code path observed in source from behavior exercised at runtime. “Verified” in the status matrix means repository evidence was traced; because tests were not run, it does not mean runtime acceptance passed.

## 3. Actual architecture and persistence

| Layer | What is present | Evidence |
|---|---|---|
| Desktop shell | Tauri v2, Rust 2021, React 19, TypeScript, Vite, React Router and Zustand. Routes include chart, sources, snapshots, and home; planning/research surfaces are panels within the App shell. | [package.json](../../package.json), [src/main.tsx](../../src/main.tsx), [src/App.tsx](../../src/App.tsx) |
| Frontend domain surfaces | Canvas/cards/library, source pages, Questions Board and triage, hypothesis/Lab detail, journal, task flow, chart/market, snapshots, documents/media, Trace, Forge. Domain types are split across `src/types`, `src/mind/types`, and feature/service modules. | [src/shared/layout/](../../src/shared/layout), [src/pages/](../../src/pages), [src/mind/types/index.ts](../../src/mind/types/index.ts), [src/types/index.ts](../../src/types/index.ts) |
| Frontend persistence path | Most domain calls use typed service wrappers invoking Tauri, with Zustand actions/caches. The main store persists only selected browser state (including cards and executed tasks); question/hypothesis durability is through the SQLite service path. | [src/mind/services/dbService.ts](../../src/mind/services/dbService.ts), [src/source/services/sourceService.ts](../../src/source/services/sourceService.ts), [src/shared/store/useAppStore.ts](../../src/shared/store/useAppStore.ts) |
| Rust IPC and domain | Tauri command registration and a substantial set of command bodies are in `lib.rs`; source and market have separate `ipc.rs`; domain persistence lives under `mind`, `source`, `market`, and `forge`. There is no root `src-tauri/src/resolver.rs` or root `src-tauri/src/db.rs`. | [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs), [src-tauri/src/mind/](../../src-tauri/src/mind), [src-tauri/src/source/](../../src-tauri/src/source), [src-tauri/src/market/](../../src-tauri/src/market), [src-tauri/src/forge/](../../src-tauri/src/forge) |
| Main workspace database | Tauri uses AppData `vora_data.vora`, stores a shared `rusqlite::Connection` in `AppState`, and copies the legacy `vora_workspace.db` when the fixed file does not exist. A prior configurable database selector is handled by migration code. | [lib.rs](../../src-tauri/src/lib.rs), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs) |
| Runtime schema | `initialize_workspace_schema` composes Rust initializers for canvas, task flow, journal, questions, hypotheses, market, source, Forge, and portable tables. Source and market include `schema_source.sql` and `schema_market.sql`; canvas and other Mind schemas are inline Rust SQL. There is no coordinated numbered migration ledger or observed `PRAGMA user_version` convention. | [portable_db.rs](../../src-tauri/src/mind/portable_db.rs), [source/db.rs](../../src-tauri/src/source/db.rs), [market/db.rs](../../src-tauri/src/market/db.rs) |
| Other persistence | Worker queue is a separate `vora_worker_queue.db`; Trace registry/graph/decision files live under AppData; settings are a sibling `.settings.json`; source documents/media and market Parquet remain filesystem assets. DuckDB candle reads are in-memory. | [vora_worker.rs](../../src-tauri/src/bin/vora_worker.rs), [vora_trace.rs](../../src-tauri/src/vora_trace.rs), [portable_db.rs](../../src-tauri/src/mind/portable_db.rs), [market/local_db.rs](../../src-tauri/src/market/local_db.rs) |

### 3.1 Database initialization, connections, and migrations

The application startup resolves the fixed AppData database, copies an older default database when needed, runs module initializers, enables foreign keys on the shared connection, selects `journal_mode=DELETE`, initializes settings, and places the connection and path in `AppState`. The existing “workspace selection” is therefore primarily a legacy selector migration; the active app path is not a user-selected arbitrary SQLite database in the inspected startup path.

The module initializers are mostly `CREATE TABLE IF NOT EXISTS` plus local `PRAGMA table_info`/`ALTER TABLE` checks. Market also uses `_vora_meta` markers and historical one-time data changes. These mechanisms are not one ordered migration system, do not establish one supported-from version matrix, and do not make all initializers atomic as a unit. Some historical Market migration statements deliberately ignore errors. SQLite foreign-key enforcement is connection-scoped: startup configures the shared connection, but `open_default_db_connection()` opens another connection and calls Market schema initialization without setting `foreign_keys`. This matters to future scientific relations even though many current Market tables do not declare FKs.

`src-tauri/src/schema.sql` is an additional legacy/duplicated SQL file, not the current runtime schema entrypoint. The target spec’s current-state prose and the root `AGENTS.md` still direct readers toward that file for source/schema ownership; runtime source SQL is actually included from `src-tauri/schema_source.sql`, and Market SQL from `src-tauri/schema_market.sql`. The static file also has older canvas/source definitions. Do not edit or treat it as the sole runtime schema without first resolving this ambiguity.

### 3.2 Backup, import, and restore

SQLite online backup is used for database export and for selected legacy-file migrations. Database import stages a copy, runs `PRAGMA quick_check`, checks for a small set of workspace tables, initializes schema, then replaces the active file with rollback handling. Settings JSON is handled separately and can be exported/imported independently. The inspected import validation does **not** establish a `PRAGMA foreign_key_check`; database backups do not establish that media, document, Trace, or Parquet files referenced from the database are packaged or restored. Existing Rust tests cover portions of backup/settings/import behavior, but were not run here. See [portable_db.rs](../../src-tauri/src/mind/portable_db.rs).

## 4. Representative end-to-end workflows

### 4.1 Source acquisition and extraction

`SourcesPage` and its source list/detail/add/segment UI call `src/source/services/sourceService.ts`; that service invokes the nine source IPC commands in `source/ipc.rs`. The IPC layer delegates to `source/db.rs`, which initializes the included source SQL plus hierarchy migration. A source has mutable metadata; it can have hierarchical segments and typed-transform extracts with parent/superseding IDs, and links from extracts to typed-by-string targets. `source_revisions` records selected source operations and SQLite triggers reject UPDATE/DELETE of revision rows.

This is a real source registry and extraction/provenance workflow, but no immutable edition/capture identity or source-version-pinned citation was found. Revision history is not SourceVersion. The target claims/concepts/evidence lifecycle is not implemented. Some source writes insert/update domain data and then record a revision without a single encompassing transaction, so the revision log cannot be assumed atomic for every multi-write path. Code locations: [sourceService.ts](../../src/source/services/sourceService.ts), [source/ipc.rs](../../src-tauri/src/source/ipc.rs), [source/db.rs](../../src-tauri/src/source/db.rs), [schema_source.sql](../../src-tauri/schema_source.sql).

### 4.2 Questions to hypotheses and Lab

Questions persist to `questions`; the current initializer adds triage fields for status, impact, feasibility, urgency, score, notes, selection, and a promoted hypothesis ID. The Questions Board loads/creates/edits/deletes questions through `useAppStore` and `dbService`. Triage and promotion are backend-supported. `promote_inquiry` checks selected/triaged state and inserts the hypothesis and updates the question in one transaction. Hypothesis rows contain H0/H1 and variable/metric/threshold strings, protocol and lock/execution fields, raw-data and analysis text, verdict, insights, JSON checklist entries, source card/highlight strings, library definition links, snapshot IDs, and parent/refinement IDs. Hypothesis operations include protocol lock/execution, data entry, verdict, child hypothesis and Library graduation.

This is a persisted hypothesis/Lab workflow, not an experiment system. No project ownership, structured experiment design/run, typed observation, dataset/version, analysis execution record, evidential artifact, or finding was found. Lab checklist JSON has a UI-defined schema but is not a general sample/observation contract. Question promotion and hypothesis lineage rely partly on soft references; question deletion is supported. Relevant code: [questions.rs](../../src-tauri/src/mind/questions.rs), [hypothesis.rs](../../src-tauri/src/mind/hypothesis.rs), [HypothesisDetail.tsx](../../src/shared/layout/HypothesisDetail.tsx), [useAppStore.ts](../../src/shared/store/useAppStore.ts), [dbService.ts](../../src/mind/services/dbService.ts).

### 4.3 Board, Library, and Vora Trace

Canvas boards, objects/cards, object versions, board versions, Library item groups/versions, snapshot deltas/checkpoints, and text/media links are SQLite-backed in the main workspace database. They provide spatial authoring and visual history. Canvas edges and snapshot links are not canonical scientific relations; group and board snapshots are not model/framework versions. Board/history operations have selected transaction boundaries and Rust unit tests, but no tests were executed in this audit.

Trace is separate from the scientific database. Its typed graph has state/decision/experiment/rejected node kinds and graph edges; its graph and Markdown decisions are file-backed in AppData, with a registry YAML and legacy copy path. In non-Tauri browser mode its Zustand store starts from `mockGraph`; failed disk loads produce an empty graph plus an error state. Trace’s “project” registry and “experiment” node are not `ResearchProject` and `ExperimentRun`. See [vora_trace.rs](../../src-tauri/src/vora_trace.rs), [traceService.ts](../../src/mind/services/traceService.ts), [traceStore.ts](../../src/features/vora-trace/store/traceStore.ts), [mockGraph.ts](../../src/features/vora-trace/data/mockGraph.ts).

### 4.4 Market data, charting, snapshots, and worker

Market has SQLite persistence for candles, boxes, scales, symbol origins, drawing templates and alerts. Separately, chart candle fetches call `mkt_fetch_local_candles`, resolve a symbol directory of Parquet tick files, open an in-memory DuckDB connection, and aggregate OHLCV. Thus market has both database-backed candle facilities and a Parquet-backed local chart read path; neither supplies a dataset registry, immutable version manifest, sample split, experiment run, or analysis provenance. Chart snapshots use portable SQLite item/snapshot commands; drawing templates/alerts and market state are not scientific findings.

The Rust worker is an independently launched processing/coordination service with its own relative-path queue database, statuses, locks, heartbeat and logs. It is not the application’s scientific ExperimentRun/AnalysisRun engine. A future adapter would need explicit scientific inputs/outputs, provenance, immutable run identities, and isolation from SQLite lock-holding computation.

## 5. Target compliance snapshot

| Capability | Classification | Repository evidence and boundary |
|---|---|---|
| Tauri/React/Rust local-first shell and typed IPC pattern | `IMPLEMENTED_BUT_UNVERIFIED` | Commands, services, stores and DB modules exist and representative paths were traced. No runtime smoke test was run. |
| Durable board, object and library snapshots/versions | `IMPLEMENTED_BUT_UNVERIFIED` | Tables, Rust operations, DTOs and UI exist. These preserve visual/content history, not scientific entity/version semantics. |
| Source → segment → extract and limited revision trail | `PARTIALLY_IMPLEMENTED` | Real persistence and UI path; no source-version citation contract and no universal relation integrity. |
| Inquiry triage and transactional promotion to hypothesis | `IMPLEMENTED_BUT_UNVERIFIED` | Current schema, Rust promotion operation, service/store/UI path and tests are present. No test execution in this audit. |
| Hypothesis/Lab protocol, checklist JSON, execution marker, verdict | `PARTIALLY_IMPLEMENTED` | Durable workflow exists; it does not create structured experiment/run/data/analysis/finding entities. |
| Market data and chart snapshots | `IMPLEMENTED_BUT_UNVERIFIED` | Market schemas, DB operations and Parquet/DuckDB path exist; they are not scientific dataset versioning or evaluation. |
| Trace graph/project registry | `IMPLEMENTED_BUT_UNVERIFIED` | UI, Rust DTOs and file persistence exist. Mapping it to scientific projects/experiments conflicts with target semantics. |
| Research project/domain/gap, SourceVersion, learning/mastery | `MISSING` | No first-class implementation was found in Rust schema/domain or frontend contracts. |
| Phenomena, typed observations/patterns/ideas, experiment design/run | `MISSING` | Hypothesis JSON and Trace nodes are not equivalent contracts. |
| Dataset/version/sample, AnalysisRun, Evidence, Finding | `MISSING` | No dedicated schema, Rust domain operations or frontend services found. |
| Model/specification/version/parameters/evaluation, Validation, Replication | `MISSING` | Board/Library versions and hypothesis verdicts are not these entities. |
| Universal typed scientific relationship graph and project transfer | `MISSING` | Existing links are source-specific with polymorphic target strings; other graphs are canvas, Trace, or Forge-specific. |
| Current repo instructions/schema claims as architectural map | `CONFLICTING_WITH_TARGET_ARCHITECTURE` | Root `AGENTS.md` names `src-tauri/src/db.rs`, root `resolver.rs`, `src/store`, and `src/services/dbService.ts`; these do not describe the inspected implementation. It also names the static schema as runtime owner. |

No target capability is classified `IMPLEMENTED_AND_VERIFIED`: the audit did not execute tests or an app/database round trip. `IMPLEMENTED_BUT_UNVERIFIED` means an end-to-end-looking source path was traced, not that tests passed.

## 6. Tests, build, and documentation evidence

Rust module tests exist in canvas, questions, hypothesis, source, market, and portable-database modules; Cargo also declares integration tests, many centered on Forge/worker/semantic indexing. The relevant tests demonstrate focused behaviors in source, including some migration/idempotence, transactions, DTO/domain rules, and backup handling. No frontend test files or frontend test script were found in the inspected source/manifests. `pnpm build` runs TypeScript and Vite; Cargo tests/checks are not stated to have run in this audit. Coverage percentage and full cross-domain workflow coverage are unknown.

Some existing audit documents require caution: `docs/audit/RECON_INQUIRY.md` describes an earlier question model before the current triage columns, and `docs/audit/RECON.md` lists 14 canvas tables while the current `canvas_db.rs` declares 18. This report and the companion inventory use the currently inspected schema entrypoints instead.

## 7. Uncertainties not resolvable from source inspection

- Actual user database count, file paths beyond the default/legacy code paths, schema shapes, row counts, integrity and deployed version distribution.
- Whether every existing install has all module initializers applied successfully and whether partial initialization exists.
- Runtime behavior of UI flows after restart and on each supported installation.
- Backup coverage and recoverability of documents, media, tick Parquet, Trace files, and other external paths.
- Whether the worker is deployed, its actual working directory, and any external scheduling/integration.
- Effective database foreign-key settings on every process/connection and whether legacy rows contain dangling references.
- Exact scientific meaning/units/timezone for all legacy free-text/JSON values and all integer timestamps.

No user DB/files were inspected to resolve these questions. See [data inventory](./VORA_EXISTING_DATA_INVENTORY.md), [gap analysis](./VORA_TARGET_GAP_ANALYSIS.md), [migration risks](./VORA_DEPENDENCY_AND_MIGRATION_RISKS.md), and [readiness](./VORA_IMPLEMENTATION_READINESS.md).
