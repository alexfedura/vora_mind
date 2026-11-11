# Vora Scientific Architecture Implementation Readiness

**Baseline:** Repository revision `b36fbd4`, audited 2026-11-11.  
**Authoritative target:** [VORA_MIND_SCIENTIFIC_ARCHITECTURE.md](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md).  
**Audit companions:** [current-state audit](./VORA_CURRENT_STATE_AUDIT.md), [target gap analysis](./VORA_TARGET_GAP_ANALYSIS.md), [existing data inventory](./VORA_EXISTING_DATA_INVENTORY.md), [dependency and migration risks](./VORA_DEPENDENCY_AND_MIGRATION_RISKS.md).

## 1. Readiness decision

**Readiness for scientific schema or production migration: NOT READY.**  
**Readiness for a scoped Phase 0 discovery/design kickoff: CONDITIONAL.**

There is a substantial working application and several reusable persistence workflows, but no verified inventory of deployed user DB variants or external assets, no unified migration ledger, no approved policy for many scientific identity/ownership/retention decisions, and no executed test baseline for this audit. Proceeding directly to domain-table implementation or an automatic data conversion would force guesses about user data and can break existing behavior.

Phase 0 may begin as a requirements and evidence-gathering phase if the prerequisites below are met. Its design decisions must be recorded and approved before a schema/migration implementation begins. No tests, builds, or migrations were executed as part of this audit; none are represented as passing.

## 2. What actually exists

- Tauri v2 + React 19/TypeScript application with Rust domain modules and SQLite workspace persistence.
- Canvas objects, boards, Library snapshots/versions, documents, highlights, media references/slices and visual snapshots.
- Source registry with segments, extracts, polymorphic links and append-only revision rows.
- Questions with triage fields and a transactional promotion-to-hypothesis path.
- Hypotheses/Lab records with H0/H1 text, protocol/checklist JSON, raw-data/analysis text, verdict, insights, lineage and Library promotion fields.
- Market SQLite tables and a separate Parquet/DuckDB local chart read path.
- Forge code indexing/lineage, independent worker queue DB, and file-backed Vora Trace graph/decision records.
- SQLite online DB backup/import and a separate settings JSON workflow.

These are real current features, but they are not one coherent scientific provenance graph. The exact tables, FKs, timestamps, external roots and caveats are listed in the [data inventory](./VORA_EXISTING_DATA_INVENTORY.md).

## 3. What is incomplete or conflicts with the target

The target’s core domain entities are absent or only approximated: ResearchProject, ResearchDomain, ResearchGap, immutable SourceVersion, typed scientific claims/concepts/principles/frameworks/theories, learning/mastery, phenomena/observations/patterns/ideas, ExperimentDesign/Run, DatasetVersion/Sample, AnalysisRun, EvidenceArtifact, Finding, scientific Model/ModelVersion, Validation/Replication, typed relations, and cross-project transfer.

Current Lab JSON/strings, Trace graph nodes, board/Library history, source operation revisions, chart candles, worker tasks and Forge lineages must not be relabeled as these entities. Root `AGENTS.md` and prior recon notes name stale source paths/schema ownership; runtime initialization is through domain modules, `schema_source.sql`, and `schema_market.sql`, not `src-tauri/src/db.rs`, a root resolver, or the duplicate `src-tauri/src/schema.sql`.

## 4. What must be preserved and what may be extended

### Preserve

- Original workspace DB bytes and stable IDs during investigation and migration trials.
- All source, segment, extract, link and revision rows, including unrecognized/soft-linked targets.
- Hypothesis/question data, JSON blobs, status history, board/library snapshots, visual history and soft references.
- Original market DB data and Parquet tick assets; do not assume one is a regenerated copy of the other.
- External media/document paths and hashes; settings JSON; Trace files; and worker DB/logs when in deployment scope.
- User-authored free text and unknown JSON fields. Never silently discard unrecognized values.

### Extend after approval

- Existing Tauri/SQLite boundary and service patterns, with a centralized connection/migration policy.
- Source provenance operations with immutable content/capture versions and citations.
- Question/hypothesis lifecycle with explicit project/gap ownership and typed evidence lineage.
- Canvas/Library/Trace/chart as UI or operational integrations over new canonical domain contracts, without conflating their existing records.
- Worker queue as a possible executor only after a separate immutable scientific run contract is established.

### Introduce

Approved first-class target entity contracts, typed relations, lifecycle/retention rules, versioned migrations, complete fixture/restore tests, and an external-asset backup manifest. See the [gap matrix](./VORA_TARGET_GAP_ANALYSIS.md) for per-domain contracts and acceptance tests.

### Cannot safely auto-migrate

Trace project/experiment/evidence strings; question-as-gap assumptions; hypothesis checklist/data-entry JSON as samples/observations; insights as findings; Library/board versions as model/framework versions; source revision events as SourceVersion; polymorphic links as typed relations; Market candles/Parquet ticks as DatasetVersions; and worker tasks as ExperimentRuns/AnalysisRuns. Keep them intact and either leave them legacy, map with explicit review, or create a deterministic audited adapter once semantics are settled.

## 5. Prerequisites before Phase 0 kickoff

These are prerequisites to starting Phase 0 as an accountable discovery effort; they are not implementation decisions that should be guessed in advance:

1. **Authority and scope:** product/research owner confirms which revision of the root architecture spec is authoritative, which pathways/domains are in the first release, and what is explicitly out of scope.
2. **Decision-makers:** name accountable owners for scientific semantics (e.g. evidence, experiment, dataset, model, mastery), data retention/privacy, and final migration approval.
3. **Data access authorization:** decide whether approved, anonymized copies of real workspaces can be inspected. If not, document that readiness is based on source only and defer compatibility claims; do not inspect production/user data without authorization.
4. **Support and deployment envelope:** identify supported app/DB versions, OS platforms, upgrade guarantees, backup/restore expectations, and whether worker/Trace/Parquet assets are in the supported workspace.
5. **Acceptance bar:** agree what constitutes successful preservation, acceptable conversion loss, migration rollback, and a user-visible manual resolution workflow for ambiguous records.
6. **Change boundary:** confirm that Phase 0 remains specification/inventory work and does not modify production code, perform migrations, or rewrite existing schema ownership until explicit approval.

If these are satisfied, Phase 0 can resolve the technical contracts below. The lack of user DB fixtures can be accepted as a documented limitation only if the product owner explicitly accepts that migration readiness remains blocked.

## 6. Decisions required during Phase 0, before implementation

| Decision | Why required | Evidence/output needed |
|---|---|---|
| Workspace, ResearchProject and ResearchDomain boundaries/ownership | Existing app uses a fixed local workspace, while Trace and worker have separate project/path concepts. | Approved context diagram, ownership cardinality, IDs and isolation rules. |
| Entity identity and version semantics | Source revisions, board versions, content versions and project identities are not interchangeable. | Entity catalog, stable ID strategy, immutable/versionable fields, aliases and lifecycle examples. |
| Typed relationship contract | Current soft/polymorphic links lack endpoint constraints. | Allowed relation matrix, directionality, cardinality, endpoint type, provenance, cycle rules and delete policy. |
| Source acquisition, capture and citations | Mutable paths/metadata and revision logs cannot identify an immutable cited edition. | URL/file capture policy, hashes, timestamps, license/rights fields, source version retention and citation resolution. |
| Units, timestamps, precision and missing values | Current timestamps use different units and scientific measurements lack a unified value/units model. | Canonical time convention, timezone, numeric precision, unit vocabulary/conversion provenance, null/missingness policy. |
| Experiment and run lifecycle | Lab execution flags and worker queue are not reproducible run records. | Design/run state machine, immutable inputs, environment/code capture, cancellation/failure/retry rules and output contracts. |
| Dataset storage and asset packaging | Market data is both SQLite and Parquet; media and other files are external. | Dataset manifest/hash, in-DB vs file storage, partitioning, size limits, backup/restore and licensing decisions. |
| Evidence/findings/models/validation | Current verdict/insight fields are informal and not independently versioned. | Review, confidence, conflicting evidence, model version, metric/evaluation, validation and replication contracts. |
| Deletion, archive, privacy and cross-project transfer | Existing cascade/soft-reference patterns are inconsistent. | Per-entity retention policy, user delete semantics, project export/transfer acceptance and audit requirements. |
| Migration framework and compatibility window | Module initializers are not a unified ordered migration system. | Supported-from matrix, schema ledger, migration transaction policy, fail-fast rules, backup/rollback and legacy-read/export horizon. |
| Frontend/backend ownership and testing | Types and services are split; no frontend test baseline was found. | Contract source of truth, IPC DTO parity, behavior tests, error UX and release gate. |

## 7. Recommended implementation order with required gates

The order below follows the intent of the authoritative architecture while placing migration/integrity work ahead of feature tables. Each gate is a stop condition; a failed check means no progression to production migration.

| Order | Workstream | Exit gate / concrete tests |
|---|---|---|
| 0 | Spec and data contract | Approved entity/relationship/lifecycle/retention/unit decisions; architecture docs agree on current runtime ownership and target requirements; ambiguous mappings are explicitly not automated. |
| 1 | Schema and migration foundation | Version ledger and connection factory; fixture matrix for supported versions; migration repeatability, injected failure/restart, backup/rollback, `quick_check`, `foreign_key_check`, application-level references, and no data loss across row/hash inventory. |
| 2 | Source provenance and typed relations | SourceVersion and version-pinned citations; preserve old revision records/IDs; test revision transaction atomicity, content hash, dangling old links, endpoint constraints and old source/extract round trip. |
| 3 | Project/domain/gap/question/hypothesis | Add ownership and lifecycle around existing flows; test legacy migration, question triage/promotion rollback/idempotency, lineage, project isolation, UI restart and delete/archive policy. |
| 4 | Experiment/data/observation | Versioned designs/runs, typed observations, DatasetVersion/Sample; test units/time precision, immutable input manifests, splits, hashes, large files, cancellation/failure and restore. |
| 5 | Analysis/evidence/finding/model/validation | Capture code/environment/input/output provenance and typed results; test deterministic replay, artifact integrity, contradictory evidence, evaluation and independent replication. |
| 6 | Learning/mastery and transfer | Implement after stable identity/relations; test prerequisite graph, learner-specific progress/privacy, accepted transfer, origin immutability and provenance. |
| 7 | Legacy compatibility and retirement | Convert only approved deterministic mappings; test every fixture and manual-review path; retain legacy reader/export/archive until support owners approve retirement. |

## 8. Required test baseline

Prior to the first implementation change, establish and record a baseline for:

- Rust unit/integration suites for source, questions, hypotheses, canvas/persistence, Market, portable DB and worker/Forge where they are affected.
- `cargo check --manifest-path src-tauri/Cargo.toml` and the repository’s frontend typecheck/build commands.
- A new frontend test baseline for high-value workflows: source add/extract/reopen, question triage/promote/reopen, hypothesis protocol/data/verdict/reopen, board/history/reopen, import/export/restore, and save-error visibility.
- Fresh DB and representative historical DB fixture migrations with pre/post schema/row/hash comparison.
- Full workspace backup/restore including sidecars and declared external assets.
- Runtime foreign-key/integrity checks on each supported connection path.

The audit did not execute these checks. Existing Rust test source is evidence of intended coverage, not a result or proof of coverage completeness.

## 9. Final audit answers

- **What exists:** a local-first desktop workspace with persisted canvas, source/extract, question/hypothesis, chart/Market, document/media, Forge and operational Trace/worker subsystems.
- **What is incomplete:** the scientific entities, typed graph, immutable dataset/run/evidence/model lineage, learning/mastery, and cross-project transfer required by the target.
- **What conflicts with target semantics:** treating Trace, chart/Market, canvas/Library, Lab JSON, source revision events or worker tasks as canonical scientific domain objects; stale repository guidance also conflicts with current runtime layout.
- **What must be preserved:** all user DB rows/IDs/history/blobs and separate settings/assets/Trace/worker state unless product explicitly scopes them out and users approve.
- **What can be extended:** source provenance, question/hypothesis transitions, visual history, DB/IPC services, market data ingestion and worker execution—only behind approved contracts and migration controls.
- **What must be introduced:** target domain records, typed relations, stable immutable versions, complete migration/connection integrity, test fixtures and workspace-level backup/restore.
- **What cannot safely be migrated automatically:** any mapping whose scientific meaning, ownership, units, version or lifecycle is not explicit in current data. See the mapping table in the [risk report](./VORA_DEPENDENCY_AND_MIGRATION_RISKS.md).
- **What blocks Phase 0 kickoff:** authority/scope, accountable domain/data owners, data-access authorization, support/deployment envelope, acceptance criteria, and confirmation of the Phase 0 change boundary. Technical entity and migration decisions must then be completed in Phase 0 before implementation.
