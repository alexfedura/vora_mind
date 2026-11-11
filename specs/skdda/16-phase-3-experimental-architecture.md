# Phase 3 — Experimental Architecture and Dataset Management

## Authority and boundary

This phase implements the experiment/run and dataset/sample portion of the
approved scientific architecture and extends the Phase 2 Research design
records. It preserves existing research projects, hypothesis versions, design
IDs, legacy Lab records, sources, observations, and dataset references.
Execution is limited to recording and managing runs; statistical analysis,
findings, model development, and interpretation are out of scope.

## Design and preregistration

- Keep `research_experiment_designs` and
  `research_experiment_design_versions` as the stable design identity/version
  foundation. Add an exact-version typed specification, append-only version
  lineage, and preregistration record in an Experiment-owned schema.
- A design specification captures question and hypothesis/model targets,
  estimand and study type, groups/intervention/control, factors/outcomes,
  covariates, unit of analysis, sampling and eligibility, sample size/stopping,
  randomization/blinding/instruments, operational definitions, schedules,
  preregistered analysis and multiplicity plans, deviation policy, exact
  dataset-version/role pins, code/environment, seeds and reproducibility
  instructions. Market context is a typed optional extension.
- Rust validates applicable required fields and project/version relationships.
  A preregistration operation snapshots the canonical specification checksum,
  reviewer, timestamp, and rationale in one transaction. Locked specs cannot
  be updated or deleted. Revisions create a new draft version and preserve the
  prior version and rationale.
- Existing designs without typed specifications remain unchanged and are
  explicitly represented as legacy/unconfigured. They cannot be used to create
  new runs until a complete new version is made; their historical statuses are
  not rewritten.

## Runs and lifecycle

- Every run has a unique, non-reusable ID and references exactly one
  preregistered immutable design version.
- Lifecycle transitions are `created -> ready -> running -> completed`,
  `failed`, `cancelled`, or `inconclusive`, with failed/cancelled outcomes
  retained. Rust validates transitions; database constraints prevent deletes
  and immutable identity/design-pin changes.
- Run creation pins the exact sealed dataset versions and explicit dataset
  roles compatible with the design. Floating “latest” references are rejected.
- Starting execution persists `running` before returning control to any long
  computation. The application does not hold a SQLite transaction or lock for
  the duration of computation. Completion/failure and declared output pins,
  final log, and terminal state are committed atomically. Append-only logs,
  deviations, and lifecycle events preserve failure context. No statistical
  computation is added.

## Datasets, assets, and samples

- Dataset identity is separate from immutable DatasetVersion identity. Version
  creation is staged; sealing validates its versioned sample schema, stored
  sample count, asset checksums, and manifest checksum. Sealed versions cannot
  be modified or deleted.
- Dataset transformations record a method, canonical parameters, provenance,
  all exact input dataset versions, and the exact output version.
- Dataset assets use workspace-managed opaque keys, SHA-256, byte length, and
  media type. Import stages/copies and hashes the asset before its manifest is
  committed. External paths are not treated as durable asset identity.
- `DataSample` is the canonical queryable sample shape. Samples have stable
  IDs, exact version membership, typed fields, units, timing, missingness, and
  provenance. `TradeSample` extends this shape with typed instrument, order/
  execution, market/session, fee, slippage, and timestamp context. Scientific
  decimals remain canonical strings.
- Split membership is explicit and names one of `TRAINING`, `VALIDATION`,
  `TEST`, `EXPLORATORY`, `PROSPECTIVE`, or `FORWARD`. Roles are never inferred
  from paths or filenames. A sample can have only one assignment per split
  group; assignments across groups remain independent.

## Migration and compatibility

- Experiment tables have an ordered, checksummed domain migration ledger and
  initialize after Research, Sources, and legacy Lab schemas.
- Do not rewrite Research migration version 1 or infer typed design fields,
  dataset roles, or runs from legacy Lab text/JSON. Existing dataset references
  remain readable and distinct from canonical sealed DatasetVersions.
- Domain behavior and database constraints live in Rust repositories/services;
  Tauri commands only acquire state and dispatch typed requests. The frontend
  uses typed DTOs/services and never writes SQL or decides lifecycle authority.
- Multi-record version creation, preregistration, dataset sealing, run creation,
  start, and finalization are transactional. Asset I/O and checksumming occur
  outside write transactions.

## User workflows

The Research workspace adds structured design authoring/revision/comparison,
preregistration and lock inspection, run creation/progress/deviation/log
inspection, dataset identity/version registration and verification, sample
browsing, and role/split visualization. Persisted lifecycle and verification
state is displayed explicitly. Existing Research and Lab workflows remain.

## Acceptance tests

Rust tests cover exact design/dataset pins, required-field preregistration
gates, immutable revisions and past runs, valid/invalid transitions, durable
failed runs, checksum mismatch detection, schema/count validation, stable
sample queries, role/split disjointness, transformation lineage, asset
integrity, and restart persistence. The existing Research migration fixture
must remain intact. Frontend type-check/build and backend compilation are
required. The Phase 3 stop line is before analysis, findings, and model
development.
