# Vora Mind Scientific Architecture

**Status:** Authoritative target architecture specification  
**Scope:** Scientific knowledge acquisition, learning, research, experimentation, model development, validation, and cross-project knowledge transfer  
**Audience:** Product, frontend, backend, database, scientific-method, and test contributors  
**Implementation rule:** This document defines the target. It does not assert that proposed functionality exists. Each feature is labeled **Existing**, **Existing but requiring extension**, or **New**.

## 1. Purpose and normative language

Vora Mind is to evolve into a **Scientific Knowledge Acquisition, Discovery & Development Architecture (SKDDA)**. It must support mastering existing knowledge, investigating and developing existing knowledge, and discovering knowledge from first principles, using one durable and traceable scientific domain model.

“MUST”, “MUST NOT”, “SHOULD”, and “MAY” are requirements for future implementation. The current-state descriptions below are audit findings, not requirements and not claims of completeness. A status label describes the state evidenced in the repository at the time of the architecture audit; verify code and persistence end-to-end before treating a capability as complete.

### 1.1 Status labels

| Label | Meaning |
|---|---|
| **Existing** | The audited repository has an implementation of the described capability. The exact implemented scope is stated; this label does not mean it fulfills all target requirements. |
| **Existing but requiring extension** | A working, related component exists, but it does not yet satisfy the target contract. |
| **New** | No meaningful implementation was established by the audit; the capability is part of the target design. |

Implementation planning may use the audit categories **fully implemented**, **partially implemented**, **UI only**, **backend only**, **structurally present but incomplete**, **not implemented**, or **blocked** in addition to the three labels above. A name in a type, comment, mock, or specification is not proof of working persistence or end-to-end behavior.

## 2. Architectural principles

1. **One shared scientific infrastructure.** All three pathways use the same source, concept, claim, observation, hypothesis, experiment, evidence, finding, model, and lineage contracts where their meanings are the same.
2. **Reuse working infrastructure.** Extend the existing source/provenance, SQLite persistence, Tauri IPC, board/canvas, library, and versioning capabilities where they meet requirements. Do not copy their data into parallel scientific databases.
3. **Durable local-first ground truth.** Persist canonical scientific records in the selected workspace’s durable database. UI state and derived indexes are caches, never the only copy of research results.
4. **Preserve provenance.** Every derived claim, observation, analysis, finding, and model artifact must retain its origin and transformation context.
5. **Preserve history.** A recorded observation, completed run, analysis result, finding, or model version is immutable in meaning. Corrections create a new revision or superseding record and leave the prior record inspectable.
6. **Separate epistemic classes.** Source claims, extracted concepts, raw observations, exploratory patterns, hypotheses, experimental results, findings, and validated knowledge are distinct records or explicitly typed states. Promotion between them is an auditable action, not an implicit change in wording.
7. **Separate learning mastery from research validation.** Completing a course or passing an assessment does not validate a scientific claim; a supported research finding does not establish a learner’s competence.
8. **Models are independent domain objects.** A model exists independently of any one hypothesis, experiment, or project. Experiments may test models; findings may change model versions.
9. **Shared infrastructure, independent pathways.** Pathways may start at different lifecycle points, but must not fork entity definitions or lineage storage.
10. **Explicit semantics and typed boundaries.** Use typed domain values, validated lifecycle transitions, explicit relationships, and mirrored Rust/TypeScript DTO contracts. Do not represent an important scientific relation as an unvalidated title or arbitrary text field.
11. **No silent degradation.** Persistence, migration, validation, or provenance failures must be surfaced. Do not silently drop links or return success-shaped fallback records.
12. **No scientific overclaim.** The application records what a protocol and evidence justify. It does not automatically equate a statistical result with truth or scientific consensus.

## 3. Audited current state

The following is the evidence baseline; it is deliberately separate from the target design.

| Area | Audited implementation | Target gap |
|---|---|---|
| Sources and extraction | **Existing.** `sources`, `segments`, `extracts`, `links`, and append-only `source_revisions` are defined in `src-tauri/src/schema.sql`; source models and operations are in `src-tauri/src/source/`. Extracts can have parent and superseding references. | Extend with unified research ownership, typed claims/concepts, source snapshots/identifiers, and cross-domain links. |
| Board and snapshots | **Existing.** Boards, canvas objects, snapshots, deltas, and resolver functionality support spatial notes and temporal board state in `src-tauri/src/mind/canvas_db.rs` and `src-tauri/src/mind/resolver.rs`. | Keep as visualization and authoring surfaces; do not make canvas edges the sole scientific relation store. |
| Library and versions | **Existing.** Library groups and group versions store reusable board content in `src-tauri/src/mind/library_groups.rs`; board versions also retain snapshots. | Extend as a presentation/reuse mechanism; they are not a scientific framework/model registry or scientific version lifecycle. |
| Inquiry queue | **Existing.** Questions support triage and selection for promotion in `src-tauri/src/mind/questions.rs`. | Connect research questions to projects, domains, gaps, literature, and subsequent hypotheses. |
| Hypothesis workflow | **Existing but requiring extension.** `hypotheses` includes H1/H0, independent/dependent variable strings, metric, threshold, status, verdict, protocol, raw-data text, analysis text, data entries, insights, source-card fields, parent/refinement references, linked definitions, and linked snapshots. See `src-tauri/src/mind/hypothesis.rs` and `src/types/index.ts`. | Add first-class projects, experiment designs/runs, typed observations/datasets, evidence/finding records, versioned analyses, rigorous integrity metadata, and general lineage. |
| Hypothesis UI | **Existing.** Planning/Lab, inquiry, hypothesis detail, and canvas hypothesis components exist, including `src/shared/layout/QuestionsBoardPage.tsx`, `src/shared/layout/HypothesisDetail.tsx`, and `src/mind/components/canvas/VoraHypothesisNode.tsx`. | Extend into pathway workflows and connect to shared scientific services. |
| Research project/domain/gap | **New.** The audit did not establish these as first-class persisted research entities. Vora Trace project registries are not scientific research projects. | Add canonical project, domain, and gap records. |
| Experiments, datasets, analyses, findings | **New.** The audit did not establish a structured experiment-run, dataset-version, statistical analysis, or finding subsystem. Hypothesis data entries are not a substitute. | Add immutable, linked scientific records and analysis execution provenance. |
| Scientific models and versions | **New.** Board/library versions are not scientific model versions. No model specification/parameter/metric lineage lifecycle was established. | Add model registry and immutable versions associated with evidence and evaluation datasets. |
| Scientific relation graph | **Existing but requiring extension.** Source links include some `supports`/`contradicts` semantics and hypotheses have limited parent/refinement fields. | Add typed, constrained, cross-entity relations that make complete lineage queryable. |
| Frontend/backend persistence | **Existing.** The app uses a Tauri/Rust/SQLite persistence path with frontend services and Zustand; examples include `src/mind/services/dbService.ts`, `src/shared/store/useAppStore.ts`, and `src-tauri/src/lib.rs`. | Keep one authoritative persistence route per domain operation, with domain service/repository boundaries and tested DTO parity. |
| Statistical safeguards | **New.** The audit did not establish safeguards for leakage, bias, multiple testing, or out-of-sample evaluation. | Make methodology and dataset roles explicit; validate required disclosures and testable split constraints. |

Repository audit references: `src-tauri/src/schema.sql`, `src-tauri/src/source/{model.rs,db.rs,revision.rs}`, `src-tauri/src/mind/{questions.rs,hypothesis.rs,canvas_db.rs,library_groups.rs,resolver.rs}`, `src/types/index.ts`, `src/mind/services/dbService.ts`, `src/shared/store/useAppStore.ts`, `specs/lab-hypothesis-promotion.md`, and `specs/vora-mind-research-pipeline.md`.

## 4. Three scientific pathways

Pathways are workflow templates over shared domain records. A research project can use more than one pathway over time; changing pathway does not duplicate its sources, evidence, concepts, hypotheses, or models.

### 4.1 Pathway A — Existing Knowledge Acquisition & Mastery

**Purpose:** Acquire an existing framework, reconstruct it faithfully, understand it, organize its concepts, develop skills, and demonstrate independent reproduction.

Required progression:

1. **Acquire sources:** register source identity and version/provenance; attach books, courses, papers, media, documents, or other materials.
2. **Extract:** create source-grounded extracts with location, transformation, creator/tool, confidence, review state, and parent transformation.
3. **Classify epistemic content:** distinguish source assertion/claim, definition, concept, principle, procedure, example, and learner interpretation.
4. **Reconstruct framework:** organize concepts, principles, claims, prerequisites, procedures, and contradictions as a versioned framework linked to citations.
5. **Understand:** connect learning objectives to framework elements; record questions and explanations without overwriting source claims.
6. **Practice:** define observable tasks and procedures, record attempts and feedback.
7. **Assess:** use an assessment definition/rubric and immutable attempt results; associate them with learning objectives and competencies.
8. **Demonstrate mastery:** require a configured mastery rule and evidence of independent reproduction. Course completion alone is not mastery.

**Outcome:** a sourced, versioned reconstruction and a separate, evidence-backed learner competency record.

**Status:** source intake/extraction and board/library organization are **Existing but requiring extension**; learning objectives, formal framework reconstruction, assessment, competency, progress, and independent reproduction are **New**.

### 4.2 Pathway B — Existing Knowledge Investigation & Development

**Purpose:** Assess claims in an existing framework, identify limitations and gaps, formulate research questions, test claims, and develop, extend, refine, revise, or replace models/framework versions.

Required progression:

1. Select the framework/model version and exact claims under investigation.
2. Record prior literature, evidence, assumptions, known limitations, and the research gap.
3. Define a scoped research question and phenomenon; specify observations and measurement context.
4. Create hypotheses with operational variables and falsification criteria.
5. Preregister a design where appropriate, including inclusion/exclusion, outcomes, analysis plan, and dataset roles.
6. Execute one or more immutable experiment runs and record raw data/dataset versions.
7. Run versioned analyses; report uncertainty, limitations, and integrity checks.
8. Create findings that explicitly support, contradict, qualify, or fail to resolve claims/hypotheses.
9. Decide whether and how to change the model/framework, preserving prior versions and supporting evidence.
10. Validate independently and record replication outcomes where possible.

**Outcome:** evidence-supported development, extension, refinement, revision, or replacement of prior knowledge, with traceable provenance.

**Status:** inquiry triage, hypothesis protocols, verdicts, and hypothesis-to-library graduation are **Existing but requiring extension**; research projects/domains/gaps, experiment/dataset/analysis/finding entities, and formal model evolution are **New**.

### 4.3 Pathway C — First-Principles Scientific Discovery

**Purpose:** Construct new knowledge when no adequate adopted framework exists for the research problem. “First principles” does not waive literature review or prior-knowledge assessment.

Required progression:

1. Define a research project and domain, boundaries, terminology, and intended use.
2. Review literature and prior knowledge; record sources, search scope/date, and what is unknown or unsuitable.
3. Identify a phenomenon and observation plan before treating an observed pattern as an explanation.
4. Capture raw observations with instruments, context, sampling, time, provenance, and quality markers.
5. Explore patterns and create ideas/concepts with links to the observations and analysis that produced them.
6. Formulate competing and null hypotheses, including operational definitions and falsification criteria.
7. Design and run experiments; separate exploratory from confirmatory analysis and data.
8. Accumulate evidence and findings, including negative, null, and inconclusive results.
9. Construct a model/framework as explicit, versioned specifications; distinguish model assumptions from findings.
10. Validate and seek independent replication; publish a reusable framework version with full provenance.

**Outcome:** a new, evidence-supported framework whose origin, uncertainty, limitations, and validation status remain inspectable.

**Status:** generic source and canvas capture are **Existing but requiring extension**; the systematic discovery lifecycle is **New**.

## 5. Unified scientific domain model

### 5.1 Identity, ownership, and shared metadata

Every canonical record MUST have an immutable ID, creation timestamp, creator/provenance, owning workspace, and explicit lifecycle state where applicable. Mutable records MUST also have an update timestamp and revision policy. Immutable records MUST never be silently rewritten.

Scientific records are owned by a `ResearchProject` unless they are reusable global/workspace knowledge objects such as a `KnowledgeSource`, a published `KnowledgeFramework`, or a `Model`. Project use is represented by explicit relations, not by copying the reusable record. A framework or model version retains its original project and may be adopted by additional projects.

Avoid one generic `objects(type, JSON)` table as the only scientific data contract. JSON is appropriate for versioned, validated configuration payloads and extensible metadata; lifecycle-critical, query-critical fields and foreign keys remain typed columns or constrained relations.

### 5.2 Entity catalog

All attributes below are logical contracts, not a prewritten physical schema. IDs use opaque stable identifiers; references are foreign keys unless explicitly marked external. Timestamps are UTC instants. Large raw payloads MAY be stored as immutable content-addressed workspace files with a durable database manifest, checksum, media type, and size.

#### Knowledge acquisition and learning

| Entity | Status | Responsibility and required attributes | Lifecycle |
|---|---|---|---|
| `KnowledgeSource` | Existing but requiring extension (`sources`) | Canonical source identity: `id`, kind, title, creators/authors, publication/version/date, URI or local asset reference, checksum, language, metadata, access/availability, created/updated. Distinguish source identity from a particular file/version. | registered → available/unavailable → archived; source revisions preserve changes. |
| `SourceVersion` | New | Immutable source edition/retrieval: source FK, edition/version, canonical URI, captured timestamp, checksum, asset reference, retrieval metadata. Citations bind to this version when known. | captured → superseded (never deleted). |
| `SourceSegment` | Existing but requiring extension (`segments`) | Addressable region: source/version FK, parent segment, order, page/time/line/selector range, label, notes. | active → archived; corrections are revisioned. |
| `Extract` | Existing but requiring extension (`extracts`) | Source-derived transformation: segment FK, kind, content or asset, parent extract, method/tool/model/version, author, confidence, review state, timestamps. | draft → reviewed/accepted/rejected → superseded/retracted. |
| `KnowledgeClaim` | New | Attributable proposition: statement, claim kind, author/source, exact citation(s), scope, assumptions, uncertainty, epistemic status, creation/revision. Claims do not equal findings. | proposed → assessed → supported/contested/unsupported/unknown; may be superseded, never erased. |
| `KnowledgeConcept` | Existing but requiring extension (cards/extracts approximate only) | Stable concept identity, preferred label, definition, aliases, scope/domain, provenance, version. Separate the concept from each source’s wording of it. | proposed → reviewed → accepted/deprecated; versioned changes. |
| `KnowledgePrinciple` | New | Reusable rule/generalization with statement, conditions, exceptions, confidence, source claims/findings, and framework version membership. | proposed → reviewed → supported/contested/deprecated. |
| `KnowledgeFramework` | Existing but requiring extension (`library_item_groups` approximate reusable containers) | Named, scoped, reusable organization of concepts, principles, claims, procedures, and dependencies; owner/origin project, description, provenance, current version pointer. | draft → reviewed → published → superseded/retired. |
| `KnowledgeFrameworkVersion` | Existing but requiring extension (`library_item_group_versions` are not scientific framework versions) | Immutable exact framework composition and metadata; parent version, change class/reason, source claims, findings, contributing projects, created by/time. | immutable version; may be draft, published, or withdrawn by a new event. |
| `LearningObjective` | New | Observable learner outcome, framework/version context, linked concepts/procedures/competencies, assessment criteria, level, sequence/prerequisites. | draft → active → achieved/not-achieved/retired per learner record. |
| `LearningAssessment` | New | Assessment definition: objectives, tasks/items, rubric, scoring method, mastery threshold, version, authorship. | draft → active/locked → retired; attempts always reference exact version. |
| `AssessmentAttempt` | New | Immutable learner attempt: assessment version, learner identity, start/end, responses/artifacts, score, rubric outcome, evaluator, feedback, integrity/provenance. | submitted → evaluated; corrections append evaluations, not rewrite attempt evidence. |
| `Competency` | New | Named ability with scope, observable criteria, level scale, and evidence requirements. | proposed → active → retired; learner attainment is separately recorded. |
| `LearningProgress` | New | Learner/objective/competency progress event, evidence links, status, result, timestamp, assessor, and pathway context. | append-only progress events; current state is derived. |
| `ReproductionRecord` | New | Evidence that a learner independently reproduced a framework procedure/result; procedure/version, conditions, artifacts, evaluator/rubric, deviations, outcome. | planned → attempted → evaluated; repetitions are distinct records. |

#### Research and discovery

| Entity | Status | Responsibility and required attributes | Lifecycle |
|---|---|---|---|
| `ResearchProject` | New | Scientific unit of work: title, aims, scope, status, pathway(s), owner, dates, domain links, lineage, protocol/governance metadata. | proposed → active → paused/completed → archived. |
| `ResearchDomain` | New | Reusable subject area with description, parent domain, vocabulary, and source references. Projects can link to several domains. | proposed → active → deprecated. |
| `ResearchGap` | New | Evidence-supported statement of missing/limited knowledge, affected framework/version and claims, literature review scope, rationale, uncertainty, origin. | candidate → assessed → accepted/rejected → addressed/superseded. |
| `ResearchQuestion` | Existing but requiring extension (`questions`) | Scoped, answerable question with project, gap, domain, rationale, priority, pathway, status, and resulting hypotheses/findings. Inquiry queue remains intake/triage UX, not the sole research record. | parked → triaged → active → answered/closed/archived; promotion preserves same identity or explicit lineage. |
| `Phenomenon` | New | Precisely scoped phenomenon of interest, context, entity/population, temporal/spatial boundary, source/literature grounding, and linked questions. | identified → characterized → retired; changing scope creates a version/revision. |
| `ObservationPlan` | New | Preregisterable procedure for observing a phenomenon: sampling frame, unit, variables, instruments, timing, inclusion/exclusion, quality controls. | draft → preregistered/locked → executed/withdrawn. |
| `Observation` | New | Individual observation record: plan/run, subject/sample key, observed values or immutable payload reference, units, instrument, time, context, missingness/quality flags, provenance. | captured → quality-reviewed → excluded/accepted; corrections append a superseding observation. |
| `Pattern` | New | Exploratory regularity inferred from specified observations/dataset and analysis; includes selection context, uncertainty, and exploratory status. A pattern is not automatically a finding or causal claim. | candidate → examined → supported/unsupported/retired. |
| `Idea` | New | Unvalidated concept for a question, mechanism, or model; records origin and links to motivating evidence. | captured → explored → promoted to a defined record or abandoned. |
| `KnowledgeConcept` | Existing but requiring extension | Shared concept record as specified above; used for learned and discovered concepts without duplicate pathway-specific tables. | shared lifecycle above. |
| `Hypothesis` | Existing but requiring extension (`hypotheses`) | Falsifiable proposition tied to project/question/framework/model context. Includes H1 and H0, optional competing hypotheses, variables with operational definitions/units, population/scope, predicted effect/direction, falsification criteria, prior, status, and lineage. | draft → reviewed → preregistered → testing → supported/partially supported/rejected/inconclusive; terminal assessments remain revisable only through a new assessment event/version. |

#### Experiments, data, evidence, and findings

| Entity | Status | Responsibility and required attributes | Lifecycle |
|---|---|---|---|
| `ExperimentDesign` | New | Reusable, versioned protocol: target hypothesis/model, design type, intervention/control, factors, outcomes, sampling, sample-size rationale, stopping rule, inclusion/exclusion, randomization/blinding, preregistration timestamp, analysis plan, costs/slippage where relevant. | draft → reviewed → preregistered/locked → retired; revisions create new versions. |
| `ExperimentRun` | New | One execution of one exact design version: run ID, timestamps, operator/software/environment, status, deviations, seeds, resources, logs, outcomes, dataset versions. A run is not the design itself. | scheduled → running → completed/failed/cancelled; immutable after close except appended correction/annotation. |
| `Dataset` | New | Logical dataset identity: title, description, domain, owner, source/provenance, data dictionary/schema, licensing/privacy classification, storage policy. | registered → active → restricted/retired. |
| `DatasetVersion` | New | Immutable snapshot: dataset FK, version, checksum, schema version, row/sample count, creation method/time, source IDs, split assignment/role, transformations, storage URI. | immutable; later data is a new version. |
| `DataSample` | New | Observation-level unit (including `TradeSample` when appropriate): dataset-version FK, stable sample key, entity/time/context, feature/outcome values or payload reference, provenance, quality flags. | captured → accepted/excluded; corrections supersede rather than rewrite. |
| `AnalysisRun` | New | Reproducible analysis invocation: code/notebook or method/version, parameters, environment/dependency lock, random seeds, input dataset-version IDs and roles, start/end, outputs, logs, success/failure, analyst. | queued → running → completed/failed; rerun creates new record. |
| `Evidence` | New | Typed evidential contribution linking a source/observation/dataset version/analysis/run/finding to a claim, hypothesis, model version, or framework version; relationship polarity, method, quality/strength rationale, scope, limitations, citation, creator/time. | recorded → assessed → superseded/retracted with history. |
| `Finding` | New | A bounded conclusion from specified evidence and analysis, including claim, result, uncertainty, limitations, scope, and status. Distinct from hypothesis verdict and from established knowledge. | draft → reviewed → accepted/contested/unsupported/inconclusive → superseded. |

#### Models, validation, replication, and lineage

| Entity | Status | Responsibility and required attributes | Lifecycle |
|---|---|---|---|
| `Model` | New | Persistent identity for a conceptual, mathematical, computational, or procedural model; purpose, scope, owner/origin project, domain, current version, status. Not owned by a hypothesis. | proposed → active → deprecated/replaced/retired. |
| `ModelSpecification` | New | Machine-readable and human-readable definition, assumptions, equations/algorithm, interfaces, applicable scope, units, dependencies, and specification format/version. | immutable per model version. |
| `ModelVersion` | New | Immutable exact implementation/specification/configuration, parent version(s), semantic change class, rationale, author/time, contributing findings/hypotheses/experiments, code/artifact checksum, evaluation references. | draft → reviewed → released → superseded/withdrawn; status events do not rewrite content. |
| `ModelParameter` | New | Parameter definition/value for an exact version or run: name, type, value/unit, bounds/default, source, tuneability, and role. Values used in a run are immutable snapshots. | version-bound; changes require a new version or run configuration. |
| `ModelEvaluation` | New | Metrics for one exact model version and dataset version/split, with metric definitions, uncertainty, analysis-run FK, evaluation time, threshold, and limitations. | immutable evaluation; reassessment is a new record. |
| `Validation` | New | Planned or completed validation against predeclared criteria: target model/version or finding, method, independence, dataset/population, pass/fail/inconclusive outcome, evidence, deviations, evaluator. | planned → executing → passed/failed/inconclusive; each attempt retained. |
| `Replication` | New | Independent or direct repeat of a specified experiment/design/model version; identifies replication type, operator/team, protocol deviations, dataset, results, and relation to original. | planned → executed → assessed; never replaces original. |
| `ResearchLineage` | New | Explicit cross-project derivation/adoption context: originating record/version, receiving project, adoption date, exact version, transformation/adaptation, attribution, and permission/licensing context. | proposed → accepted → adapted/withdrawn; history retained. |
| `KnowledgeTransfer` | New | Auditable act of making a validated/reviewed finding, concept, principle, or framework version available to another project, with source and receiving project, exact version, rationale, and adoption state. | proposed → accepted/rejected → adapted/withdrawn. |

### 5.3 Lifecycle rules

- Status transitions MUST be explicit domain operations with validation, actor, time, and reason where a state change affects interpretation.
- `supported`, `validated`, `replicated`, `mastered`, and `established` are different predicates and MUST NOT be collapsed into one status.
- Hypothesis verdicts summarize a particular body of testing; they do not erase runs or evidence and do not by themselves establish a model.
- Findings may be disputed or superseded. Preserve every prior finding and assessment.
- Dataset versions, completed run inputs, analysis inputs, model versions, assessment attempts, and submitted observations are immutable scientific snapshots.
- A rejected, failed, null, or inconclusive result remains queryable and participates in later evidence review.
- “Established knowledge” is a governed synthesis state requiring a documented evidence/replication policy, independent review, scope, and version. It is not a user-editable shortcut.

## 6. Explicit scientific relationships

### 6.1 Relationship contract

Store scientific relationships as typed, persisted relations, not just canvas edges, link text, or JSON references. A relation has: immutable ID; subject record type and ID; predicate; object record type and ID; project/context where relevant; direction; provenance/citation; creator/time; optional rationale, scope, confidence, and superseding relation. Enforce valid endpoint types and cardinality in the domain layer and, where practical, with foreign keys or typed junction tables. Avoid unenforceable polymorphic foreign keys: use typed relation tables or an entity registry with real referential constraints.

| Predicate | Meaning and minimum constraint |
|---|---|
| `derived_from` | Subject was produced by a stated transformation of the object. Preserve transformation/tool/version and source snapshot. |
| `tests` | Experiment design/run targets a hypothesis or model version. A run references the exact design version. |
| `supports` | Evidence/findings increase support for a specified claim/hypothesis/model within stated scope. Include supporting rationale and evidence provenance. |
| `contradicts` | Evidence/findings conflict with a specified proposition within stated scope. Must not be modeled as deletion or simple negative sentiment. |
| `validates` | A validation record evaluates a named model/version/finding against explicit criteria. Result and method belong to the validation record. |
| `refines` | New hypothesis/model/framework version narrows or improves a prior one while retaining its core scope; record change rationale. |
| `extends` | New version adds scope/capability without claiming the prior version was false; retain both versions. |
| `revises` | New record changes a prior claim/model/framework because assumptions or evidence changed; record what changed and why. |
| `supersedes` | New version/record replaces the current recommendation while preserving the superseded record and reason. |
| `replicated_by` | Original experiment or finding has a linked independent/direct replication record; replication outcome is explicit. |
| `based_on` | A design, interpretation, model, or framework relies on exact sources, concepts, findings, or versions. Exact version/citation is required when applicable. |
| `originated_from` | Records the originating project, source, observation, question, or prior knowledge context. This is provenance, not necessarily causal derivation. |

Additional required predicates include `cites`, `defines`, `contains`, `part_of`, `measures`, `observed_in`, `analyzed_by`, `produces`, `evaluates`, `adopts`, `implements`, `assesses`, and `retracts`. The vocabulary is versioned; adding a predicate requires documented endpoint semantics and migration compatibility.

### 6.2 Required lineage queries

The backend MUST be able to reconstruct, with stable IDs and exact versions:

1. A claim → source version/citation → segment → extract/transformation.
2. A finding → evidence → observations/dataset version → experiment run → design version → hypothesis/model version.
3. A model version → change class and rationale → contributing findings/experiments/hypotheses → exact prior model version.
4. A framework version → constituent claims/concepts/principles → source/evidence and findings.
5. A transferred item → originating project/version → receiving project/adaptation → downstream research outputs.
6. A learner mastery outcome → objective/competency → assessment version/attempt → framework version and supporting reproduction evidence.

Canvas edges MAY visualize these relations and offer navigation, but the canonical relation must survive canvas deletion, board fork, and layout changes.

## 7. Knowledge acquisition, reconstruction, and mastery

### 7.1 Source and extraction flow

The source workflow extends current `sources` → `segments` → `extracts` behavior. Every extraction must retain the exact source version and location where available, extraction kind, parent transformation, tool/model and version when machine-produced, confidence, review status, timestamp, and correction/supersession history. A quote, paraphrase, summary, learner interpretation, and source claim are distinct kinds.

Source claims MUST remain attributable to their source and must not be silently promoted to application-verified facts. User review confirms extraction quality, not scientific truth. Citation locations MUST be stable enough to reopen or report the referenced page/time/line when the source asset is available.

### 7.2 Framework reconstruction

A framework version is a curated, immutable composition of concepts, principles, claims, procedures, learning objectives, and typed relations. Each included item has a role, order/structure, and provenance. Conflicting source claims are retained and explicitly related; reconstruction must not erase disagreement to produce a falsely coherent framework.

Framework authoring supports draft work, review, publication, version comparison, and deprecation. Editing a published framework creates a new version with a change class and rationale. Exact adopted versions are pinned by downstream projects and learning assessments.

### 7.3 Mastery and competence

Learning objectives, assessments, competency records, and research findings have separate tables and state machines. Assessment attempts reference a specific assessment version and learner; competency attainment references criteria and evidence. Mastery rules are explicit, versioned, and configurable by assessment/framework designers. Reproduction requires a recorded independent task, conditions, artifact/result, evaluator, and rubric.

The system MUST NOT infer mastery from time spent, content opened, a checklist tick, or a framework’s scientific validation state. A later assessment changes the derived current progress view but does not erase prior attempts.

## 8. Scientific research architecture

### 8.1 Projects, domains, gaps, and questions

Projects own the research plan and connect domain(s), pathway(s), literature review, gaps, phenomena, questions, and outputs. A project may adopt external knowledge versions without taking ownership of their origin. Domains are reusable classifications, not project containers. A gap must cite the literature/prior-knowledge assessment that makes the gap credible and may have multiple questions. Questions may be refined; preserve their parent/derived lineage.

### 8.2 Phenomena, observations, patterns, concepts, hypotheses

An observation is a record of what was measured/seen under stated conditions. A pattern is an interpretation derived from a declared dataset/analysis; it is not raw data. A concept is a reusable semantic abstraction; an idea is an unvalidated proposal. A hypothesis is a falsifiable proposition with operational definitions and test plan, distinct from an exploratory pattern and a finding.

Hypotheses MUST support H0 and alternative/competing hypotheses, independent/dependent variables, covariates where relevant, units, operational measurement, population/scope, expected direction/effect, falsification criteria, assumptions, preregistration, parent/child relationships, and multiple design versions/runs. The current text fields are carried forward only where their semantics fit; they are not treated as sufficient typed definitions.

## 9. Experimental architecture

### 9.1 Design and run separation

An `ExperimentDesign` defines an intended procedure and can have immutable versions. Each `ExperimentRun` references exactly one design version and records what actually happened. Deviations are first-class fields and must be declared before interpretation. Runs may be completed, failed, cancelled, or inconclusive; none are deleted or reused as a later run.

Designs specify, as applicable:

- Research question, target hypothesis/model version, and estimand.
- Study/design type; intervention, control/comparison, factors, outcomes, and covariates.
- Unit of analysis, sampling frame, eligibility/inclusion/exclusion, sample size and rationale, stopping rule.
- Randomization, blinding, instruments, operational definitions, data collection schedule.
- Preregistered analysis plan, primary/secondary outcomes, multiplicity strategy, and deviations policy.
- Data source and dataset versions/roles; code, environment, seed, and reproducibility instructions.
- Transaction costs, fees, slippage, market/session assumptions for relevant market experiments.

### 9.2 Datasets and observations

Dataset identity and dataset version are separate. A version is immutable and checksummed; transformations from source dataset versions are declared and reproducible. Every analysis/run pins exact versions. Training, validation, test, exploratory, and prospective/forward roles are explicit and cannot be inferred from filenames.

Individual observations are queryable when needed; large data may be in workspace-managed files or columnar formats, with a SQLite manifest and referentially linked observation/sample IDs. `TradeSample` is a domain-specific `DataSample` shape, not a separate disconnected persistence system. It may add instrument, order/execution, fees, slippage, timing, and market-context fields with typed units.

### 9.3 Analysis, evidence, and findings

An analysis run is an immutable execution with input versions, method/code version, parameters, environment, seeds, results, logs, and errors. A failed analysis is preserved. Analysis outputs do not overwrite source datasets or previous results.

Evidence links a specific artifact to a proposition and records polarity, scope, methodological quality rationale, limitations, and provenance. Findings summarize the results of specified analyses and evidence. They retain uncertainty and applicability; a finding can be null, contradictory, inconclusive, or later contested.

## 10. Statistical integrity and reproducibility

Integrity controls are part of the domain contract and workflow validation, not merely explanatory documentation.

### 10.1 Required safeguards

For quantitative evaluation, the design and run MUST record applicable safeguards and exceptions:

- **Overfitting:** distinguish training/tuning from evaluation; retain parameter search space and selection process; report complexity and sensitivity where relevant.
- **Data snooping / leakage:** pin data cutoffs and transformation order; prevent evaluation data from influencing training or feature selection; record all tried variants/hypotheses when known.
- **Selection/survivorship bias:** record sampling frame, inclusion/exclusion rules, missing/filtered cases, and survivor population.
- **Look-ahead bias:** enforce feature availability timestamps against prediction/decision timestamps where temporal data is used.
- **Multiple hypothesis testing:** record family of tests and planned correction or explicitly justify no correction.
- **Train/validation/test separation:** pin immutable dataset versions and sample assignments; prohibit overlapping sample IDs where the design requires disjoint splits.
- **Out-of-sample testing:** identify test period/population before evaluation and show whether it was used for model selection.
- **Walk-forward analysis:** record ordered windows, training/validation/test boundaries, refit cadence, and aggregation method.
- **Forward testing:** distinguish prospective/paper/live observations and preserve actual execution conditions and costs.
- **Reproducibility:** preserve protocol/design version, raw inputs/checksums, transforms, analysis code/version, dependencies/environment, parameters, seeds, timestamps, and deviations.

Applicability is explicitly `applicable`, `not_applicable` (with reason), or `not_assessed`; omission is not interpreted as a passed safeguard. The UI warns on missing required declarations and the backend rejects invalid transitions such as marking a preregistered design complete without required locked fields.

### 10.2 Statistical reporting

An analysis result MUST identify the estimand, sample size, method, effect estimate and units, uncertainty interval where meaningful, assumptions, exclusions, missing data, and multiplicity treatment. P-values alone do not constitute evidence of practical importance or truth. Statistical methods are versioned and tested; the application records computation provenance rather than claiming that a numeric metric independently validates a model.

## 11. Model development lifecycle

### 11.1 Model identity and versions

Models are independent of hypotheses. A hypothesis can test a model; a model may be tested by many hypotheses and projects. `Model` is stable identity; each `ModelVersion` captures exact specification, parameters, implementation/artifact, assumptions, dependencies, and scope. No edit may mutate the meaning of an existing version.

Each version MUST:

- Preserve the exact human/machine-readable specification and parameter values.
- Reference parent version(s), author/time, change rationale and change classification.
- Link contributing/contradicting hypotheses, findings, experiments, and evidence by stable IDs and exact versions.
- Reference immutable evaluation records and exact dataset versions/splits.
- Preserve code/artifact and environment identities/checksums where applicable.

### 11.2 Change classes

Every new version declares one of:

- **Refinement:** improves precision or performance inside the previous scope without changing its central assumptions.
- **Extension:** adds a capability, domain, parameterization, or scope while retaining the prior scope.
- **Revision:** changes assumptions, mechanism, or conclusions because prior evidence or reasoning needs correction.
- **Replacement:** a distinct successor is recommended in place of the old model; the old model and versions remain available.

Changes MUST include rationale, affected claims/assumptions, expected implications, and links to evidence. Classification does not by itself assert that the new model is validated.

### 11.3 Metrics and historical results

Metrics attach to `ModelEvaluation` with exact `ModelVersion`, `DatasetVersion`, split/role, `AnalysisRun`, metric definition/version, value/units, uncertainty, and evaluation time. A dashboard’s “current performance” is a derived view; it must not rewrite earlier evaluations when a model changes. Re-evaluating old versions creates new evaluation records.

## 12. Validation and replication

Validation is a test of a specified claim/finding/model version against predeclared criteria, data/population, and method. The system records evaluator independence, conflicts/role, pass/fail/inconclusive result, deviations, and underlying evidence.

Replication identifies the original experiment/design/version, replication type (direct or conceptual), independent team/operator, exact protocol and dataset context, deviations, and results. A replication does not overwrite or automatically reverse the original. Conflicting replications are related with `contradicts` and resolved through reviewed synthesis, not deletion.

“Validated” is scoped to the validation criterion, population, and model version. “Replicated” means a qualifying replication record exists and its result is known; it does not mean the result is successful. Promotion to established knowledge requires a separately defined review policy and evidence synthesis.

## 13. Knowledge lineage and cross-project transfer

Every reusable output retains its origin project and exact version. Another project adopts it using a `KnowledgeTransfer`/`ResearchLineage` record, pinning that version and recording attribution, context, adaptation, and any license/access constraints. The receiving project may use it as prior knowledge without duplicating its source facts or rewriting its origin.

If the receiving project adapts an artifact, the adapted object has its own identity/version and `derived_from`/`based_on` relation to the original. Contributions from both projects remain queryable. Project archives do not delete adopted knowledge or invalidate references.

Required user operations:

- Browse/share eligible findings, concepts, principles, model versions, and framework versions.
- Preview provenance and validation scope before adoption.
- Adopt an exact version into a receiving project with attribution.
- Fork/adapt as a new version while retaining upstream relation and license.
- View downstream projects and outputs; withdraw/restrict future adoption without erasing historical uses.
- Resolve provenance for any result from a receiving project back to source/project/version.

## 14. Database architecture

### 14.1 Ownership and boundaries

SQLite in the selected Vora workspace remains the durable ground truth. Scientific entities are owned by their domain module and persisted through domain operations, not inline SQL in UI components or Tauri command registration. A scientific record is not stored only inside a canvas snapshot, browser-local state, or unvalidated JSON blob.

Proposed logical ownership:

| Domain | Owns |
|---|---|
| Knowledge acquisition | sources/versions/segments/extracts/claims/concepts/principles/frameworks |
| Learning | objectives/assessment definitions and versions/attempts/competencies/progress/reproduction |
| Research | projects/domains/gaps/questions/phenomena/observation plans and observations/patterns/ideas/hypotheses |
| Experiment and evidence | designs and versions/runs/datasets and versions/samples/analyses/evidence/findings |
| Model development | models/specifications/versions/parameters/evaluations/validations/replications |
| Lineage | typed scientific relations, research lineage, transfers, provenance events |
| Canvas | spatial placement, board snapshots, and visual edge state; references scientific IDs but does not own scientific truth |

These are logical ownership boundaries, not a requirement for separate databases. The initial storage may remain one workspace SQLite database with clearly owned tables and migrations.

### 14.2 Key and foreign-key rules

- Use immutable opaque text IDs consistent with existing repository conventions; ID format must not encode mutable titles or status.
- Every single-owner relation uses a foreign key with intentional delete behavior. Scientific records default to restrictive/no-cascade deletion; correction/deprecation uses lifecycle events. Cascading delete is permitted only for disposable, non-scientific UI/cache children.
- Many-to-many relations use junction tables with composite uniqueness and foreign keys, such as project-domain membership, framework-version membership, hypothesis-definition/version links, evidence targets where typed, and transfer/adoption.
- Do not use dangling polymorphic `target_kind/target_id` for required scientific lineage without a referential-integrity strategy. Prefer typed relation tables or a validated entity registry. Existing source `links` remain compatible during migration but are not the final universal relation mechanism.
- Cross-project references pin exact source/model/framework/dataset versions where interpretation depends on content.
- Preserve units, timezone, numeric precision, missingness, and external asset hashes. Validate JSON schema/version at the service boundary when JSON is appropriate.
- Index foreign keys and common queries: project/status/time; source/version; hypothesis/run; dataset/version/split/sample; model/version/evaluation; relation subject/object/predicate; lineage origin/recipient.
- SQLite foreign-key enforcement must be enabled and verified on every relevant connection. Transactions must cover multi-record lifecycle operations (e.g., promotion, preregistration, run finalization, version publication, knowledge transfer).

### 14.3 Versioning, append-only history, and migrations

- Separate stable identity tables from immutable version tables for frameworks, model specifications, experiment designs, assessments, datasets, and analysis definitions.
- Mutable drafts may be edited with optimistic concurrency or explicit revisions. Locking/preregistration seals the exact version used by runs.
- Runs, observations, submitted attempts, evidence, findings, evaluations, and version snapshots are append-only in scientific meaning. Corrections create superseding records or explicit correction events with before/after provenance.
- Never cascade-delete a scientific record that is referenced by a run, finding, model version, or transfer. Retraction and archival are explicit states.
- Migrations are idempotent, ordered, transactional where supported, and tested against: fresh database, each supported prior schema, repeat application, interruption/rollback behavior, and existing realistic data.
- Migration code and initial schema remain in the repository’s established persistence layer; migrations must not be hidden in UI startup logic. Add schema version tracking if absent from the relevant database path.
- Migrations preserve existing source, board, library, question, and hypothesis data. Backfills must state how legacy untyped text/JSON is mapped; uncertain mappings remain explicitly unknown, never guessed.
- Schema changes update Rust models, IPC DTOs, frontend types, service functions, documentation, and tests in the same change.
- Export/backup and restore must preserve IDs, version pins, links, checksums, and migration compatibility. Validate referential integrity after restore.

## 15. Backend architecture and service boundaries

### 15.1 Target request path

Persisted operation flow:

`React view → workflow component/hook → Zustand cache/action → typed frontend service → Tauri command/IPC DTO → domain service → repository/database transaction → SQLite`

The Rust domain operation is the authority for validation, permissions, state transitions, and atomic writes. The IPC handler performs transport parsing and delegates; it does not contain SQL or scientific business rules. Frontend validation improves UX but never replaces backend validation.

### 15.2 Proposed modules and responsibilities

| Module | Responsibility |
|---|---|
| Source/knowledge | Source versions, citations, extraction lineage, claims, concepts, principles, framework versions. Extend current `source` module and library integration without conflating them. |
| Learning | Objectives, assessments, attempts, competencies, mastery/reproduction evidence. New domain service. |
| Research | Projects, domains, gaps, questions, phenomena, observations, patterns, ideas, hypotheses and validated transitions. Extend `mind::questions`/`mind::hypothesis` behind explicit domain boundaries. |
| Experiment | Design/version, run lifecycle, dataset/version, samples, integrity checks, analysis run manifests. New domain service. |
| Evidence and findings | Typed evidence relations, finding lifecycle, evidence synthesis and provenance. New domain service. |
| Model development | Model registry, specification/version, parameters, evaluations, validation and replication. New domain service. |
| Lineage/transfer | Typed relationship validation, provenance traversal, project adoption and downstream impact. New shared domain service. |
| Persistence/migrations | Transactions, repository operations, schema migrations, integrity checks, backup integration. Follow existing workspace DB architecture; do not duplicate database files without a documented scale/ownership need. |
| IPC | Stable commands and versioned DTOs, input shape validation, clear error mapping; registered in existing Tauri setup. |

Long-running analysis must not block the UI or hold SQLite locks while computing. Persist a run state, execute against immutable inputs, and atomically finalize outputs/status. Any worker integration is a later implementation choice and must preserve local-first durability and provenance.

### 15.3 Error and consistency behavior

Return typed, actionable errors for invalid state transition, missing version, invalid relation endpoint, schema mismatch, checksum failure, and database failure. Multi-entity transitions must rollback entirely on failure. The frontend marks persisted state only after confirmed writes or provides explicit optimistic rollback/error display. Never silently omit unavailable provenance.

## 16. Frontend architecture and workspace organization

Use the existing Tauri + React + TypeScript + Zustand pattern. Keep canonical domain interfaces in `src/types/index.ts` or a deliberately approved typed domain submodule that preserves the repository’s single-source-of-truth policy. Rust Serde models and frontend DTOs must mirror fields and enum semantics.

Proposed workspace organization (names illustrate responsibilities and do not require a single large page):

- **Research home:** project selection, pathway, status, recent work, and gaps/questions.
- **Knowledge workspace:** source library, citations, extraction review, framework/concept/claim editor, version compare.
- **Learning workspace:** objectives, practice, assessments, attempts, competency progress, reproduction evidence.
- **Research workspace:** project overview, literature/gap, phenomena/observations, hypotheses, protocols, evidence/findings.
- **Experiment workspace:** design editor/locking, run monitor, dataset versions/sample inspection, analysis manifests, integrity checklist.
- **Model workspace:** model registry, exact version/spec/parameter comparison, evaluations, validation/replication.
- **Lineage explorer:** queryable provenance graph and cross-project adoption history.
- **Canvas:** optional visual navigation and reasoning surface; nodes reference canonical IDs and edges mirror persisted typed relations where appropriate.

Visual components consume hooks/stores and types; only frontend service modules call Tauri invoke. Stores are cache/workflow state, not independent persistence. Complex forms are driven by typed schemas from domain contracts; every state transition has explicit validation and confirmation as needed.

## 17. User workflows

### 17.1 Acquisition and mastery workflow

1. Create/select a learning project and choose an existing framework/source context.
2. Import/register sources; capture immutable source versions and citations.
3. Segment and extract content; inspect uncertain/machine-generated extracts and approve/correct with history.
4. Build a framework draft from claims/concepts/principles/procedures and link each item to source evidence.
5. Review contradictions and publish a version; publish never overwrites an earlier version.
6. Define objectives, prerequisites, competencies, assessments, and rubrics against a pinned framework version.
7. Practice, submit attempts, and record evaluator feedback.
8. Complete independent reproduction task(s); show evidence, deviations, and rubric.
9. Present mastery as a derived, explainable result with linked attempts/objectives and the exact framework version.

### 17.2 Existing-knowledge investigation workflow

1. Create a research project; adopt exact framework/model versions and review relevant literature.
2. Record a research gap with sources and rationale; create/triage linked questions.
3. Define phenomenon, variables, operational definitions, and competing hypotheses.
4. Create a versioned design and predeclare outcomes, sample, analysis, and bias controls.
5. Lock/preregister design; execute distinct runs against pinned dataset versions.
6. Inspect data quality and deviations; run reproducible analyses.
7. Review evidence and findings including negative/inconclusive outcomes.
8. Propose model/framework refinement, extension, revision, or replacement as a new version with traceable contribution links.
9. Validate/replicate independently and publish a scoped result or transfer an eligible version to another project.

### 17.3 First-principles discovery workflow

1. Create project/domain and record boundaries and intended use.
2. Conduct and record literature/prior-knowledge review, including search scope and source versions.
3. Identify phenomenon and create an observation plan; capture raw observation records before explanatory pattern claims.
4. Explore data; register candidate patterns/ideas and distinguish them from hypotheses.
5. Define concepts/operational measurements and competing falsifiable hypotheses.
6. Preregister experiment design, datasets, analysis plan, sample rationale, and integrity safeguards.
7. Run tests; preserve raw data, deviations, failed runs, analyses, and all results.
8. Create findings and evidence synthesis; construct a model/framework version only with explicit assumptions and provenance.
9. Validate and replicate; revise via a new version and transfer the reusable, scoped knowledge to other projects.

## 18. Mapping: existing components to target

| Existing component | Status | Target role and boundary |
|---|---|---|
| `src-tauri/src/source/{model.rs,db.rs,ipc.rs,revision.rs,ops.rs}` and source UI in `src/pages/sources/` | Existing but requiring extension | Source registry, source segments, extracts, review and provenance foundation. Add source-version identity, typed claims/concepts, and relation integration; preserve the useful revision trail. |
| `src-tauri/src/schema.sql` source tables (`sources`, `segments`, `extracts`, `links`, `source_revisions`) | Existing but requiring extension | Starting persistence for acquisition. Existing polymorphic links are not sufficient as the final scientific relation graph. |
| `src-tauri/src/mind/questions.rs`, `src/shared/layout/QuestionsBoardPage.tsx` | Existing but requiring extension | Inquiry intake/triage workflow; connect durable question identity to project/gap/domain and later scientific records. |
| `src-tauri/src/mind/hypothesis.rs`, `src/shared/layout/HypothesisDetail.tsx`, `src/shared/layout/HypothesesList.tsx`, `src/mind/components/canvas/VoraHypothesisNode.tsx` | Existing but requiring extension | Existing hypothesis and Lab workflow; retain useful protocol, H0/H1, verdict, data-entry, insight, and source hooks while moving toward typed experiments/evidence/lineage. |
| `src-tauri/src/mind/library_groups.rs` and Library UI | Existing but requiring extension | Reusable visual/content collections and UI. May provide framework editor surface but must not be treated as the canonical scientific framework/version model without extension. |
| `src-tauri/src/mind/canvas_db.rs`, snapshot resolver, board transfer, `src/App.tsx` | Existing | Spatial authoring and versioned visualization. Reference canonical domain IDs; scientific facts and relations remain database records outside canvas state. |
| `src/mind/services/dbService.ts`, `src/shared/store/useAppStore.ts`, Tauri commands in `src-tauri/src/lib.rs` | Existing but requiring extension | Typed persistence/IPC/state pattern to extend. Add domain services and explicit errors; avoid moving SQL/business logic into UI/IPC. |
| `src/features/vora-trace/` and `src-tauri/src/vora_trace.rs` | Existing, separate domain | General decision/project trace visualization. It may link to research records but is not a substitute for scientific project, experiment, or evidence entities. |
| `src-tauri/src/market/`, charting, snapshots, and worker | Existing, separate domain with possible adapters | Potential market-data/analysis sources and tools. Integrate samples/datasets through explicit adapters and immutable dataset versions; do not assume chart or worker data is already a scientific dataset or validated result. |
| Learning domain/UI | New | Add objective, assessment, competency, progress, and independent-reproduction workflows. |
| Research project/domain/gap/phenomenon/pattern/finding domain | New | Add canonical research lifecycle, linked to existing questions/hypotheses where migration can establish identity without guessing. |
| Experiment/dataset/analysis/statistical-integrity domain | New | Add structured, reproducible execution and integrity records. |
| Model/version/validation/replication/transfer domain | New | Add full scientific model lifecycle and cross-project knowledge reuse. |
| Universal typed lineage graph | New | Add constrained scientific relations and provenance traversal; visualize through optional canvas and dedicated lineage UI. |

## 19. Phased implementation roadmap

Phases are dependency ordered. Each phase requires its own scoped spec, migration plan, compatibility plan, and test plan before implementation. This master document does not authorize bundling every phase into one release.

| Phase | Deliverable | Dependencies / exit gate |
|---|---|---|
| 0. Domain contracts and migration strategy | Resolve terminology, ID/relation constraints, lifecycle transitions, source/version semantics, storage/checksum and retention policy; inventory actual workspace DB initialization/backup. | This spec; review with product/scientific-method/database owners. Approve an entity/relationship diagram and migration compatibility matrix before schema work. |
| 1. Shared provenance and typed relations | Add relation/provenance contract, source version/citation improvements, queryable lineage, stable typed DTOs; retain compatibility with existing `links` and source revisions. | Phase 0. Must migrate existing records without guessed semantics and prove source→extract trace. |
| 2. Research projects and question/hypothesis integration | Project/domain/gap/phenomenon/question lifecycle; attach existing inquiries/hypotheses to projects; typed operational definitions and transitions. | Phase 1. Backfill unknown project ownership as explicitly unassigned; do not fabricate projects. |
| 3. Datasets and experimental design/run | Dataset/version manifests, samples/observations, design versions, preregistration/locking, run lifecycle, deviation capture. | Phase 2. Requires immutable inputs, FK/transaction tests, integrity declaration design. |
| 4. Analysis, evidence, findings, integrity | Analysis manifests/execution, evidence and finding lifecycles, statistical reporting, bias/leakage checks, reproducibility UI. | Phase 3. Gate on reproducible fixture end-to-end and negative/failed-run preservation. |
| 5. Model lifecycle and evaluation | Model/specification/version/parameters, change classes, metric attribution, validation and replication. | Phase 4. Model version must link to exact findings/designs/data and historical metrics remain unchanged. |
| 6. Learning and mastery | Framework versions, objectives, assessments/attempts, competencies/progress/reproduction and explainable mastery views. | Phase 1 framework/provenance; may proceed independently of Phases 2–5 after Phase 1 contracts. |
| 7. Cross-project transfer and synthesis | Eligibility/review rules, exact-version adoption, provenance, downstream impact, restrictions and reusable validated knowledge. | Phases 1, 4, 5, and 6 as applicable. Gate on end-to-end project A→project B lineage and no duplicated canonical records. |
| 8. Consolidation and operational readiness | Backup/restore, migration support policy, performance/indexing, export, accessibility, UX coherence, documentation and deprecation of temporary compatibility paths. | Relevant feature phases complete; no unresolved data-integrity or provenance failures. |

## 20. Acceptance criteria and testable requirements

Acceptance means behavior is proven through repository tests and an end-to-end workflow against a real temporary workspace database. UI presence alone does not pass.

### 20.1 Shared domain, persistence, and lineage

- Rust and TypeScript contracts agree for each persisted entity, enum, nullability, and timestamp.
- Fresh creation and every supported upgrade path produce the same schema; repeating migrations is safe; transaction failure leaves no partial migration/data mutation.
- Every scientific FK is enforced; invalid relation endpoint/type/state fails with a surfaced error.
- Deleting a board or moving a node cannot delete canonical scientific records or lineage.
- A test can traverse each of the six lineage paths in §6.2 and returns exact source/data/model/framework versions.
- Corrections/retractions preserve original values and create attributable history.
- Backup/restore preserves IDs, checksums, version pins, relationships, and query results; integrity checks report missing assets and invalid references.

### 20.2 Acquisition and learning

- A source with multiple versions can be cited to a precise version and segment; extraction transformations retain source and parent lineage.
- Unreviewed machine extraction cannot be displayed as a reviewed source claim.
- Framework versions preserve exact composition and compare without mutating prior versions.
- Contradictory claims can coexist and remain attributable.
- An assessment attempt pins its assessment and framework versions and cannot be rewritten after submission.
- Mastery is reproducible from configured rule inputs and shows objective/assessment/reproduction evidence; no completion-only shortcut qualifies.
- A research finding cannot create or mutate learner mastery without an explicit learning assessment record.

### 20.3 Research lifecycle

- Projects may use each pathway and transition between pathways without copying sources, findings, hypotheses, or model records.
- Gap acceptance requires prior-knowledge/source references or an explicit reviewed exception.
- Observations, patterns, ideas, hypotheses, and findings have distinct types and the UI presents their different epistemic states.
- Hypotheses include operationalized variables, H0/alternatives, scope, and falsification criteria before preregistration; invalid transitions are rejected server-side.
- One hypothesis can have multiple experiment designs/runs and retains each run outcome, including failed and inconclusive outcomes.

### 20.4 Experiments, datasets, and integrity

- A run pins exact design and dataset versions; edits produce new versions and do not alter past runs.
- Dataset checksum/schema/sample counts are validated on registration/import; sample IDs and split membership are queryable.
- Where a design requires disjoint splits, test data leakage/overlap is rejected or explicitly reported as a violation.
- Required bias/integrity fields cannot be left implicitly blank at preregistration; not-applicable requires a reason.
- Analysis outputs record code/method version, environment, parameters, seeds, inputs, and result/error.
- Failed or repeated analyses create distinct records; analysis never overwrites raw data or prior results.
- Statistics tests cover reported sample counts, units, intervals/uncertainty, exclusions, and multiplicity policy.
- Market experiments, when supported, record fees, slippage, timestamps, instrument, and market context where applicable.

### 20.5 Evidence, findings, models, validation, and transfer

- Evidence polarity and provenance are explicit; source claim, finding, and established-knowledge status cannot be conflated.
- Findings retain scope, uncertainty, limitations, and exact supporting/contradicting evidence.
- Model versions are immutable, have an allowed change class and rationale, and preserve complete parent/contribution links.
- Evaluation metrics are keyed to the exact model and dataset versions plus analysis run; changing the current model leaves historical results byte-for-byte unchanged.
- Validation requires predeclared criteria and records evaluator, scope, evidence, deviations, and outcome.
- Replication is separately recorded; disagreement does not overwrite original findings.
- Project B adoption of Project A framework/model/finding pins the exact version, attributes origin, and supports reverse lineage queries; adaptation creates a new version.
- Restricting or withdrawing future transfer does not erase historical project use or provenance.

### 20.6 Frontend, IPC, and reliability

- All persisted paths follow the approved service boundary; components never call Tauri invoke directly.
- Invalid operations return actionable errors and do not update a success-shaped UI cache.
- State transitions and multi-record operations are atomic; UI refresh after restart matches durable database state.
- User can inspect project, pathway, source citation, evidence, version, and provenance from each relevant detail view.
- The three workflows in §17 can be completed using usable frontend workflows; no capability is classified as implemented based only on a mock or demo graph.
- Focused Rust tests, frontend type-check/build, migration tests, and end-to-end workflow tests pass for each phase. Tests must include legacy-data fixtures and relevant negative cases.

## 21. Explicit non-goals and design safeguards

- This architecture does not prescribe a specific statistical library, external cloud service, LLM provider, or new dependency. Proposals require a separate evidence-based decision.
- A canvas graph is not the system of record for scientific relationships.
- A natural-language note, hypothesis verdict, chart annotation, or versioned board by itself is not a dataset, experiment, statistical analysis, validated finding, or scientific model version.
- AI-generated extraction or analysis is not trusted as reviewed evidence by default; record model/tool, prompt/configuration where permitted, inputs, output, and human review.
- No migration may infer missing provenance, project ownership, experimental status, or scientific validity from titles or free text. Use an explicit unknown/unassigned state and offer reviewed reconciliation.
- Feature completion is not measured by table count. Every subsystem must satisfy persistence, constraints, user workflow, provenance, and acceptance tests.

## 22. Specification governance

This file is the authoritative target architecture. Feature-level specifications in `specs/` MUST reference the relevant sections here and define only their bounded implementation slice, migration, compatibility, tests, and UI workflow. If a feature spec conflicts with this document, resolve the conflict explicitly and update this master document before implementing a divergent architecture.

Architecture changes that alter entity ownership, epistemic distinctions, lifecycle transitions, relation semantics, immutability, or pathway sharing require an ADR or an explicit revision to this specification. Keep the **Audited current state** section evidence-based and update it only after verifying implementation, persistence, and end-to-end behavior.
