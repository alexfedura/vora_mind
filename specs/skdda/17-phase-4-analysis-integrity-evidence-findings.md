# Phase 4 — Analysis, Statistical Integrity, Evidence, and Findings

## Authority and scope

This phase implements reproducible analysis records, statistical-integrity
contracts, typed evidence, and scoped research findings on the existing
workspace SQLite database. It extends the Phase 3 dataset/run foundation and
preserves all prior records. Model identity, model development, model
evaluation, and arbitrary code execution remain out of scope.

The execution model is **external capture**: analysts run code in their chosen
environment and register the exact method, inputs, logs, outputs, and result
reports in Vora. Vora does not execute user-supplied code. Method and metric
versions must pin implementation and test-suite identities, and completed
reports are validated by Rust; the application does not claim to verify the
scientific correctness of externally computed values.

## Architecture

- Add an `analysis` Rust domain after Experiment initialization. Its additive,
  checksummed migration owns analysis, evidence, finding, and immutable asset
  reference tables. Tauri commands delegate to its services/repositories.
- An AnalysisDefinition has immutable AnalysisDefinitionVersions. Each
  version pins one exact AnalysisMethodVersion, exact MetricDefinitionVersions,
  parameters/schema, protocol version, and complete statistical-integrity
  declarations. A changed method or plan creates another version.
- AnalysisMethodVersion and MetricDefinitionVersion preserve code/formula
  identity, assumptions/units, and a checksummed test-suite report. Locking
  requires a non-empty test manifest and all declared tests passing. Vora
  validates the report structure and checksum; external tests are not run by
  the application.
- Every AnalysisRun is a distinct immutable execution record. Run creation
  pins the exact definition version, parameters, environment, seeds, and
  protocol. Inputs pin sealed DatasetVersions, explicit roles, split groups,
  checksums, and exact sample assignments. Inputs are never resolved through
  a floating “latest” pointer.
- Input selections snapshot sample membership and split assignment. The
  backend validates membership against the pinned versions and rejects
  duplicate/overlapping samples when the declared design requires disjoint
  splits. For applicable temporal analyses, each sample's feature-availability,
  prediction, decision, and outcome-availability timestamps are validated and
  retained; a feature available after prediction is a blocking leakage error.
- A run is persisted before external work begins. Logs, errors, deviations,
  output manifests, metrics, and terminal state are append-only or immutable.
  Finalization is transactional; failed runs and retries remain distinct.
  Output artifacts are staged outside SQLite transactions, content-hashed,
  copied to workspace-managed storage, then referenced from committed output
  rows. Analysis never mutates a dataset or prior output.

## Statistical integrity and reporting

The AnalysisDefinitionVersion carries an explicit typed applicability state
for every safeguard: `applicable`, `not_applicable`, or `not_assessed`.
`not_applicable` requires a non-empty reason. Missing JSON fields or invalid
declarations fail deserialization/validation; `not_assessed` is persisted as
unresolved, never as passed. Server validation produces persisted integrity
findings used by both IPC and UI. Blocking defects (invalid chronology,
invalid sample membership, violated required disjointness, malformed
declarations, or invalid test manifests) prevent run start/finalization;
unassessed controls remain visible warnings.

The typed plan covers overfitting and selection, leakage and data cutoffs,
sampling/selection/survivorship, look-ahead, multiplicity, dataset separation,
out-of-sample usage, walk-forward boundaries, forward-test execution context,
and reproducibility. Exact dataset checksums and transformation lineage are
captured with the run.

Each quantitative AnalysisMetric pins an exact MetricDefinitionVersion and
stores its estimand, sample size, statistical method/version, decimal effect
estimate and unit, uncertainty interval or a reason it is not meaningful,
assumptions, exclusions, missing-data handling, multiplicity treatment, and
optional p-value. P-values are not a substitute for effect size, uncertainty,
or scope. Values are preserved as decimal strings; invalid intervals, missing
report fields, or mismatched metric versions are rejected.

## Typed evidence and findings

- Each Evidence record points to exactly one typed proposition
  (SourceClaim, exact HypothesisDefinitionVersion, or exact ResearchFinding
  version) and exactly one typed artifact (AnalysisOutput, ExperimentRun,
  DatasetVersion, ResearchObservation, or SourceClaim). Concrete foreign keys
  and a one-target constraint prevent dangling or semantically invalid targets;
  no universal polymorphic target pair is introduced.
- Evidence records one controlled polarity: `supports`, `contradicts`,
  `qualifies`, or `neutral`, plus exact scope, quality rationale, limitations,
  provenance, actor, and timestamp. Distinct contradictory evidence coexists.
- A ResearchFinding is a stable identity with immutable versions. Each version
  states proposition/scope, applicability, uncertainty, limitations, outcome
  (including null, contradictory, or inconclusive), exact analysis lineage,
  and exact evidence references. Review/contestation events are append-only;
  corrections create a new version with an explicit rationale and parent.
- Findings and evidence remain assessments, never application-verified facts.
  Later contestation does not rewrite an earlier result or finding.

## Frontend workflows

The Research workspace provides method/metric version registration, definition
versioning and integrity review, run start/finalization and failure inspection,
exact input/sample provenance, server-produced safeguard warnings, artifact and
metric inspection, evidence creation with typed targets/polarity, and finding
review, revision, contestation, and comparison. Persisted state and unresolved
controls are visible without inferring a pass from absence of a warning.

## Compatibility and migration

- Preserve all Phase 1–3 source, research, experiment, dataset, sample, and run
  records and IDs. Add no speculative mappings from legacy Lab text, raw JSON,
  visual edges, or project titles.
- Apply the additive Analysis migration after Experiment. Migration checksum
  mismatches and invalid foreign keys fail explicitly. Repeated initialization
  is idempotent.
- New findings/evidence reference exact typed endpoints and use restrictive
  foreign keys. Model targets are not accepted until the Model domain exists.
- Managed artifact bytes and their checksums survive application restart;
  database and asset writes are staged so no committed row points at an
  unverified file.

## Acceptance tests

Rust repository/service tests prove exact analysis definition, dataset-version,
role, split, and sample pins; temporal leakage rejection; required and
not-applicable integrity semantics; failed and repeated run retention; tested
method/metric version requirements; required statistical report validation;
artifact checksum and restart persistence; typed evidence target rejection;
contradictory evidence coexistence; valid null/inconclusive findings; append-only
contestation and revision; and unchanged historical outputs after later runs.
Representative end-to-end fixtures initialize a workspace, register method
and metric versions, create and run an analysis, record artifacts/results,
connect evidence to a proposition, revise/contest a finding, and reopen the
database to verify the complete lineage.

The stop line is before Model Development, evaluation, and analysis-code
execution inside Vora.
