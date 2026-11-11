# Phase 2 — Research Domain and Scientific Inquiry

## Authority and scope

This implementation follows [the scientific architecture](../../VORA_MIND_SCIENTIFIC_ARCHITECTURE.md),
Phase 0 contracts (`09`–`13`), and the audited runtime schema path in
`src-tauri/src/mind/portable_db.rs`. It does not treat the duplicate
`src-tauri/src/schema.sql`, Vora Trace projects, canvas edges, Library groups,
or legacy Lab JSON as canonical Research records.

The Research domain owns project plans, reusable domains, pathways, literature
reviews, prior-knowledge assessments, gaps, question lifecycle/lineage,
phenomena, observation plans and records, dataset references, patterns, ideas,
and typed hypothesis definitions. It references immutable SourceVersion and
FrameworkVersion identities from Phase 1. It does not execute experiments or
analyses, create statistical results, or convert legacy Lab data into
observations/evidence/findings.

## Persistence and compatibility

- Add a Research-owned schema/migration ledger and DDL; do not rewrite the
  existing `questions`, `hypotheses`, source, Library, or canvas schemas.
- Preserve every legacy question and hypothesis ID and payload. The migration
  adds one typed Research identity per legacy row, marks ownership
  `unassigned`, and marks unmapped scientific state `legacy_unassessed` or
  `legacy_untyped`. No project, gap, operational definition, unit, method,
  dataset, or evidence is inferred.
- New Research questions also create their legacy Questions row in the same
  transaction, preserving the existing board, triage, and promotion workflow.
- New hypotheses retain a legacy Lab row for existing protocol, H0/H1,
  checklist, verdict, insights, source-link, and canvas behavior; typed
  definitions are authoritative and separately versioned.
- Referenced Research questions cannot be physically deleted. Existing
  unreferenced question deletion remains supported.
- Project-domain and pathway memberships are explicit many-to-many relations.
  Domains are reusable; research plans and artifacts are project-owned or
  explicitly unassigned. Framework adoption pins the exact immutable version
  and does not change framework ownership.
- Source citations pin the exact SourceVersion and, when supplied, a location
  and SourceClaim from that same version.

## Lifecycle and integrity

- Accepting a ResearchGap requires at least one reviewed LiteratureReview or
  PriorKnowledgeAssessment relation, or a separately recorded and reviewed
  exception with reviewer, rationale, and timestamp.
- Multiple ResearchQuestions may reference one gap. Question lifecycle is
  separate from legacy priority triage and promotion status. Parent/derived
  lineage is typed, FK-backed, cycle-checked, and append-only.
- Phenomenon, Observation, Pattern, Idea, and Hypothesis have distinct typed
  tables and DTOs. Observations pin a plan version, conditions, time, unit and
  exact decimal or explicit missingness state. A dataset-backed observation
  references an immutable external dataset-version manifest; a Pattern must
  declare one or more observation/dataset inputs and its method.
- Hypotheses have immutable definition/design versions with typed H0/H1,
  competing-hypothesis relations, scoped population, expected effect,
  falsification criteria, assumptions, and typed independent/dependent/
  covariate variables with operational definitions, optional units, and
  measurement methods. Preregistration requires the typed fields and locks
  the version. Legacy text remains available but is not backfilled as typed
  definitions.
- Experiment support is design-only: one hypothesis can link to multiple
  exact design versions. There are no execution, run, or statistical-analysis
  records in this phase.
- Hypothesis evidence links are typed to SourceClaims, Observations, and
  Patterns with supports/contradicts/qualifies polarity. Model/version
  references remain explicitly unresolved until the Model domain exists.
- Multi-record create, review, transition, lineage, adoption, and linking
  operations are transactional. Rust validates all state transitions,
  ownership/project consistency, exact version endpoints, and evidence.

## UI workflows

The Planning workspace gains a Research tab for project/pathway setup, reusable
domains and assignment, exact-version knowledge adoption, literature/prior
knowledge review, evidence-backed gaps, question lifecycle/lineage, phenomena,
plans, observations, dataset references, patterns, ideas, and typed hypothesis
comparison/design links. Existing Queue, Triage, Lab, and canvas entry points
remain available. Legacy/untyped and unreviewed epistemic state is shown from
persisted values, not inferred badges.

## Acceptance tests

Rust repository tests cover project/pathway ownership, reusable domains,
version-pinned adoption/citations, gap evidence/exception gates, multiple
questions and lineage, distinct observation/pattern records, hypothesis
operational definitions and preregistration, competing hypotheses, multiple
design versions, valid/invalid transitions, and restart persistence. A
legacy-schema fixture verifies original question/hypothesis rows and IDs
survive migration with unassigned ownership and unknown typed definitions.
Frontend type-check/build and compilation of all Rust test targets are release
gates. Deployed workspace compatibility remains uncertified without approved
database fixtures.
