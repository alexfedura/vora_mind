# SKDDA Phase 0 — Lifecycle and Versioning Specification

This is a proposed state and history contract. It does not claim the repository implements these lifecycle machines. Existing implementation qualifications are linked from [03-domain-gap-analysis.md](03-domain-gap-analysis.md).

## 1. Operation classes

| Class | Meaning | Examples |
|---|---|---|
| Mutable draft | Editable before it has been used as a scientific input. Every edit updates revision metadata; optimistic concurrency is recommended. | Project description; unreviewed gap; draft design. |
| Version-producing | Change to an artifact whose exact content is used by downstream work creates a new immutable version. | Framework, model, experiment design, assessment, dataset, analysis method. |
| Append-only | New events/results are added; prior events remain. | Observation capture, run, attempt, evidence assessment, evaluation. |
| Immutable after lock/submit/close | Content used for a run or submitted evidence cannot be edited in place. | Preregistered design, closed experiment run, submitted assessment, dataset version. |
| Superseding | A new record explicitly identifies a prior record it replaces or corrects. | Corrected observation, revised finding, newer model version. |
| Retraction-only | The record may be marked withdrawn/retracted with reason and actor; the original payload remains. | Incorrect claim, evidence link, finding, or transfer. |

## 2. Lifecycle matrix

| Entity | Proposed lifecycle | Mutable and locking rules | History/version policy |
|---|---|---|---|
| ResearchProject | proposed → active ↔ paused → completed → archived | Metadata editable while active; close/archive requires confirmation if active runs remain. | Status events retained; project never cascades its scientific records. |
| ResearchQuestion | parked → triaged → active → answered/closed; any nonterminal may archive | Text/rationale mutable before downstream preregistration; after use, corrections are revisions. | Preserve identity through promotion where possible; otherwise explicit derived relation. Existing deletion must be constrained once referenced. |
| ResearchGap | candidate → assessed → accepted/rejected → addressed/superseded | Evidence basis editable during assessment; acceptance requires cited literature or documented exception. | Gap changes scope through revision; never erase addressed history. |
| Hypothesis | draft → reviewed → preregistered → testing → supported/partially supported/rejected/inconclusive | Draft may edit; preregistration locks proposition/definitions for runs. Verdict cannot rewrite runs. | New hypothesis/version for changed statement after preregistration; assessments and results append. Existing H0/H1 are strings and need typed extension. |
| ExperimentDesign | draft → reviewed → preregistered/locked → retired/withdrawn | Editable before lock; locked design cannot be changed in place. | Design change creates version with parent and reason; run pins version. |
| ExperimentRun | scheduled → running → completed/failed/cancelled/inconclusive | Append run logs/deviations; close seals results and exact input manifest. | Each execution is a new ID; errors/failures retained. |
| Dataset | registered → active → restricted/retired | Identity/metadata may be revised; restrictions cannot erase historical use. | Stable logical dataset with separate immutable versions. |
| DatasetVersion | staged → validated → sealed → superseded (by later version) | Before sealing, validate checksum/schema; after seal, no content update. | Immutable manifest and payload checksum; split assignments pinned. |
| AnalysisRun | queued → running → completed/failed | Run inputs/code/config frozen at start; append logs; finalize outputs once. | Re-run creates new analysis ID; do not overwrite prior output. |
| Evidence | recorded → assessed → accepted/contested/rejected; may be superseded/retracted | Original artifact/provenance immutable; evaluation is a new event. | Polarity is an explicit relation, not mutable sentiment field. |
| Finding | draft → reviewed → accepted/contested/unsupported/inconclusive → superseded/retracted | Draft can edit; accepted finding corrected via new version/finding and relation. | Retain scope, uncertainty, and evidence snapshot. |
| Model | proposed → active → deprecated/replaced/retired | Stable identity; status is event-driven. | New model identity for replacement when conceptually distinct. |
| ModelVersion | draft → reviewed → released → superseded/withdrawn | Draft editable until release; released payload immutable. | Every version pins specification/parameters/artifact and parent(s), change class and evidence. |
| ModelParameter | defined → active in version → retired | No in-place changes to values used in released version/run. | Version parameters are immutable; execution may snapshot overrides separately. |
| ModelEvaluation | queued → running → completed/failed/inconclusive | Inputs and metric definitions frozen; result immutable. | Re-evaluation creates new record tied to exact model/data/analysis versions. |
| Validation | planned → executing → passed/failed/inconclusive → superseded | Criteria locked before evaluation; record deviation. | Each validation attempt is retained; new criteria require new validation. |
| Replication | planned → executing → completed/failed/inconclusive → assessed | Protocol and independence declarations captured before execution. | Separate record linked to original; never replaces original. |
| KnowledgeFramework | draft → reviewed → published → superseded/retired | Identity stable; published content changes only through new version. | Publication/retraction events retained. |
| FrameworkVersion | draft → reviewed → published → withdrawn/superseded | Immutable once published/adopted; unpublished draft may be replaced with history. | Exact membership and source/evidence references stored. |
| KnowledgeTransfer | proposed → under review → accepted/rejected → adapted/withdrawn | Destination use only after acceptance if policy requires. | Transfer events and prior adoptions remain after withdrawal. |
| AssessmentAttempt | in progress → submitted → evaluated | Submitted responses immutable; evaluator correction appends evaluation revision. | Each attempt remains; mastery derives from policy/versioned rubric. |
| LearningProgress | event recorded | Append-only. | Current progress is projection; never overwrite earlier attempts. |
| Observation | captured → quality-reviewed → accepted/excluded/retracted | Captured values immutable; correction requires superseding observation. | Preserve instrument/context/source/time and correction reason. |

## 3. Scientific meanings are separate

### Hypothesis support

An assessment of how observed evidence relates to a particular hypothesis under one design/run scope. It may be supported, partially supported, rejected, or inconclusive. It does not by itself validate a model.

### Model validation

A validation record applies predeclared criteria to a specific model version and population/data context. It may fail or be inconclusive. A passed validation is scoped to the criteria.

### Independent replication

A separate experiment conducted by an identified independent/direct/conceptual replicator, with protocol, data context and deviations. Replication may agree or disagree; mere existence is not confirmation.

### Learner mastery

Derived from learning objectives, assessment attempts, competencies and independent reproduction evidence under an explicit rubric. It is not a property of the scientific validity of the framework.

### Established knowledge

A governance/review state over an evidence synthesis, with policy version, scope, reviewers and qualifying independent evidence. It is not equivalent to hypothesis support, one validation result, or learner mastery.

## 4. Model change taxonomy

| Change class | Required meaning and evidence |
|---|---|
| Refinement | Improves precision/performance within prior assumptions and scope; state what is unchanged and what evidence motivated change. |
| Extension | Adds scope/capability while maintaining the prior domain; identify compatibility and new applicability boundary. |
| Revision | Changes mechanism, assumptions or conclusions due to new evidence or identified error; link contradicted/revised claims. |
| Replacement | Distinct successor is recommended instead of prior model; create a distinct model identity if semantics differ, with explicit replacement relation. |

The class is mandatory for a released successor, but is not a validation verdict. Model versions and all evaluation results remain addressable after replacement.

## 5. Transition validation and transaction rules

- Transitions are backend domain operations, not arbitrary status assignments from the UI.
- State transition plus associated relation/event must commit atomically.
- A preregister action checks required design fields and integrity declarations before locking.
- Run finalization binds actual outputs/deviations; no analysis may run against an ambiguous floating “latest dataset.”
- Version release checks complete specification, parameters and provenance links.
- Retraction/supersession adds events and never physically deletes evidence needed to understand prior conclusions.
- UI may show derived state but must preserve a visible history and explain why transitions are unavailable.

## 6. Transition test cases

For each entity: valid path; invalid skip/back transition; edit before/after lock; duplicate transition; concurrent transition; failure midway through transaction; retraction; supersession; history query; migration of legacy unknown status. Tests must assert the old record is unchanged and the new event/version is queryable.
