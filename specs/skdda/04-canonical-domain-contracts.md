# SKDDA Phase 0 — Canonical Domain Contracts

**Contract status:** Proposed logical contract, not production SQL or Rust/TypeScript code.  
**Authority:** [VORA_MIND_SCIENTIFIC_ARCHITECTURE.md](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md).  
**Existing implementation evidence:** [01-repository-audit.md](01-repository-audit.md) and [03-domain-gap-analysis.md](03-domain-gap-analysis.md).

## 1. Cross-entity rules

1. Each canonical entity has an immutable opaque ID, created-at, creator/source of creation, and typed lifecycle status where it has a lifecycle.
2. Mutable drafts record updated-at and revisions; immutable facts/results are corrected by superseding record or explicit correction event.
3. Every record states project ownership or an explicit reusable/workspace scope. Unknown legacy ownership remains unassigned; it is never guessed from text.
4. Relationship endpoints pin exact versions when the content version changes interpretation.
5. Canonical source claims, concepts, observations, patterns, ideas, hypotheses, designs, runs, evidence, findings, models, theories, and frameworks are distinct semantic types.
6. Large data payloads may be immutable assets; their manifests/checksums and IDs remain in SQLite.
7. Rust domain operations own transitions and persistence validation. Serde IPC DTOs mirror stable public contracts; frontend types/store are transport/cache, not authority.
8. This document is intentionally logical. Physical table normalization, CHECK constraints, date representation, JSON schema format, and SQL type choices require the Phase 0 ERD/migration decision.

## 2. Shared identity and provenance primitives

### 2.1 Common fields

Common where applicable: `id`; `created_at`; `created_by`; `updated_at` for editable drafts; `lifecycle_status`; `project_id` for project-owned records; `origin_project_id` for reusable outputs; `provenance_ref`/relation(s); `revision` or exact version reference; `metadata` only for non-critical extensible attributes.

IDs MUST not be sequential titles or path-derived mutable labels. Timestamps MUST have one specified UTC unit and consistent frontend/Rust representation. Numerical scientific values MUST carry unit and sufficient precision. Missing values are distinct from zero and not-applicable.

### 2.2 Evidence-class separation

| Type | Scientific meaning | Must not be conflated with |
|---|---|---|
| Source claim | What an author/source asserts, attributable to citation/version. | Application-verified fact or finding. |
| Knowledge concept | Stable semantic idea/term with definitions and provenance. | A single wording in an extract. |
| Observation | Measurement/record of a phenomenon in stated conditions. | Interpretation, pattern, or causal explanation. |
| Pattern | Exploratory regularity derived from specified observations/data and analysis. | Confirmatory finding or hypothesis. |
| Idea | Proposal for a question/mechanism/model; unvalidated. | Observation or evidence. |
| Hypothesis | Testable proposition with scope and falsification criteria. | Result or validated model. |
| Experiment | Planned design or actual run, kept as separate entities. | Trace graph node or generic protocol text. |
| Evidence | A provenance-bearing artifact and its relationship/polarity to a proposition. | Free-text notes alone. |
| Finding | Bounded conclusion synthesized from evidence/analysis. | Hypothesis verdict alone or consensus. |
| Model | Explanatory/predictive/procedural object independent of a hypothesis. | Board/library item. |
| Theory | Proposed semantic type: explanatory system of claims/mechanisms with scope and predictions. It may be represented as a framework composition initially, but requires explicit approval before a distinct persisted type. | Generic framework or proven truth. |
| Knowledge framework | Versioned organization of concepts, claims, principles, methods and procedures. | Automatically accepted theory or model. |

## 3. Entity contract catalogue

In this table, “FKs” means required logical references; physical enforcement is designed in [05-scientific-relationship-architecture.md](05-scientific-relationship-architecture.md). Status refers to current repository mapping, not target readiness.

### 3.1 Acquisition and learning

| Entity / status | Responsibility and required / optional attributes | Ownership and relations | Lifecycle, versioning, immutability, provenance |
|---|---|---|---|
| KnowledgeSource — Existing but requiring extension (`sources`) | Required identity, kind, title; optional author/creator, language, publication info, canonical URI, asset ref/checksum, access/license metadata. | Reusable source identity; segment/version and project-use links. | Registered/available/unavailable/archived. Identity metadata may revise with revision event; preserve source origin. |
| SourceVersion — New | Required source FK, capture/edition identity, capture time, checksum; optional URI/asset, retrieval method, version label. | Belongs to source; cited by segments, claims, extracts, evidence. | Immutable snapshot; newer edition is new record; provenance includes acquisition method/time. |
| SourceSegment — Existing but requiring extension (`segments`) | Required source and locator; optional source-version FK, parent, ordering, label/note, page/time/line/selector. | Owned by source; referenced by extracts/citations. | Active/archived; corrections recorded; locator pinned to version where possible. |
| Extract — Existing but requiring extension (`extracts`) | Required segment, transform kind, content/payload; optional parent extract, creator/tool/model version, language/confidence/review outcome. | Owned by source context; `derived_from` parent/source segment. | Draft/reviewed/accepted/rejected/superseded/retracted; each transform carries method and origin. |
| KnowledgeClaim — New | Required proposition and claim-kind; optional attribution, scope, assumptions, uncertainty, citations, statement language, review status. | Reusable claim or project-scoped assessment; cites SourceVersion/Segment and may be included in framework version. | Proposed/assessed/supported/contested/unsupported/unknown; revision event, never overwrite attribution. |
| KnowledgeConcept — Existing but requiring extension (cards/extracts only approximate) | Required stable label and definition; optional aliases, domain/scope, language, concept version. | Reusable; linked to source claims, framework versions, learning objectives and findings. | Proposed/reviewed/accepted/deprecated; meaning changes produce revision/version. |
| KnowledgePrinciple — New | Required rule statement; optional conditions, exceptions, strength, scope, source/finding basis. | Reusable; linked to claims/concepts/framework. | Proposed/reviewed/supported/contested/deprecated; provenance records evidence synthesis. |
| KnowledgeFramework — Existing but requiring extension (`library_item_groups` approximate) | Required identity/name/scope; optional summary, origin project, framework category/current version. | Reusable; contains exact framework versions; projects adopt version. | Draft/reviewed/published/superseded/retired. Identity stable; content changes create new version. |
| KnowledgeFrameworkVersion — Existing but requiring extension (library versions) | Required framework FK, version, composition manifest, parent, change class/reason; optional publication metadata. | Contains concepts/claims/principles/procedures/objectives by typed membership; based_on exact sources/findings. | Immutable composition; draft/review/published/withdrawn represented as lifecycle events. |
| LearningObjective — New | Required observable outcome, framework version, criteria; optional prerequisites, difficulty, sequence. | Learner/project context; linked to concepts, competencies, assessments. | Draft/active/retired; outcome per learner is separate progress record. |
| LearningAssessment — New | Required definition/version, objective links, tasks, rubric, scoring and mastery threshold; optional evaluator guidance. | Reusable assessment definition; attempts reference exact version. | Draft/active/locked/retired; version after changes. |
| AssessmentAttempt — New | Required learner, assessment version, submitted answers/artifact, timestamps; optional evaluator, score, feedback, integrity metadata. | Owned by learner/project; linked to objectives/competencies. | Submitted/evaluated; evidence immutable, evaluation correction appended. |
| Competency — New | Required competency identity/name and observable criteria; optional level rubric, scope/domain. | Reusable descriptor; attainment stored per learner. | Proposed/active/retired; descriptor revisioned. |
| LearningProgress — New | Required learner/objective or competency, event time, result/status; optional evidence, assessor, rationale. | Learner-owned and project-contextual. | Append-only events; current progress derived. Mastery is explainable aggregation. |
| ReproductionRecord — New | Required target procedure/framework version, conditions, attempt evidence, evaluator/rubric; optional deviations. | Learner/project owned; supports competency attainment. | Planned/attempted/evaluated; repetitions are distinct immutable attempts. |

### 3.2 Research

| Entity / status | Responsibility and required / optional attributes | Ownership and relations | Lifecycle, versioning, immutability, provenance |
|---|---|---|---|
| ResearchProject — New | Required title, aim, pathway(s), owner, status; optional scope, dates, governance, parent/origin project. | Root owner for research-specific records; may adopt reusable external versions. | Proposed/active/paused/completed/archived; archival does not delete records. |
| ResearchDomain — New | Required name/definition; optional parent domain, vocabulary/version. | Reusable; project-domain many-to-many. | Proposed/active/deprecated; revise with history. |
| ResearchGap — New | Required gap statement, literature review basis, affected knowledge scope; optional confidence, rationale, related framework claims. | Project-owned; based_on sources/framework versions; generates questions. | Candidate/assessed/accepted/rejected/addressed/superseded. |
| ResearchQuestion — Existing but requiring extension (`questions`) | Required question/title, description, status; optional gap/domain, priority, rationale, project, pathway. | Project-owned; may generate hypotheses/findings. Existing promotion ID is soft today. | Parked/triaged/active/answered/closed/archived; do not delete after downstream use. |
| Phenomenon — New | Required description, scope/context/population/time boundary; optional domain, literature/citations. | Project-owned and domain-linked; observed via plans. | Identified/characterized/retired; scope change creates revision. |
| ObservationPlan — New | Required target phenomenon, units, variables/instruments, sample frame and procedure; optional inclusion rules/quality controls. | Project-owned; may be locked/preregistered and link to datasets/runs. | Draft/reviewed/preregistered/locked/executed/withdrawn; lock creates exact version. |
| Observation — New | Required plan/run, timestamp/context, measured values or payload and units; optional instrument, sample key, missingness/quality flags. | Project-owned; may belong to dataset version. | Captured/reviewed/accepted/excluded; immutable after capture, correction supersedes. |
| Pattern — New | Required inferred pattern statement, input observation/dataset versions, method/analysis reference; optional uncertainty and filters. | Project-owned; linked to ideas/hypotheses. | Candidate/examined/supported/unsupported/retired; exploratory status retained. |
| Idea — New | Required content/origin; optional mechanism/question links and supporting observations/patterns. | Project-owned; can seed hypothesis/model. | Captured/explored/promoted/abandoned; never represented as finding by promotion alone. |
| Hypothesis — Existing but requiring extension (`hypotheses`) | Required H1/H0 and operational variables, scope, measure, falsification criteria; optional competing hypotheses, expected effect, assumptions, parent/version. | Project-owned; tests via design/run, supports/contradicts findings, based_on framework/source versions. | Draft/reviewed/preregistered/testing/supported/partially supported/rejected/inconclusive; each verdict is versioned assessment. |

### 3.3 Experiment, evidence and model

| Entity / status | Responsibility and required / optional attributes | Ownership and relations | Lifecycle, versioning, immutability, provenance |
|---|---|---|---|
| ExperimentDesign — New | Required question/hypothesis or model target, methods, variables/outcomes, sampling, analysis/integrity plan; optional randomization/blinding/costs. | Project-owned; immutable design versions; tested proposition relation. | Draft/reviewed/preregistered/locked/retired; modifications after lock create new version. |
| ExperimentRun — New | Required design version, run ID/times, inputs, operator/software/environment, outcome; optional deviations, logs, seeds. | Project-owned; references exact dataset versions and produces observations/analysis. | Scheduled/running/completed/failed/cancelled/inconclusive; close preserves all output. |
| Dataset — New | Required identity/title/schema/owner; optional license/privacy/retention. | Reusable or project-scoped; has immutable versions. | Registered/active/restricted/retired. |
| DatasetVersion — New | Required dataset FK, checksum, schema version, source/transform provenance, sample count; optional split roles/storage URI. | Referenced by run/analysis/evaluation. | Immutable; update creates new version. |
| DataSample — New | Required dataset version and stable sample ID; optional entity/time/features/outcomes/context/quality. `TradeSample` is typed specialization. | Belongs to exact DatasetVersion. | Captured/accepted/excluded; correction supersedes. |
| AnalysisRun — New | Required method/code version, parameters, inputs/splits, environment, seed, outputs; optional logs/errors. | Project-owned; produces Evidence/Findings and ModelEvaluation. | Queued/running/completed/failed; each rerun new record, immutable after finalization. |
| Evidence — New (source links/notes are incomplete analogues) | Required target proposition, artifact/version, polarity, scope and provenance; optional quality rationale/limitations. | Can support/contradict claim/hypothesis/finding/model version. | Recorded/assessed/superseded/retracted; correction is event/new record. |
| Finding — New | Required conclusion, analysis/evidence refs, scope, uncertainty, limitations; optional review/consensus. | Project-owned; may revise framework/model. | Draft/reviewed/accepted/contested/unsupported/inconclusive/superseded. |
| Model — New | Required stable identity/name/purpose/scope; optional owner/origin project/domain/current version. | Reusable across projects; independent of hypothesis. | Proposed/active/deprecated/replaced/retired. |
| ModelSpecification — New | Required equations/algorithm/interface/assumptions and format version; optional human explanation/dependencies. | Version-bound to ModelVersion. | Immutable per version. |
| ModelVersion — New | Required model FK, exact spec/config/artifact, parent, change class/rationale, author/time; optional checksum. | Links to contributing/contradicting research records and evaluations. | Draft/reviewed/released/superseded/withdrawn; immutable. |
| ModelParameter — New | Required name/type/value/unit and exact model version or run; optional bounds/default/role. | Belongs to version or immutable run config. | Value changes create new version/config snapshot. |
| ModelEvaluation — New | Required model version, dataset version/split, analysis run, metric definition/value/unit/uncertainty. | Project-owned evaluation linked to exact inputs. | Immutable; reassessment is new record. |
| Validation — New | Required target version/finding, criteria, method, population/data, evaluator and outcome; optional independence/conflict/deviation. | Project-owned; `validates` exact version. | Planned/running/passed/failed/inconclusive; each attempt retained. |
| Replication — New | Required original experiment/design, replication type, operator/team, method and results; optional deviations/independence. | Project-owned; links to original and outcome. | Planned/executed/assessed; does not overwrite original. |
| ResearchLineage — New | Required origin record/version, recipient project, adopted version, attribution, adaptation context. | Cross-project relation record. | Proposed/accepted/adapted/withdrawn; history retained. |
| KnowledgeTransfer — New | Required reusable item/version, source and recipient projects, eligibility/review, adoption state. | Connects origin and adopter. | Proposed/accepted/rejected/adapted/withdrawn; future revocation does not erase history. |
| ScientificRelation — Existing but requiring extension (source links, hypothesis fields, canvas edges) | Required typed subject/predicate/object and endpoint versions, context/provenance/time. | Cross-domain, validated relation; details in relationship contract. | Append-only relation creation/retraction; no unvalidated polymorphic dangling IDs. |
| Theory — Proposed decision required | If distinct: explanatory system identity, scope, mechanisms/claims/predictions, assumptions, version. Otherwise express as framework/model composition with explicit theory role. | Links to claim/model/framework versions and evidence. | Versioned and contestable, never “truth” by type. Requires approval before a dedicated table. |

## 4. Rust/TypeScript contract strategy

- Rust domain structs/enums are authoritative for persisted states and transitions; Serde serialization provides IPC representation.
- IPC command DTOs should be explicit and versioned where compatibility requires it; do not expose DB rows wholesale.
- TypeScript mirrors nullable/optional semantics exactly; distinguish omitted field (not supplied), `null` (known absent), and empty value.
- Use explicit Rust enum variants serialized in a documented casing and TypeScript discriminated unions with equivalent allowed values.
- Choose one UTC timestamp representation at Phase 0; existing integer timestamps are mixed in unit/meaning, so do not assume ms vs s. Use checked conversions at boundaries.
- Numerical contracts specify integer/decimal precision and units; avoid lossy f64 for exact counts/currency.
- Rust validates endpoint existence, state transitions, immutability, transaction boundaries and scientific requirements. Frontend validation is UX only.
- Error DTOs must identify validation/state/persistence failure without success-shaped fallback. Store updates follow successful IPC result or explicit optimistic rollback.
- Add contract tests for serialization, enum casing, nullable fields, timestamp/numeric conversion and IPC command payloads.

## 5. Decisions explicitly not made

This package does not choose final SQL table decomposition, a generic entity registry implementation, a statistics library, asset storage technology, theory-specific table, authentication/learner identity model, or a remote collaboration protocol. Resolve these in Phase 0 design review before schema or migration implementation.
