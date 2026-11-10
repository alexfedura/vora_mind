# SKDDA Phase 0 — Domain Gap Analysis

This matrix compares the target in [VORA_MIND_SCIENTIFIC_ARCHITECTURE.md](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md) with the audited repository. Status means:

- **EXISTING:** actual implementation in code/persistence for the stated scope.
- **EXISTING BUT REQUIRING EXTENSION:** related working capability exists but does not satisfy the target contract.
- **NEW:** no meaningful implementation was established.
- **UNVERIFIED:** evidence is insufficient to classify runtime behavior; listed separately where appropriate.

Frontend integration is described independently from backend/schema evidence. A row is never called end-to-end merely because UI and backend names exist.

## 1. Domain matrix

| Target domain | Status | Repository/backend and DB evidence | Frontend integration | Gap, dependency, recommended approach |
|---|---|---|---|---|
| Knowledge Sources | EXISTING BUT REQUIRING EXTENSION | `sources` table in `schema_source.sql`; `Source` and `SourceKind` in `source/model.rs`; CRUD in `source/db.rs`. | Source list/detail/add pages under `src/pages/sources/`; adapter `src/source/services/sourceService.ts`. | Current source identity is mutable and lacks distinct immutable edition/version identity. Extend the existing registry; preserve source IDs. |
| Source Versions | NEW | No source-version table/model established; source revision events track mutations, not a distinct edition/retrieval snapshot. | No explicit version-selection workflow verified. | Add immutable source edition/capture record and bind citations to it. Phase 1 provenance dependency. |
| Source Segments | EXISTING BUT REQUIRING EXTENSION | `segments` with source FK, locator fields, parent and order; `source/db.rs::migrate_segments_tree`. | Source segment tree/modal components exist. | Locator is not pinned to source version; extend rather than duplicate. |
| Extracts | EXISTING BUT REQUIRING EXTENSION | `extracts` includes transformation kind, content, confidence, creator/review, parent/superseded refs. | Source detail and segment extraction flows exist. | Add source-version binding, machine tool/version provenance, durable review lifecycle and claim distinction. |
| Source Claims | NEW | No first-class `KnowledgeClaim` table. Extracts may contain text but do not represent typed claims. | Cards/notes can carry free text, not canonical claim lifecycle. | Add attributable claims and citations; avoid converting all extracts automatically. |
| Knowledge Concepts | EXISTING BUT REQUIRING EXTENSION | No canonical concept table established; library/cards/extracts can represent content but not stable concept identity. | Canvas cards/Library organize content. | Add stable identity, aliases, definition/version/provenance; retain card as presentation. |
| Knowledge Principles | NEW | No first-class principle entity. | No dedicated principle workflow established. | Add typed principle as reusable knowledge record with conditions and evidence. |
| Knowledge Frameworks | EXISTING BUT REQUIRING EXTENSION | `library_item_groups` are reusable snapshots, not scientific framework semantics. | Library group UI exists. | Formal framework identity and composition must be separate from generic Library group. Can reuse editor/presentation. |
| Framework Versions | EXISTING BUT REQUIRING EXTENSION | `library_item_group_versions` and board versions preserve JSON snapshots. | Version/history UX for groups/boards exists. | Exact scientific membership, claims/evidence, publication, change classes, provenance not guaranteed. |
| Learning Objectives | NEW | No table/model/UI. | None found. | New learning domain; depends on framework version. |
| Learning Assessments | NEW | No assessment definitions/attempt schema. | None found. | Add versioned rubrics and attempts; separate from hypothesis testing. |
| Competencies / Mastery | NEW | No competency or mastery model. | No workflow. | Add explainable evidence-based mastery state; no completion shortcut. |
| Research Projects | NEW | Trace has projects/ecosystem registry, but no scientific project table. | Trace sidebar is not an SKDDA project workspace. | New scientific project identity; do not reuse Trace project IDs without an explicit relation. |
| Research Domains | NEW | No domain taxonomy table established. | None found. | New reusable domain links to projects/phenomena/frameworks. |
| Research Gaps | NEW | No first-class gap record. | Inquiry queue can hold questions but not evidence-backed gap context. | Add gap with literature/prior-knowledge provenance before project questions. |
| Research Questions | EXISTING BUT REQUIRING EXTENSION | `questions` table plus triage; status/impact/feasibility/urgency/selected/promotion ID added idempotently in `mind/questions.rs`. | Questions board/triage UI; store/service operations exist. | No project/domain/gap ownership FK; question deletion and promotion link are soft/application-level. Extend with explicit safe legacy assignment. |
| Phenomena | NEW | No table/model established. | None found. | Add scoped phenomenon linked to project/domain/questions and literature. |
| Observation Plans | NEW | No schema. | No workflow. | Add design/plan before observation collection; support lock/preregistration. |
| Observations | EXISTING BUT REQUIRING EXTENSION (weak analogue only) | `hypothesis_data_entries.data_json` can persist timestamped hypothesis-scoped JSON. | Lab data-entry UI exists. | Not general observations, no units/sample/instrument/source, dataset, immutable correction, or project context. Keep only as legacy input; do not map automatically. |
| Patterns | NEW | No pattern entity. | No structured workflow. | Add exploratory result linked to dataset/analysis; distinguish from finding. |
| Ideas | NEW | Extract `extracted_idea` kind exists but no generic idea entity/lifecycle. | Ideas may exist as cards/text. | Existing extract kind is not equivalent; new idea object with origin links. |
| Hypotheses | EXISTING BUT REQUIRING EXTENSION | `hypotheses` with H0/H1, IV/DV/metric/threshold, status/verdict, protocol/data/analysis text, insights, source IDs, parent/refinement; SQL in `mind/hypothesis.rs`. | Lab list/detail, promotion, source creation, child creation, canvas node. | Text fields, missing project FK and complete experimental/evidence linkage; statuses/immutability constraints incomplete for target. Preserve existing rows and workflow. |
| Experiment Designs | NEW | No experiment-design schema. Hypothesis protocol text/flowchart is not a versioned design with factors, sampling, preregistration and plan. | Protocol editor is partial adjacent UI. | Add versioned design entity referencing hypothesis/model. |
| Experiment Runs | NEW | No `experiment_runs`. Trace experiment node is not a run. | Trace graph node and Lab execution UI do not prove run-level data contract. | Separate design from run; immutable input/output/deviations. |
| Datasets | NEW | No dataset registry. Market candles/Parquet files are data sources, not project-pinned scientific datasets. | Market/chart views offer data access, not dataset version control. | Add logical dataset and immutable version manifest/checksum. |
| Dataset Versions | NEW | No version manifest/split entity. | None. | Pin exact input versions to runs/analyses. |
| Data Samples / TradeSample | NEW | No scientific sample entity; candles/market boxes are domain storage. | Chart displays market records. | Use shared DataSample contract with market-specific typed extension/adapter; no second research DB. |
| Analysis Runs | NEW | `data_analysis` is text on hypothesis; no execution record, code/env/inputs/seed. | Lab supports analysis text/AI prompt compilation. | Add reproducible immutable AnalysisRun. LLM-generated text is not an analysis execution. |
| Evidence | EXISTING BUT REQUIRING EXTENSION (weak links/notes) | `links.link_kind` includes evidence/support/contradict semantics; hypothesis `evidence_notes` is text. | Source links and Lab notes exist. | No typed evidence target, scope, polarity constraints, quality, dataset/run provenance. |
| Findings | NEW | No finding table/lifecycle. Hypothesis verdict is not a reusable finding. | Verdict UI exists but not finding management. | Add scoped conclusion with evidence and uncertainty. |
| Models | NEW | No scientific model registry. Board/library definitions are not models. | No model lifecycle UI. | Create Model independent from hypothesis. |
| Model Specifications | NEW | No spec contract. | None. | Versioned structured model definition with assumptions and interfaces. |
| Model Versions | NEW | Board/library versions do not track scientific model versions or contributing experiments. | Generic history UI only. | Immutable version records with change type and evidence links. |
| Model Parameters | NEW | No parameter schema associated with model version/evaluation. | None. | Typed version/run parameters and exact snapshots. |
| Model Evaluations | NEW | No evaluation/metrics-by-dataset records. | Market metrics/chart displays do not provide model evaluation provenance. | Add evaluation keyed to exact model and dataset versions plus AnalysisRun. |
| Validation | NEW | No validation record. Hypothesis verdict is not validation. | No dedicated workflow. | Add criteria, scope, independence, outcome and evidence. |
| Replication | NEW | No replication entity. | None. | Add direct/conceptual replication linked to original run/design. |
| Research Lineage | EXISTING BUT REQUIRING EXTENSION | Source revision chain and hypothesis parent/refined_from exist; canvas snapshots and Forge symbol lineage exist in separate domains. | Partial history and trace views. | No unified scientific lineage across projects and exact versions. |
| Knowledge Transfer | NEW | No transfer/adoption record; Library reusability is not cross-project provenance. | No cross-project research adoption workflow. | Add exact-version transfer/adoption and downstream history. |
| Typed scientific relationships | EXISTING BUT REQUIRING EXTENSION | Source `links` provides limited kinds but target is polymorphic text; hypothesis and canvas edges are separate. | Canvas edges visualize links but are not canonical. | Add typed endpoint relation tables/registry with FK strategy and query traversal. |

## 2. Cross-cutting status

### Pathway 1 — Existing Knowledge Acquisition & Mastery

Source intake, segmentation, extraction and visual organization are present. Mastery is not. There are no learning objectives, assessment attempts, competency progress, framework reconstruction contract, or independent reproduction records. Status: **partially supported at acquisition/organization; target pathway not implemented end-to-end**.

### Pathway 2 — Existing Knowledge Investigation & Development

Question triage and hypothesis/Lab workflows are real; source-to-hypothesis and question-to-hypothesis paths exist. Structured project ownership, experiments, dataset versions, analysis runs, findings, and model evolution are missing. Status: **partial research planning and hypothesis capture, not a complete development pathway**.

### Pathway 3 — First-Principles Scientific Discovery

No dedicated project/domain/phenomenon/observation/discovery model exists. Generic sources/canvas and market data are not an integrated scientific discovery lifecycle. Status: **not implemented**.

## 3. Dependency and implementation order

1. Establish canonical IDs, source versioning, typed relation/FK strategy, and migration guarantees.
2. Add ResearchProject/Domain/Gap and safe question/hypothesis linkage.
3. Add Observation/Dataset/Experiment design/run and immutable data manifests.
4. Add AnalysisRun/Evidence/Finding/statistical-integrity contract.
5. Add Model/ModelVersion/Evaluation/Validation/Replication.
6. Add Framework/learning and mastery; framework/source provenance dependency may allow parallel work after shared IDs/lineage.
7. Add transfer/adoption after versions, review states, and provenance exist.

See [phase-0-executive-report.md](08-phase-0-executive-report.md) for the recommended bounded first implementation scope.
