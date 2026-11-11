# Vora Target Gap Analysis

**Baseline:** Repository revision `b36fbd4`, audited 2026-11-11.  
**Target:** [VORA_MIND_SCIENTIFIC_ARCHITECTURE.md](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md).  
**Related evidence:** [current-state audit](./VORA_CURRENT_STATE_AUDIT.md), [existing data inventory](./VORA_EXISTING_DATA_INVENTORY.md).

## Status definitions

- **IMPLEMENTED_AND_VERIFIED** — requirement is represented by a complete code/data path and its required behavior was actually tested in this audit.
- **IMPLEMENTED_BUT_UNVERIFIED** — complete-looking code/data path was traced, but behavior was not runtime-tested here.
- **PARTIALLY_IMPLEMENTED** — adjacent capability exists, but one or more essential target semantics/contracts are absent.
- **MISSING** — no corresponding first-class domain implementation was found.
- **CONFLICTING_WITH_TARGET_ARCHITECTURE** — the current representation or repository guidance conflicts with target semantics or would be unsafe to treat as equivalent.
- **UNKNOWN** — source inspection cannot establish the state; requires real data, deployment, or product-owner evidence.

No behavior was tested for this audit, so no target requirement is marked `IMPLEMENTED_AND_VERIFIED`. Evidence of code and schemas is not an assertion that tests passed.

## Domain and system gap matrix

| Target requirement | Status | Current evidence and exact boundary | Required implementation/test evidence |
|---|---|---|---|
| Local-first Tauri/React/Rust shell and IPC | `IMPLEMENTED_BUT_UNVERIFIED` | Tauri commands, typed frontend service wrappers, Zustand state and Rust domain modules exist. Current structure differs from stale path guidance. | Add/retain a startup smoke test that opens an isolated workspace and exercises a representative IPC round trip; typecheck/build in CI. |
| Workspace schema evolution and integrity | `PARTIALLY_IMPLEMENTED` | Module initializers and some local migrations exist; no single migration ledger, ordered rollback/forward policy, or consistent connection factory was found. | Upgrade tests from each supported fixture; fail-fast migration tests; interruption/restart test; `foreign_key_check` and integrity checks; validate settings/data separation. |
| ResearchProject and ResearchDomain ownership | `MISSING` | No scientific project/domain entity or ownership foreign keys. Trace registry projects are file-backed and do not have scientific project semantics. | Test project lifecycle, membership/ownership, deletion/archival, cross-project isolation, workspace reopen, backup/restore. |
| ResearchGap and status/priority | `MISSING` | Question triage exists but is not a gap entity tied to a domain, evidence base, or research project. | Test gap create/edit/status transitions, project/domain constraints, prioritization and provenance links. |
| ResearchQuestion, triage, promotion | `PARTIALLY_IMPLEMENTED` | Questions, triage fields and transactional promotion to hypothesis exist. Question does not carry the full target’s project/gap/evidence relationships. | Test complete create/triage/promote/reopen/delete lifecycle and rollback; verify one-to-one/idempotency and expected source/evidence references. |
| Source acquisition and Source | `PARTIALLY_IMPLEMENTED` | Source metadata, hierarchical segments, extracts, links, revisions and UI/IPC path exist. | Test source ingest/validation, duplicate identity, stable URL/DOI handling, all write+revision operations in a transaction, and restoration of external source assets. |
| Immutable SourceVersion / capture and pinned citations | `MISSING` | `source_revisions` is an append-only operation log, not an immutable version of source contents. Extracts/links do not pin a source version. | Test version immutability, content hash, capture metadata, version-specific citation/extract references and retention of older versions after source updates. |
| Claim, Concept, Principle, Framework, Theory and typed relations | `MISSING` | No domain tables/contracts were found. Canvas links, Library groups and source polymorphic links do not satisfy typed scientific relations. | Contract tests for allowed relation types, endpoint types, cardinality, cycle policy, referential integrity and relation provenance; round-trip every public DTO. |
| Learning pathway, LearningObjective and MasteryRecord | `MISSING` | No learner/course/objective/mastery domain. Task Flow and journal are activity-management surfaces. | Test objective prerequisite graph, learner progress/mastery transitions, evidence attribution, independent learners and deletion/privacy behavior. |
| Phenomenon, Observation, Pattern and Idea | `MISSING` | Hypothesis data-entry JSON and chart/Trace content are not typed observational records. No units, measurement method, context or provenance contract exists. | Test typed values/units, exact precision and timezone, missingness, sample membership, provenance and reproducible export/import. |
| Hypothesis and lineage | `PARTIALLY_IMPLEMENTED` | H0/H1 text, variables, protocol, data-entry JSON, verdict, insights, parent/refinement and graduation fields are persisted. They are not project-scoped, fully typed, or backed by scientific evidence entities. | Test hypothesis lifecycle and lineage with typed variables, immutable protocol versions, provenance, explicit archive/delete behavior and failed transitions. |
| ExperimentDesign, ExperimentRun, execution and protocol versions | `MISSING` | Lab lock/execution flags and Trace `experiment` graph nodes do not provide design/run records, immutable inputs, status, outputs or reproducibility. | Test design version immutability, run transitions, exact input capture, failure/cancellation, idempotency, concurrent runs and input/output lineage. |
| Dataset, DatasetVersion, DatasetSample, partitions and metadata | `MISSING` | Market SQLite candles and Parquet tick files are separate chart/market mechanisms, not curated scientific dataset versions. | Test immutable manifests/hashes, schema and units, sample identity, split leakage, lineage, partition preservation, large-data backup and round-trip. |
| AnalysisRun and reproducible computation | `MISSING` | Worker queue and chart computations are operational/visual paths without scientific run provenance, code/environment capture, or immutable inputs/outputs. | Test deterministic replay, environment/code versions, input hashes, failure states, output lineage, cancellation and database lock avoidance. |
| EvidenceArtifact and Finding | `MISSING` | Hypothesis insights and source extracts are not typed evidence/finding records with claims, quality, provenance and review status. | Test evidence linkage, confidence/quality fields, finding lifecycle, contradictory evidence, reviewer attribution and deletion restrictions. |
| Model, ModelVersion, parameters and evaluation | `MISSING` | Library and board versions are visual/content snapshots; no scientific model or evaluation records. | Test immutable model revisions, parameter schema, train/validation/test lineage, metric definitions, reproducibility and safe model promotion. |
| Validation, Replication and scientific status | `MISSING` | Hypothesis verdict and Trace decision node are not validation/replication outcomes linked to a model or claim. | Test replication independence, protocol equivalence, outcomes, status derivation and relation to original inputs/evidence. |
| Typed relationship graph, lineage and auditability | `PARTIALLY_IMPLEMENTED` | Source links have polymorphic string targets; canvas/Trace/Forge have independent graph-like constructs. No unified, endpoint-typed scientific graph or universal relation policy. | Test foreign-key enforcement on every connection, relation endpoint/type validation, cascade/restrict rules, lineage traversal and audit-event atomicity. |
| Cross-project knowledge transfer | `MISSING` | No transfer operation or accepted-copy lineage. Existing Library promotion is not cross-project transfer. | Test transfer preserves origin and version, destination acceptance/rejection, no accidental mutation of origin, and reversible/traceable ownership. |
| Canvas, visual versioning, snapshots and Library | `PARTIALLY_IMPLEMENTED` | SQLite-backed canvas, object/library versions, snapshots and groups exist. They are reusable presentation/storage foundations, not target scientific entity versions or typed evidence. | Test round-trip of visual history separately from scientific relations; verify deleted/archived scientific references stay explainable. |
| Vora Trace | `CONFLICTING_WITH_TARGET_ARCHITECTURE` | File-backed graph and “experiment” nodes are operational trace records, not ResearchProject/ExperimentRun. Avoid data migration based on labels alone. | If retained, test explicit adapters and provenance without conflating Trace node identity, lifecycle, or storage with scientific entities. |
| Market/candle/chart system | `CONFLICTING_WITH_TARGET_ARCHITECTURE` | SQLite candles, Parquet tick reads, DuckDB aggregation and chart snapshots are market/chart data paths, not immutable target datasets/analyses. | Test an explicit import/manifest boundary, timestamps/units, source/licensing metadata, deterministic conversion and no mutation of original assets. |
| Backup/export/restore across durable state | `PARTIALLY_IMPLEMENTED` | SQLite online backup and JSON settings import/export exist. External files and Trace/worker state have separate roots and are not proven to be included. | Test a full workspace bundle against a fixture containing all referenced assets; restore into a clean path; verify hashes, FK checks and settings policy. |
| Frontend/backend domain contract alignment | `PARTIALLY_IMPLEMENTED` | Types are split among shared, Mind, source, Market and local DTO modules. No target domain contracts exist. | Add generated or compile-time DTO parity tests, IPC serialization tests and frontend behavior tests for every new entity lifecycle. |
| Current user DB/deployed-data compatibility | `UNKNOWN` | Source code documents only default/legacy paths; actual installed database versions and rows were not inspected. | Obtain approved, anonymized representative backup fixtures and inventory DB/app versions without modifying originals. |
| Existing browser/frontend test coverage | `UNKNOWN` | No frontend tests/script were found in inspected manifests/source. Runtime visual and restart behavior was not tested. | Establish a minimal repository-standard UI/unit/integration test harness; baseline key workflows before migration. |

## What exists, what can be extended, what must be introduced

### Existing and extendable foundations

- Extend the current SQLite workspace and Tauri IPC path only after a supported migration strategy and connection policy are defined.
- Reuse Source acquisition/segment/extract operations and their provenance for explicit `SourceVersion` and version-pinned citations; do not relabel revision rows as versions.
- Extend Questions/Hypotheses lifecycle operations with project/gap ownership and typed evidence links, preserving their current behavior behind migrations.
- Reuse board/library/snapshot UI for scientific visualization while keeping durable domain objects and typed relations in backend-owned contracts.
- Reuse market Parquet/DuckDB capability behind a dataset-ingestion boundary, not as the dataset model itself.
- Reuse worker infrastructure only after defining the scientific run contract and separating queue bookkeeping from immutable run inputs/results.

### Must be introduced

First-class contracts/storage/services for project/domain/gap; immutable source versions and citations; typed concepts/claims/principles/frameworks/theories and relations; learning/mastery; phenomena/observations/patterns/ideas; experiment design/run; dataset/version/sample; analysis run; evidence/finding; model/version/parameter/evaluation; validation/replication; and explicit cross-project transfer. Introduce a versioned migration framework, integrity policy, comprehensive fixture tests, and a complete backup/restore contract as prerequisites rather than incidental implementation details.

### Cannot be safely auto-mapped from current labels

Do not automatically convert: Trace “project” to ResearchProject; Trace “experiment” node to ExperimentRun; hypothesis checklist/data-entry JSON to experiment samples/observations; hypothesis insights to Findings; library groups/items to frameworks/theories/models; board snapshots to scientific entity versions; source revision events to SourceVersion; polymorphic source-link targets to typed relations; or chart candles/Parquet ticks to scientific DatasetVersions. Each requires explicit user/domain review or a deterministic, approved conversion rule.

## Recommended implementation sequence and acceptance checks

1. **Phase 0 — contracts and migration policy.** Resolve entity IDs, ownership, lifecycle, soft-delete/retention, relation endpoint rules, source-version semantics, time/unit rules, dataset asset packaging and supported DB versions. Produce fixtures and migration/backup plans. No schema rollout before these are approved.
2. **Integrity and migration foundation.** Establish migration ledger/ordering, one connection factory with consistent SQLite pragmas, fail-fast error semantics, `foreign_key_check`, backup-before-upgrade, rollback/forward recovery, and supported fixtures. Test migration from every supported schema and injected failure/restart.
3. **Provenance and typed relations.** Introduce immutable source versions, citations, typed relation endpoints and audit events, while preserving legacy IDs/records. Test old source/history round-trip, dangling-reference policy and relations under concurrent connections.
4. **Projects, domains, gaps, questions and hypotheses.** Add ownership and lifecycle around existing flows with reversible IDs/lineage. Test complete transitions, rollback, project isolation, existing question/hypothesis migration and UI restart.
5. **Experiment and data foundations.** Introduce versioned designs/runs, datasets/versions/samples and observation contracts. Test reproducibility, precision/units, immutable manifests, large external files, splits and failed/cancelled runs.
6. **Analysis, evidence, findings, models, validation and replication.** Add captured computation environment, durable artifacts and typed outcomes. Test deterministic replay, lineage closure, contradictory evidence, evaluation and replication.
7. **Learning and transfer.** Add objective/mastery and explicit cross-project accepted transfer after relation/identity/retention contracts are settled. Test independent progression, provenance and privacy.
8. **Consolidation and retirement.** Migrate only explicitly approved legacy mappings; retain compatibility readers/export or a documented archival path until all supported workspaces are verified.

The [implementation readiness report](./VORA_IMPLEMENTATION_READINESS.md) lists the decisions and prerequisites that block Phase 0.
