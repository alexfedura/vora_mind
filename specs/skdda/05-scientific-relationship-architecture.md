# SKDDA Phase 0 — Scientific Relationship Architecture

**Purpose:** Define relationship semantics and integrity requirements for an implementation-ready typed graph without writing production SQL.

## 1. Why the current relationships are insufficient

The repository currently has several separate relation-like systems:

- `source.links` links an extract to a polymorphic `(target_kind,target_id)` with string `link_kind`; the extract has an FK, but the target does not.
- `extracts.parent_extract_id` and `superseded_by` have self-FKs.
- `segments.parent_segment_id` has a self-FK.
- `hypotheses` has `parent_hypothesis_id`, `refined_from_id`, `inquiry_id`, source card/highlight IDs, and library promotion ID; these are not all DB-enforced FKs.
- Canvas edges are snapshot/delta data; their endpoints are IDs in visual graph state.
- `hypothesis_definitions` is a typed junction table pinning library versions.
- Forge has a separate code-specific lineage graph.

These are real but domain-limited. No universal scientific relation contract or cross-project traversal is established. Canvas edges, Trace edges and source link strings are not interchangeable.

## 2. Relationship record contract

Every scientific relation has:

- Stable relation ID.
- Subject and object typed endpoint identifiers, with enforceable reference integrity.
- Predicate from a controlled/versioned vocabulary.
- Context/owning project when interpretation is project-specific.
- Exact version IDs where a versioned endpoint is relevant.
- Direction; inverse display must not create a second contradictory fact.
- Created-by/time, provenance/citation, rationale and scope where relevant.
- Optional confidence/quality only where semantics are defined.
- Retraction/supersession event; do not delete historical relation silently.

### 2.1 SQLite referential integrity options

Do **not** rely solely on a row `(subject_type, subject_id, object_type, object_id)` because SQLite cannot declare a polymorphic FK to multiple target tables. Phase 0 must choose one of:

1. **Typed relation tables/junctions (preferred where endpoint pair count is bounded):** e.g. `claim_source_citations`, `hypothesis_experiment_designs`, `evidence_hypotheses`, `model_version_findings`; each FK targets concrete tables and uses constraints for predicate/cardinality.
2. **Entity registry plus subtype tables:** every canonical scientific entity has a row in a registry keyed by entity ID/type; subtype tables have FK to registry; generic relations FK both endpoints to registry. Registry deletion is restrictive and type consistency is checked in transaction/service (and with triggers where reliable). This adds one canonical identity layer and must not become an untyped JSON bucket.
3. **Hybrid:** typed junctions for critical relations, entity registry for genuinely open-ended graph edges, with endpoint type/subtype checks in service and DB triggers.

Decision criteria: enforceability, query workload, relation vocabulary extensibility, migration complexity, and ability to validate subtype/predicate pairing. Do not implement mixed strategies ad hoc.

Foreign-key behavior should be `RESTRICT`/no delete for referenced research records; use explicit retraction/archive and tombstones. Cascades may apply only to disposable non-scientific caches. Use transactions to validate both endpoint subtype and edge insertion.

## 3. Predicate contract

“Subject → object” is the stored direction. Cardinality notation is per individual relation row: a subject may have **0..N** relation rows and an object may be referenced by **0..N** rows unless noted. Exact singleton constraints (e.g. one dataset version per immutable run input slot) are additional uniqueness rules. Provenance is mandatory for all predicates; version pinning applies whenever a versioned object is referenced.

| Predicate | Allowed subject → object | Direction and cardinality | Required validation / provenance |
|---|---|---|---|
| `derived_from` | Extract, concept, claim, observation, pattern, idea, hypothesis, finding, model/framework version → source/extract/observation/dataset/analysis/prior version. | Directed derivation; N:M. | Transformation and exact input IDs/versions; no self-edge or cycle for strict derivation ancestry unless explicitly approved. |
| `tests` | ExperimentDesign/Run → Hypothesis or ModelVersion. | Design targets 0..N; hypothesis/model can be tested by N designs/runs. | Exact design version; run must use the design version and report target unchanged. |
| `supports` | Evidence/Finding/Validation/Replication → Claim/Hypothesis/Finding/ModelVersion/FrameworkVersion. | Directed evidential stance; N:M. | Scope, polarity strength/rationale, artifact, exact versions and analysis provenance. |
| `contradicts` | Evidence/Finding/Validation/Replication → Claim/Hypothesis/Finding/ModelVersion/FrameworkVersion. | Directed conflict; N:M. | Conflict scope and rationale; never auto-delete supported evidence. |
| `validates` | Validation → ModelVersion, Finding, FrameworkVersion, or specified claim. | Validation targets one or more explicit versioned objects; target receives N validation records. | Predeclared criterion/method/population, evaluator, outcome and evidence refs. |
| `refines` | New Hypothesis, ModelVersion, FrameworkVersion, Claim revision → prior same-semantic family/version. | Directed newer→prior; lineage may branch. | Change rationale; records exact predecessor(s); not proof of truth. |
| `extends` | New ModelVersion/FrameworkVersion/Claim version → prior version. | Directed newer→prior; N:M parents if composed. | Scope delta and compatibility; preserve old scope/history. |
| `revises` | New Claim/Finding/ModelVersion/FrameworkVersion → prior record/version. | Directed newer→prior. | Explicit changed assumption/conclusion, rationale and evidence. |
| `supersedes` | New version/record → old version/record of same logical family. | Directed newer→old; a predecessor can have at most one current successor only if product policy requires it; branches otherwise allowed. | Supersession reason and status; old record remains queryable. |
| `replicated_by` | Original ExperimentRun/Finding → Replication record. | Original has 0..N replications; replication links to exactly one primary original (or explicit multiple originals). | Replication kind, independence, design/input versions, deviations and result. |
| `based_on` | Hypothesis, Design, Analysis, Finding, ModelVersion, FrameworkVersion, LearningObjective → source/claim/concept/finding/framework/model/dataset version. | Directed dependency; N:M. | Exact cited versions, context and method of use. |
| `originated_from` | Any derived/reusable scientific record → source, phenomenon, question, project, or prior record. | Directed provenance; N:M. | Origin identity and creation context; distinguish origin from derivation. |
| `cites` | Claim, Gap, Finding, FrameworkVersion, Design → SourceVersion/SourceSegment/Claim/Finding. | Citation reference; N:M. | Citation locator, exact source/version, quotation/page/time and attribution as available. |
| `defines` | Source claim/Concept/Principle/FrameworkVersion → Concept/term/variable or defined object. | Definition relation; N:M. | Definition text/context/source and version; terms may have multiple sourced definitions. |
| `contains` | FrameworkVersion/Assessment/ExperimentDesign/DatasetVersion → typed member/criterion/sample. | Composition; N:M subject with ordered membership where ordering matters. | Membership role/order and exact version; no duplicate member/role without explicit allowance. |
| `measures` | ObservationPlan/ExperimentDesign/Instrument → Phenomenon/Variable/Outcome. | Directed measurement intent; N:M. | Operational definition, unit, instrument and scale. |
| `observed_in` | Observation → Phenomenon, ExperimentRun, DatasetVersion, or context. | Observation has one required primary context and optional secondary contexts. | Timestamp, unit, sample and source/instrument provenance. |
| `analyzed_by` | DatasetVersion/ObservationSet → AnalysisRun. | Dataset/version may feed N analyses; AnalysisRun has one or more exact inputs. | Input manifest, split/role, code/method version, parameters. |
| `produces` | ExperimentRun/AnalysisRun/AssessmentAttempt → Observation/DatasetVersion/Evidence/Finding/ModelEvaluation. | Producer has 0..N outputs; each output has explicit provenance. | Run ID, timestamps, output checksum and exact version. |
| `evaluates` | AnalysisRun/AssessmentAttempt/ModelEvaluation/Validation → ModelVersion/Claim/Hypothesis/Competency. | N:M, constrained by entity semantics. | Metric/rubric/criterion version, target scope, input version and outcome. |
| `adopts` | ResearchProject/LearningObjective/Design → exact FrameworkVersion/ModelVersion/Finding/Concept/Principle. | Recipient adopts N reusable items; item adopted by N projects. | Transfer record, exact version, attribution/license, use context and date. |
| `implements` | Code artifact/Procedure/ModelVersion → ModelSpecification/Framework procedure/Algorithm. | N:M; do not assume source-code link equals validated implementation. | Artifact hash/version, environment, relationship review and validation reference. |
| `assesses` | AssessmentAttempt/Validation/Review → LearningObjective/Competency/Claim/Finding/ModelVersion. | Evaluator record targets explicit object/version. | Criteria/rubric/version, evaluator, evidence, outcome and date. |
| `retracts` | Retraction/correction event → Claim/Evidence/Finding/Relation/Version. | Event targets one or more records. | Actor/time/reason and preserved prior state; cannot physically erase history. |

## 4. Project lineage and transfer rules

1. A record created in Project A retains origin project A even when adopted in Project B.
2. `adopts` references the exact immutable version, never a floating “latest” pointer.
3. Adaptation in B creates a B-owned record/version and `derived_from`/`based_on` upstream links.
4. Transfer is an auditable workflow record with eligibility, attribution, review, license/access, acceptance/rejection and withdrawal state.
5. Downstream traversal must resolve origin → intermediate adaptations → receiving project → downstream findings/models.
6. Project archival, workspace organization changes, or canvas deletion cannot cascade-delete transferred knowledge.

## 5. Canvas and visual graph mapping

Canvas edges may be projections of canonical relations. The edge stores presentation attributes and canonical relation ID. If an edge is deleted, the system must ask whether the user intends to retract the relation or only remove the visualization; these are separate operations. Board snapshots may preserve historical visual state but do not replace relation event history. Relations created from a canvas must still pass backend endpoint/predicate validation.

## 6. Required tests

- Every allowed endpoint pair inserts with FK-valid IDs; disallowed endpoint/predicate pairs fail.
- Deleted/missing endpoints fail even when the frontend submits a valid-looking ID.
- Versioned endpoints resolve exactly after newer versions are added.
- Cycles are rejected for acyclic predicates or explicitly supported for graph predicates per approved contract.
- Duplicate/singleton constraints are enforced under concurrency.
- Retraction preserves the prior edge and its traversal history.
- Cross-project adoption and adaptation has bidirectional provenance traversal.
- Canvas edge add/move/delete cannot silently change canonical scientific relation state.
