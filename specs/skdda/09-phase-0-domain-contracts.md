# Phase 0 — Canonical Scientific Domain Contracts

**Decision status:** Phase 0 architecture baseline. No production schema, migration, or feature code is authorized by this document.  
**Authority:** [Master scientific architecture](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md).  
**Evidence baseline:** [Current-state audit](../../docs/audit/VORA_CURRENT_STATE_AUDIT.md), [gap analysis](../../docs/audit/VORA_TARGET_GAP_ANALYSIS.md), [existing data inventory](../../docs/audit/VORA_EXISTING_DATA_INVENTORY.md), [migration risks](../../docs/audit/VORA_DEPENDENCY_AND_MIGRATION_RISKS.md), and [readiness report](../../docs/audit/VORA_IMPLEMENTATION_READINESS.md).  
**Related decisions:** [ADR index](./adr/README.md).

This document resolves Phase 0 logical ownership, identity, version, relation, value, and preservation contracts. “Decision” means a repository architecture baseline prepared for product/scientific-owner ratification; it does not assert that absent owners or deployed-data evidence have approved it.

## 1. Decisions and boundaries

1. Keep one selected workspace SQLite database as canonical durable state. Domain ownership is logical; no new scientific database is introduced.
2. Canvas owns layout and visual history only. It references stable scientific entity/version IDs; it never owns scientific meaning, lifecycle, evidence, or canonical relations.
3. Use a typed entity registry plus concrete domain subtype tables and one controlled, typed scientific-relation store. The registry stores identity and ownership only, never entity payloads.
4. Give each entity a stable opaque ID. Versioned content gets a separate immutable version ID; the stable entity ID is not a version number.
5. Project-specific records carry explicit ownership. Reusable workspace knowledge retains its origin and is adopted by reference, not copied or re-owned.
6. Preserve epistemic distinctions: source assertion, extraction, concept, principle, idea, observation, pattern, hypothesis, evidence, research finding, evaluation, validation, replication, and learner competency are not interchangeable.
7. Scientific references default to restrictive deletion. Archive, withdraw, retract, supersede, and correct through explicit lifecycle/provenance events; do not cascade-delete scientific history.
8. Use UTC microseconds for system instants, exact decimal strings plus a declared unit for scientific quantities, and versioned/validated JSON only for structured extensibility.
9. Preserve legacy rows and identifiers. Unknown ownership, type, time unit, units, endpoints, or scientific meaning stay explicitly unknown/unassigned unless evidence and a reviewed mapping resolve them.

## 2. Domain ownership matrix

Each canonical entity has exactly one owning domain. “Used by” domains may reference it but cannot mutate its meaning outside the owning domain’s operations.

| Owning domain | Canonical entities and responsibility | Must not own |
|---|---|---|
| **Knowledge Acquisition** | `Source`, `SourceVersion`, `Segment`, `Extract`, `SourceClaim`, `Concept`/`ConceptVersion`, `Principle`/`PrincipleVersion`, `Framework`/`FrameworkVersion`; source attribution and curated knowledge reconstruction. | Research conclusions, learner mastery, experiment outcomes, visual canvas edges. |
| **Learning** | `LearnerProfile`, `LearningObjective`/`LearningObjectiveVersion`, `Assessment`/`AssessmentVersion`, `AssessmentAttempt`, `Competency`, `CompetencyRecord`, `ReproductionRecord`; learner-scoped progress and assessment evidence. | Scientific claim validity or framework truth. |
| **Research** | `ResearchProject`, reusable `ResearchDomain`, `ResearchGap`, `ResearchQuestion`, `Phenomenon`, `MeasureDefinition`/`MeasureDefinitionVersion`, `ObservationPlan`/`ObservationPlanVersion`, `Observation`, `Pattern`, `Idea`, `Hypothesis`/`HypothesisVersion`, `HypothesisAssessment`; problem framing and inquiry lifecycle. | Execution details, analytical outputs, model evaluations, canvas presentation. |
| **Experiment and Evidence** | `ExperimentDesign`/`ExperimentDesignVersion`, `ExperimentRun`, `Dataset`/`DatasetVersion`, `DataSample` and its `TradeSample` specialization, `AnalysisDefinition`/`AnalysisDefinitionVersion`, `AnalysisRun`, `Evidence`, `ResearchFinding`; protocol execution and evidence synthesis. | Source-attributed claims, model identity/version, or mastery judgments. |
| **Model Development** | `Model`/`ModelVersion`, `ModelSpecification`/`ModelSpecificationVersion`, version-bound parameters, `ModelEvaluation`, `Validation`, `Replication`; model lifecycle and scoped evaluation. | Hypothesis lifecycle, experiment-run execution, or learner competency. |
| **Lineage and Knowledge Transfer** | `ScientificRelation`, `ResearchLineage`, `KnowledgeTransfer`, `ProvenanceEvent`, and the controlled predicate/type-pair catalog. | Domain entity payloads or inferred scientific meaning. |
| **Canvas and Visualization** | Boards, visual objects, placements, visual edges, view state, and snapshots. A visual reference may point to a canonical scientific ID/version. | Any scientific entity, scientific version, canonical evidence link, or scientific lifecycle. |
| **Persistence and Migrations** | Workspace metadata, migration ledger, database connection policy, immutable asset manifest/storage coordination, integrity checks, backup/restore. | Domain transitions or scientific interpretation. |

`ResearchDomain` is reusable workspace taxonomy, not a project container. `ResearchProject` owns its research plan; project-domain membership is explicit. Knowledge objects may be workspace-owned or originate in a project. Adoption by another project does not transfer or change the origin owner.

### 2.1 Ownership state and legacy uncertainty

Every registry row has `ownership_state ∈ {project, workspace, unassigned, unknown}` and nullable `owner_project_id`.

| State | Invariant |
|---|---|
| `project` | `owner_project_id` references an existing `ResearchProject`. |
| `workspace` | No project owner; reusable/workspace-scoped record. |
| `unassigned` | Known to require an owner, but no assignment has been made. Preserve as a deliberate state. |
| `unknown` | Available evidence cannot establish whether or which owner applies. Record why it is unknown. |

The state/owner nullability combination is checked in SQLite. An unassigned record is not silently assigned to a default project. A title, board, directory name, Trace registry entry, or timestamp is not ownership evidence.

## 3. Entity catalog

All IDs below are independent opaque IDs. A “version” is a distinct immutable record, not a mutable field on the stable identity. Required audit fields, ownership, FK behavior, and error semantics are specified in §§4–7 and the linked persistence contract.

### 3.1 Knowledge acquisition

| Entity | Canonical meaning and minimum contract | Version/lifecycle |
|---|---|---|
| `Source` | Stable identity for a publication, URL, book, media item, or other acquired source. Holds source kind, title/creator attribution, stable identifiers/URI where known, language, access/license metadata, and availability. | Identity metadata may be corrected through provenance events. Availability is not source content. Registered → available/unavailable → archived. |
| `SourceVersion` | Immutable edition or captured retrieval of one `Source`; includes source FK, edition/capture identity, observed/captured time, retrieval method, checksum when bytes exist, asset reference when retained, and capture metadata. | A different capture/edition gets a new ID. Captured bytes/metadata are never overwritten; supersession is an explicit relation/event. |
| `Segment` | Addressable region of a `Source`, optionally pinned to a `SourceVersion`; parent/ordering and locator support page, time, line, or stable selector. | Locator correction is a new revision or replacement Segment with lineage; never silently retarget an Extract. |
| `Extract` | Attributed transformation of a Segment or prior Extract: quote, transcription, summary, translation, paraphrase, or other declared transform. Includes method/tool/version, author, confidence, review state, and exact known source-version pin. | Draft → reviewed/accepted/rejected → superseded/retracted. A corrected extraction is a new record. |
| `SourceClaim` | A proposition asserted by an identified source/author, tied to citation and source version when known. It does not assert that Vora verified it. | Assertion content and attribution are immutable. Challenge, correction, and withdrawal are separate events/relations; it is not a `ResearchFinding`. |
| `Concept` / `ConceptVersion` | Reusable semantic identity separated from a source’s wording; version captures definition, aliases/language, scope, and provenance. | Proposed → reviewed → accepted/deprecated. Meaning change creates a new version. |
| `Principle` / `PrincipleVersion` | A stated rule/generalization with applicability conditions, exceptions, scope, and provenance; distinct from a concept or a source quotation. | Proposed → reviewed → supported/contested/deprecated. A changed rule creates a new version; epistemic assessment is not inferred from publication. |
| `Framework` / `FrameworkVersion` | Stable identity for a curated organization of version-pinned concepts, principles, claims, methods, and objectives. A version is an ordered, typed composition with origin, change class, rationale, and exact membership. | Draft → reviewed → published → withdrawn/retired. Published content is immutable; an edit creates another version. A Framework is not automatically a theory or a model. |

### 3.2 Learning

| Entity | Canonical meaning and minimum contract | Version/lifecycle |
|---|---|---|
| `LearnerProfile` | Workspace-local learner identity used by attempts and competency records. It is not an authenticated account or external identity. | Active → archived; account, sharing, and multi-device identity policy remain product decisions. |
| `LearningObjective` / `LearningObjectiveVersion` | Observable learner outcome and immutable criteria/version, with exact framework-version context and optional prerequisites. | Draft → active → retired; criteria changes create a version. |
| `Assessment` / `AssessmentVersion` | Stable assessment identity and immutable definition version containing tasks/items, rubric, scoring method, mastery rule, and pinned objective/framework versions. | Draft → reviewed → locked/published → retired/withdrawn. An attempt always pins one `AssessmentVersion`. |
| `AssessmentAttempt` | One learner’s submission against one exact assessment version; responses/artifacts, submission time, evaluator and evaluations are retained. | In progress → submitted → evaluated. Submission is immutable; evaluator corrections append a new evaluation event. |
| `Competency` | Reusable ability descriptor and criteria, independent of any one learner’s attainment. | Proposed → active → retired; descriptor changes are revisioned/versioned. |
| `CompetencyRecord` | Append-only learner-specific evidence that criteria were or were not demonstrated, with evaluator, rubric/version, and supporting attempts/reproduction. | `not_assessed`, `in_progress`, `demonstrated`, `not_demonstrated`, or `needs_review` outcome on a record. Current mastery is a derived view, not a mutable truth flag. |
| `ReproductionRecord` | A learner’s distinct attempt to independently reproduce a pinned framework procedure/result, with conditions, artifacts, evaluator/rubric, deviations, and outcome. | Planned → attempted → evaluated; retries are new records. Time spent, checklist completion, or course completion alone never implies mastery. |

### 3.3 Research

| Entity | Canonical meaning and minimum contract | Version/lifecycle |
|---|---|---|
| `ResearchProject` | Scoped scientific work with aim, pathway(s), owner, lifecycle, governance context, and project plan. It owns project-specific research records. | Proposed → active ↔ paused → completed → archived. Archive never cascades to its records. |
| `ResearchDomain` | Reusable subject classification/vocabulary. It is not a project or a gap. | Proposed → active → deprecated; parent/domain cycles are rejected. |
| `ResearchGap` | Evidence-backed statement of missing/limited knowledge, scoped to reviewed literature/prior knowledge and uncertainty. | Candidate → assessed → accepted/rejected → addressed/superseded. Acceptance requires a cited basis or explicit exception. |
| `ResearchQuestion` | Scoped, answerable project question, optionally linked to gap/domain, rationale, triage, and resulting hypotheses/findings. It is not a Gap. | Parked → triaged → active → answered/closed/archived. Promotion to a Hypothesis does not erase or change question identity. |
| `Phenomenon` | Scoped phenomenon of interest, population/context and temporal/spatial boundary. It is not an observation or explanation. | Identified → characterized → retired; scope change is revisioned. |
| `MeasureDefinition` / `MeasureDefinitionVersion` | Versioned operational definition for a variable/outcome, including quantity kind, unit, scale, procedure and interpretation. | Draft → reviewed → active → retired. Changed semantics create a new version. |
| `ObservationPlan` / `ObservationPlanVersion` | Intended observation method, target phenomenon, units/measures, instrument, sample frame, timing, inclusion rules, and quality controls. | Draft → reviewed → preregistered/locked → executed/withdrawn. Lock pins an immutable version. |
| `Observation` | A measurement/record of what was observed, with context/time precision, quantity/value state, units, instrument/source, sample, and provenance. It is not an interpretation or finding. | Captured → quality-reviewed → accepted/excluded/retracted. Captured values are immutable; correction creates a superseding Observation. |
| `Pattern` | Exploratory regularity inferred from specified observation/dataset versions and analysis method, with filters/uncertainty. It is not raw data, a causal claim, or a confirmed finding. | Candidate → examined → supported/unsupported/retired. |
| `Idea` | Unvalidated proposal for a mechanism, question, or model; records origin and motivating observations/patterns. | Captured → explored → promoted/abandoned. Promotion links to a new typed record; it does not change the Idea’s epistemic class. |
| `Hypothesis` / `HypothesisVersion` | Falsifiable proposition identity plus versioned H0/alternatives, operational definitions, variables/units, population/scope, expected direction/effect, assumptions and falsification criteria. | Draft → reviewed → preregistered → testing → closed. Preregistration pins a version. Outcome lives in separate assessment records. |
| `HypothesisAssessment` | Immutable, scoped summary of evidence for a particular HypothesisVersion, exact runs/evidence, criteria, evaluator and uncertainty. | Outcome: supported, partially supported, rejected, inconclusive, or unresolved. A new assessment supersedes/revises the recommendation without rewriting prior runs. It is not Model Validation. |

### 3.4 Experiment and evidence

| Entity | Canonical meaning and minimum contract | Version/lifecycle |
|---|---|---|
| `ExperimentDesign` / `ExperimentDesignVersion` | Stable protocol identity plus immutable, preregisterable plan: target hypothesis/model version, estimand, design type, outcomes, factors/covariates, sampling, inclusion/exclusion, stopping rule, bias controls, and analysis plan. | Draft → reviewed → preregistered/locked → retired/withdrawn. Every edit after lock is a new version. |
| `ExperimentRun` | One actual execution of exactly one design version, with times, operator, environment, exact inputs, deviations, outputs, and terminal result. | Scheduled → running → completed/failed/cancelled/inconclusive. Each retry is a distinct run; terminal scientific content is immutable. |
| `Dataset` / `DatasetVersion` | Stable logical collection plus immutable sealed manifest/content version. A version pins schema, source/transform lineage, roles/splits, counts, checksums, and external asset refs. | Dataset registered → active/restricted → retired. Version staged → validated → sealed → superseded/quarantined. No filename-based split inference. |
| `DataSample` | Stable sample identity within an exact DatasetVersion; supports values, context, time, split role and quality/missingness state. | Captured/validated/excluded; correction is a new sample or version with provenance. No scientific cascade delete. |
| `TradeSample` | Typed specialization of `DataSample` for trading/market context (instrument, order/execution, fees, slippage, timing, market context, units). | Same identity, FK, immutable-version and lifecycle rules as DataSample; not a separate disconnected dataset model. |
| `AnalysisDefinition` / `AnalysisDefinitionVersion` | Stable method identity and immutable exact specification/code/parameters/metric plan used by analyses. Versioning is required so a run never points at a floating “latest” method. | Draft → reviewed → locked/retired. Method changes create a version. |
| `AnalysisRun` | One execution of one AnalysisDefinitionVersion against immutable input versions, with code/environment, parameters, seeds, logs/errors, outputs and metrics. | Queued → running → completed/failed/cancelled. Re-run gets a new ID; failed runs are preserved. |
| `Evidence` | Provenance-bearing artifact or observation/analysis result plus explicit assessment of its relationship to a proposition, scope, polarity, quality rationale and limitations. Evidence is not the proposition or the synthesis. | Recorded → assessed/accepted/contested/rejected → superseded/retracted. Artifact and origin are immutable. |
| `ResearchFinding` | Bounded conclusion synthesized from specified analyses/evidence, including uncertainty, scope, limitations, and review state. `Finding` is shorthand only; there is one canonical type, not a generic Finding table. | Draft → reviewed → accepted/contested/unsupported/inconclusive → superseded/retracted. Corrections preserve prior findings. |

### 3.5 Model development and lineage

| Entity | Canonical meaning and minimum contract | Version/lifecycle |
|---|---|---|
| `Model` / `ModelVersion` | Model is a stable explanatory/predictive/procedural identity independent of a hypothesis. Version pins specification, parameters, assumptions, dependencies, scope, artifacts/checksums, parent versions and change class/reason. | Model proposed → active → deprecated/replaced/retired. Version draft → reviewed → released → superseded/withdrawn; released content immutable. |
| `ModelSpecification` / `ModelSpecificationVersion` | Typed specification artifact for equations, algorithm, interface, assumptions and format; an exact ModelVersion implements an exact specification version. | Draft → reviewed → released/retired. Specification changes create a version. |
| `ModelEvaluation` | One metric/rubric evaluation of one ModelVersion against exact DatasetVersion/split and AnalysisDefinitionVersion/AnalysisRun, including metric units, estimates and uncertainty. | Queued → running → completed/failed/inconclusive. Re-evaluation creates a new record. |
| `Validation` | A separately governed, predeclared assessment of a named version/finding against explicit criteria, population, method, evaluator-independence/conflict and evidence. | Planned → executing → passed/failed/inconclusive → superseded. “Passed” is scoped to the named criteria; it is not proof of truth. |
| `Replication` | A distinct direct/conceptual independent attempt tied to original design/run/finding, with team/operator, protocol/data context, independence declaration, deviations and outcome. | Planned → executing → completed/failed/inconclusive → assessed. It never replaces the original. |
| `ResearchLineage` | Explicit ancestry chain from exact origin entity/version through any adaptation to downstream project/entity/version. It is queryable provenance, not ownership transfer. | Append-only lineage facts; correction/retraction is evented. |
| `KnowledgeTransfer` | Auditable workflow for proposing/reviewing/accepting/rejecting/adapting/withdrawing use of an eligible exact reusable version by a recipient project, including attribution, context, license and origin. | Proposed → under review → accepted/rejected → adapted/withdrawn. Withdrawal affects future use, not historical provenance. |
| `ScientificRelation` | One validated directed assertion between typed registry endpoints with a controlled predicate, project context, applicable exact version pins, and provenance. | Append-only assertion. Retract/supersede with a new ProvenanceEvent; do not delete the old relation. |
| `ProvenanceEvent` | Append-only actor/action/time record for creation, correction, review, lifecycle transition, relation retraction, import, or migration. Carries typed target identity, reason, and optional before/after hashes; raw payload is not required. | Immutable. Unknown legacy actor/time is explicit and is never fabricated. |

## 4. Stable identities and versions

- New IDs are opaque lowercase UUIDv7 strings (the repository already depends on `uuid` with v7/Serde support). IDs are never derived from titles, paths, status, timestamps, or ordinal sequence.
- Rust uses per-domain ID newtypes serialized as strings. TypeScript uses branded string ID aliases in the canonical type module. Legacy IDs remain unchanged and are not rewritten to UUIDs.
- Each versioned family has a stable identity ID and distinct `*_version_id`. Version rows carry `entity_id`, monotonic sequence within that identity, parent-version link(s), author/time, change class/reason and content checksum where relevant.
- Version number/sequence is not the version identity. A version ID remains stable if display numbering is later corrected.
- Draft versions are immutable snapshots too; edits create a replacement version or explicit draft revision. Lock/preregister/release makes the exact version usable by downstream runs.
- Runs, observations, submitted attempts, evidence artifacts, findings, evaluations, validations, replications, and relations retain immutable scientific meaning. Corrections create superseding records/events.
- Multi-parent histories are allowed where composition is real; parent FKs are restrictive and ancestry cycles are rejected in the owning domain.

## 5. Relation architecture decision

Use the **validated entity registry plus typed subtype tables** for cross-domain scientific references. A registry row holds only `id`, `entity_type`, `workspace_id`, ownership state/project, and creation identity. Every entity payload remains in exactly one concrete owner-domain table. A subtype carries a fixed, checked entity type and a composite FK to `(registry.id, registry.entity_type)`.

`ScientificRelation` has composite FKs for subject and object `(entity_id, entity_type)` to the registry and a FK to an allowlist row `(predicate, subject_type, object_type)`. Version pins reference `(version_id, entity_id)` and must match the corresponding endpoint. SQLite `CHECK`/FK constraints enforce identity existence; a seeded predicate/type-pair catalog plus insert/update triggers enforce permitted endpoint types and required version pins. Rust repeats the validation for actionable errors. This is not a generic JSON object store.

**Why this choice:** the scientific graph is intentionally cross-domain and its vocabulary will grow; dedicating a physical join table to every allowed predicate pair would multiply migrations and complicate provenance traversal. Unlike an unconstrained `(kind,id)` pair, the registry has real FKs, typed subtype rows, a finite predicate/type-pair allowlist, and version-match checks. The cost is a registry and carefully tested triggers/catalog migrations. Details and limits are in [ADR-001](./adr/ADR-001-typed-scientific-entity-registry.md).

Use explicit typed FK/junction tables only for stable owner/subtype composition where they enforce useful uniqueness/cardinality (for example one run input per named slot, framework-version membership, assessment-version objective membership). These are not a second ad-hoc graph strategy; all cross-domain semantic assertions still use ScientificRelation.

## 6. Persistence-level constraints and indexes

The logical design requires these physical constraints when a later implementation adds schema:

- `scientific_entities`: `PRIMARY KEY(id)`, `UNIQUE(id, entity_type)`, checked `entity_type` vocabulary; `CHECK` owner-scope/project-ID consistency; FK workspace and `owner_project_id`; indexes `(workspace_id, entity_type, owner_scope, owner_project_id)` and `(entity_type, created_at_us)`.
- Domain subtype: `PRIMARY KEY(id)` and restrictive composite FK to `(scientific_entities.id, entity_type)`; scientific payload fields are typed columns. A scientific entity cannot exist only as a registry row.
- `scientific_versions`: unique version ID; FK to stable entity; `UNIQUE(entity_id, sequence_no)` and `UNIQUE(version_id, entity_id)`; indexes `(entity_id, sequence_no)` and `(entity_id, created_at_us)`. Concrete version subtype FKs to the version identity.
- `scientific_relations`: unique opaque relation ID; composite endpoint FKs; predicate/type-pair FK; endpoint-version FKs; context-project FK where supplied; indexes `(subject_id, predicate, created_at_us)`, `(object_id, predicate, created_at_us)`, `(context_project_id, predicate, created_at_us)`.
- Structural junctions: composite PK/unique constraints on their semantic key; e.g. `(framework_version_id, member_entity_id, member_version_id, role)`, `(assessment_version_id, objective_version_id)`, `(run_id, input_slot)`, `(dataset_version_id, sample_id)`.
- All scientific FKs specify `ON DELETE RESTRICT` (or SQLite's equivalent `NO ACTION` where a cyclic insertion-order constraint requires it). No scientific cascade. FK columns are indexed.
- Checks constrain enum spellings, decimal syntax/scale bounds, valid ranges, non-empty required IDs, valid lifecycle values, and ownership/null combinations. Cross-row state transitions and predicate rules run through backend operations and defensive SQLite triggers.
- Lifecycle/history tables (`provenance_events`, relation events, assessment evaluations and version/revision records) are append-only; UPDATE/DELETE triggers reject mutations.

The complete conceptual ERD is [here](./10-phase-0-entity-relationship-diagram.md). This is a contract, not production DDL.

## 7. Timestamp, numeric, missingness, JSON, and asset contracts

### Timestamps and dates

- System instants use signed UTC Unix **microseconds** (`i64` in Rust, safe integer number in TypeScript for the supported date range), column/field suffix `_at_us` or `_time_us`. Do not reuse legacy integer values without verified unit evidence.
- DTOs serialize the integer directly as JSON number; converters range-check JavaScript safe integer bounds. No local-time strings for instants.
- Scientific observation time separately carries time precision and source/original timezone representation when known. A date-only or uncertain time is not promoted to a fabricated instant; use explicit temporal precision/status and preserve original text/offset.
- Civil dates such as a journal day use ISO `YYYY-MM-DD` strings and are not Unix timestamps.

### Numeric measurements and unknowns

- Integer counts use checked signed 64-bit integers where the scientific definition is integral.
- Measured values, prices, rates, thresholds, currency and parameters use canonical base-10 decimal strings plus a unit code (UCUM where available) and dimension/scale metadata. Do not serialize these as JavaScript `number` or store them as SQLite `REAL`.
- Rust validates decimal syntax and range at the domain boundary. Unit conversions create a derived value/record with source value, conversion rule/version, and rounding policy; never overwrite the input.
- Missingness is a separate enum (`observed`, `not_measured`, `not_recorded`, `unknown`, `not_applicable`, `withheld`, or domain-approved `invalid`). It is never zero, an empty string, `NULL` without a meaning, or a successful default.
- Probabilities, confidence, currency and measurement uncertainty have documented scale/unit and allowed range; no generic `confidence` field without semantics.

### JSON

- Core query, lifecycle, identity, relationship, unit, status, timestamp, and version fields are typed columns/relations, not opaque JSON.
- JSON is limited to versioned schemas for protocol/configuration payloads, structured responses, provenance details, and extensible non-critical metadata.
- Every JSON payload includes a stable `schema_id` and positive `schema_version`, is checked against that exact schema by Rust, and has typed Serde DTOs for supported versions. Unknown versions are preserved as raw data and reported; never silently decode to an empty/default shape.
- `null`, absent field, empty string/list/object, and explicit unknown are distinct. Serde/TypeScript nullability and omission behavior are documented per DTO. Hashes of opaque payloads cover exact stored bytes; a semantic JSON hash requires a separately specified canonicalization algorithm.

### Assets and checksums

- A workspace-managed immutable asset is represented by a `WorkspaceAsset` manifest row: opaque asset ID, SHA-256 lowercase hex, byte length, media type, storage-relative key, availability/integrity state, and created time.
- Content is written to a staged file, hashed, placed by content digest, and only then referenced by a committed DB row. Digest collisions/mismatch fail explicitly. Paths from imports are normalized and cannot escape the workspace root.
- DatasetVersion, SourceVersion, Evidence, ModelVersion, and AnalysisRun pin asset IDs and their captured hashes. File availability is not conflated with historical existence.
- Asset garbage collection is out of scope until restrictive-reference checks, full-workspace backup, restore, and crash recovery are implemented and tested.

## 8. Rust and TypeScript contract location

- Rust owns domain validation and transitions. Domain entities/operations live under owner modules (`source`, `learning`, `research`, `experiment`, `model`, `lineage`, `persistence`); IPC command modules transport explicit DTOs and contain no SQL/business rules.
- TypeScript canonical scientific interfaces/enums live in the approved `src/types/index.ts` contract location unless a later ADR explicitly adopts a generated typed domain submodule. Feature stores/components import these types; they do not restate them.
- Rust uses Serde `rename_all = "camelCase"` for DTO field names and `rename_all = "snake_case"` for enum wire values. Optional values are `Option<T>` serialized as explicit `null`; omitted fields are used only for documented patch requests, never conflated with null.
- Add Rust Serde golden tests and TypeScript compile-time/runtime shape tests against the same JSON fixtures before exposing each IPC contract. Do not add a codegen dependency during Phase 0.
- Contract examples, field/nullability rules, and proposed DTO shapes are in [the Rust/TypeScript contract plan](./12-phase-0-rust-typescript-contract-plan.md).

## 9. Legacy compatibility and scope

The current audit establishes a composite, mostly unversioned schema and does not establish a deployed schema-version matrix. Therefore this architecture does **not** claim that any existing `.vora`, `.db`, or imported database is a supported automatic upgrade source. Phase 1 must first gather owner-approved sanitized fixtures, fingerprint exact known variants, and create a compatibility matrix. Unknown variants are preserved and rejected for automatic conversion, not coerced.

Existing source/question/hypothesis/canvas/Library/Market/Forge tables and IDs remain in place through a compatibility window. Legacy rows may stay in legacy tables behind adapters. No title/text/path-based inference; no conversion of Trace projects, source revision events, hypothesis JSON, chart candles, worker tasks, Library groups, or canvas edges into scientific entities absent explicit reviewed mapping. Detailed gates are in [migration and transaction contract](./13-phase-0-migration-and-transaction-contract.md).

## 10. Explicitly unresolved and not implemented

- Product/scientific-owner ratification of terminology, lifecycles, access/privacy policy, knowledge transfer policy, and supported workflows.
- The actual deployed workspace version/fixture matrix; exact legacy FK violations and references.
- Whether a scientific Theory merits a distinct entity. Until decided, no Theory table; express theory-like material as a Framework/Model composition with explicit roles and claims.
- External identity/authentication, shared users, collaboration, and signing/attribution authority. `LearnerProfile` is local workspace identity only.
- UCUM coverage/custom units governance, numerical range/precision policy by domain, and sub-microsecond acquisition requirements.
- External asset format/Parquet policy, encryption, quotas, cloud sync, and complete package/restore behavior.
- Production SQL normalization, trigger implementation, relation predicate catalog seeding, and query/performance proof.
- Rust/TypeScript DTO code, migrations, UI, dataset ingestion, experiment execution, analysis engine, or scientific backfill.

These are gates for later implementation. Phase 0 stops here; it does not begin Phase 1.
