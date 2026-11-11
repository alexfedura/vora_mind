# Phase 5 — Model Development, Evaluation, Validation, and Replication

## Authority and stop line

This specification implements the model-development portion of the Vora
scientific architecture. Models are reusable scientific artifacts independent
of hypotheses. This phase extends Phases 1–4 without changing their ownership
boundaries or historical records.

The stop line is before Learning, Knowledge Transfer, learner assessment, and
any claim that a model or finding is scientifically established.

## Domain ownership

- Add a `model` Rust domain after Research, Experiment, and Analysis schema
  initialization. Its migrations are additive, checksummed, transactional, and
  idempotent.
- `Model` is a stable workspace identity. Its optional origin project is
  attribution, not an ownership restriction; one model may be tested in
  multiple projects.
- `ModelVersion` is an immutable snapshot, separate from hypothesis identity
  and hypothesis versioning. A new edit always creates another version.
- The legacy `hypothesis_model_references` table is retained unchanged.
  Existing unresolved references are not guessed or rewritten; new typed
  model relations use foreign keys to exact model and hypothesis versions.
- SQL and scientific lifecycle validation live in the Model repository/service.
  Tauri commands only acquire state and delegate to those operations.
- Relationships use bounded, typed tables and restrictive foreign keys rather
  than a new universal polymorphic endpoint pair.

## Model identity and immutable versions

`Model` preserves a stable ID, display name, description, optional origin
project/domain, creator, and creation time. A status change is an append-only
event; identity rows and versions are never deleted.

Each `ModelVersion` preserves:

- Exact human-readable specification and machine-readable specification.
- Ordered parameter definitions and values, including units, bounds, defaults,
  parameter role, and tuneability.
- Assumptions, scope, dependencies, environment identity, code/artifact
  references, and caller-declared SHA-256 checksums.
- Author, creation timestamp, content checksum, and immutable parent links.
- Exactly one controlled change class, rationale, and expected implications.
- Explicit affected-claim and affected-assumption records.
- Typed links to exact hypothesis versions, finding versions, experiment
  design/run records, evidence, and dataset versions/splits.

`REFINEMENT`, `EXTENSION`, `REVISION`, and `REPLACEMENT` are the only change
classes. A parentless first version is classified as `REVISION` representing
the initial baseline; it requires an explicit baseline rationale and does not
assert validation or scientific correction. A `REPLACEMENT` must link at least
one predecessor, and all predecessor versions remain queryable. Successor
versions require at least one parent from the same model identity. Each
version requires at least one explicit impact record; assumptions can be
identified and recorded as added, changed, or removed, and claims use typed
exact-version/source-claim FKs.

Machine-readable content is validated as JSON and content-hashed by the
backend. Decimal parameter values remain strings. Artifact URIs and declared
hashes are provenance metadata: the application does not claim to fetch or
verify external bytes. Version and relation rows are protected by database
immutability triggers as well as repository transaction rules.

## Typed contribution, contradiction, and evidence links

Persist contributions in `model_contributions`, with a stable relation ID,
actor (when available), creation timestamp, rationale, typed relation kind,
and concrete foreign-key columns for each allowed target. Check constraints
enforce exactly one compatible target per contribution; no unvalidated
polymorphic `(target_type, target_id)` endpoint is used. Partial unique indexes
prevent duplicate links while retaining typed referential constraints.

Support:

- Model version ↔ exact hypothesis definition version, with relation
  `contributes_to` or `contradicts`.
- Model version ↔ exact finding version, with relation `contributes_to`,
  `contradicts`, or `qualifies`.
- Model version ↔ experiment design version/run, with relation
  `contributes_to` or `tested_in`.
- Model version ↔ exact dataset version, explicit role, optional split group,
  and rationale.
- Model version ↔ model evidence record.

Model evidence targets exactly one ModelVersion and exactly one typed artifact:
AnalysisOutput, ExperimentRun, DatasetVersion, ResearchObservation,
SourceVersionSegment, SourceClaim, or FindingVersion. It records controlled
polarity (`supports`, `contradicts`, `qualifies`, `neutral`), scope, quality
rationale, limitations, provenance, actor, and timestamp. Distinct opposing
evidence coexists. It is not a user assertion of model validity.

## Evaluation records

Each immutable `ModelEvaluation` references:

- One exact ModelVersion.
- One completed AnalysisRun and one exact `analysis_run_inputs` row.
- That input's sealed DatasetVersion, role, split group, and sample manifest.
- One AnalysisMetric from the same run, which supplies the exact metric
  definition version, method version, effect, unit, uncertainty, sample size,
  and report fields.
- Project context, evaluator/creator, limitations, and evaluation timestamp.

The service rejects a failed/incomplete AnalysisRun, a metric from a different
run, a mismatched input/dataset/role/split, or project-inconsistent references.
The evaluation does not duplicate or edit AnalysisMetric results. Re-evaluation
creates another record, including for an older ModelVersion. Historical
evaluations and analysis outputs are immutable.

## Validation records

A validation record pins exactly one target: ModelVersion, FindingVersion, or
SourceClaim. It stores a non-empty, predeclared criteria array and checksum,
data/population scope, evaluation method, evaluator identity, explicit
independence state, conflict-of-interest declaration (including an explicit
“none declared” statement), and creation time.

Validation starts in `planned`. A single append-only result event may finalize
it as `pass`, `fail`, or `inconclusive`, with explicit deviations, rationale,
and zero or more typed evidence references. A plan cannot be edited after
creation; a changed criterion or subsequent attempt is a new validation.
Independence and conflicts are recorded, not inferred. A result is scoped only
to its target version/claim, declared criterion, population, and method; it
does not promote a claim to scientific truth.

## Replication records

An immutable replication pins a target ModelVersion and one original
ExperimentRun plus its exact ExperimentDesignVersion. It records `direct` or
`conceptual` type, independent team/operator identity, protocol context,
dataset/version/role/split context, deviations, declared result, evidence, and
provenance. Replication outcomes are explicit reports (`supports`,
`contradicts`, `mixed`, or `inconclusive`); the existence of a record does not
imply success. Multiple replications, including opposing outcomes, coexist and
never rewrite the original run or findings.

## Frontend workflows

Extend the Research workspace with a Model panel providing:

- Registry creation/listing and model/version detail.
- Immutable version creation, including the initial baseline, change class,
  rationale, impact, parameters, assumptions, dependencies, artifacts, and
  parent links.
- Version comparison that displays parent/class/rationale and specification,
  parameter, assumption, and artifact differences without modifying snapshots.
- Contribution/contradiction/evidence inspection with exact endpoint IDs and
  provenance.
- Evaluation creation from completed AnalysisRuns, selecting a run input and
  metric so dataset, role, split, and report values are visibly pinned.
- Planned validation and result recording with criteria, scope, evaluator
  independence/conflict declarations, deviations, and evidence.
- Independent replication recording and history with explicit outcome,
  original experiment/design version, context, deviations, and evidence.

All records refresh from SQLite-backed IPC. Failed requests remain visible as
errors. UI labels distinguish reported outcomes and validation scope from
scientific truth.

## Migration and compatibility

- Initialize the Model migration after Analysis and before application use.
- Never update or delete existing source, research, experiment, dataset,
  analysis, evidence, finding, hypothesis, or model-reference records.
- Preserve legacy unresolved hypothesis/model references verbatim; do not infer
  model IDs or version IDs from text.
- Use restrictive FKs, append-only events, and transactional creation for
  multi-record model versions, evaluation, validation result, and replication.
- Reinitialization and portable workspace restore must retain model records,
  relations, evaluations, validation events, and replications.

## Acceptance tests

Rust repository and integration tests must prove:

1. A model can be referenced by multiple hypotheses/projects without merging
   model and hypothesis identities.
2. Initial REVISION baselines and successor versions satisfy required
   classifications, rationale, impacts, and parent rules; invalid replacement
   and cross-model parent links fail.
3. Published/stored versions, parameters, artifacts, and typed relations
   cannot be updated/deleted; replacements preserve predecessors.
4. Evaluations pin one completed run, exact metric, exact sealed dataset
   version/input/role/split, and cannot be changed by later model versions or
   runs.
5. Validation without predeclared criteria, evaluator, independence state, or
   explicit conflict declaration is rejected; pass/fail/inconclusive outcomes
   are retained and scoped.
6. Replications require an exact original run/design, support direct and
   conceptual kinds, and preserve contradictory reports independently.
7. Invalid typed endpoints and project/version mismatches fail transactionally.
8. Reopening an initialized database preserves the full model/evaluation/
   validation/replication graph and leaves Phase 1–4 fixtures unchanged.

Frontend type-check/build and representative Research workspace workflows must
cover registration, version inspection/comparison, evaluation, validation, and
replication. No Learning or Knowledge Transfer behavior is part of this phase.
