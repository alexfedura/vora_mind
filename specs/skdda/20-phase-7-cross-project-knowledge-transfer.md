# Phase 7 — Cross-Project Knowledge Transfer and Lineage

## Authority and boundary

Implement controlled transfer as typed, project-scoped provenance. A transfer
records reuse of an exact existing scientific artifact version; it does not
copy or change that artifact. This phase ends after the Project A → Project B
transfer acceptance tests pass. Do not start general consolidation.

## Scope decisions

- Phase 7's initial artifact allowlist is `FindingVersion`, `ConceptVersion`,
  `PrincipleVersion`, `FrameworkVersion`, and `ModelVersion`. Each kind has a
  concrete SQLite FK and a tagged Rust/TypeScript reference. Adding another
  type requires a reviewed contract, typed FK, validation, UI support, and
  tests; arbitrary `(kind, id)` references are rejected.
- Existing concept and principle identities have no version or project-origin
  fields. Add immutable, project-attributed versions; never infer or backfill
  their origin from a title, source link, framework membership, or creator.
- Framework identities and versions are workspace-scoped rather than
  project-owned. Their origin project is a reviewed transfer-origin assertion,
  not a mutation to the framework.
- A finding originates in its owning ResearchProject. A ModelVersion carries
  its creation project and remains part of its stable Model identity. An
  origin assertion must be consistent with those existing records.
- ResearchProject is the transfer participant in this phase. LearningProject
  remains a distinct learning context and continues to pin its exact
  FrameworkVersion; adding LearningProject as a transfer endpoint requires a
  separate typed integration and is not inferred from a ResearchProject ID.
- Eligibility requires an explicit, reviewed license identifier, access scope,
  allowed purpose, attribution, permission basis, and recorded permission.
  Unknown rights remain ineligible. The application enforces the declared
  constraints locally; actor IDs are audit metadata, not authenticated
  authorization, and the application does not assert external legal validity.

## Lifecycle and persistence

- `KnowledgeTransferOrigin` asserts the exact artifact version and origin
  project with actor, time, and rationale. Its concrete artifact reference is
  restrictive and immutable.
- `TransferEligibility` is an immutable rights/permission snapshot. Eligibility
  events append `eligible`, `restricted`, or `withdrawn`; restriction and
  withdrawal block future adoptions only. Regranting creates a new eligibility
  snapshot and event history, never edits or erases existing transfers.
- `KnowledgeTransfer` pins one eligibility snapshot, origin project, receiving
  project, purpose, context, and request time. Acceptance transactionally
  rechecks current rights, purpose, recipient scope, distinct projects, and
  source lifecycle. The accepted transfer is the adoption record and does not
  create a duplicate artifact.
- Attribution and use context are immutable snapshots on the transfer.
- `TransferAdaptation` points to a distinct exact artifact version created in
  the receiving project. Its typed FK and project check prevent adaptation by
  mutating or reusing the upstream version. The rationale and upstream transfer
  are preserved.
- `ResearchLineage` records adoption and adaptation edges. `DownstreamUsage`
  references an exact typed artifact, ModelEvaluation, AnalysisRun, or
  ExperimentRun in the receiving project. Forward and reverse queries include
  the transfer, source version, attribution, adaptations, and downstream use.
- All artifact and project references use restrictive FKs. Lifecycle and
  lineage history is append-only. Multi-record operations are transactional.
- The checksummed Phase 7 migration is initialized after Research, Analysis,
  Model, and Learning. Existing framework adoptions and canonical records
  remain readable and unchanged.

## Eligibility policy

- Only published FrameworkVersions, released ModelVersions, reviewed
  ConceptVersions/PrincipleVersions, and non-draft/non-retracted
  FindingVersions may be newly offered.
- Finding epistemic state, validation scope, limitations, and evidence are
  displayed as provenance; transfer does not promote or validate a finding.
- Origin, license, permission, and transfer declarations are explicit records.
  Missing ownership, permission, or license data prevents new adoption.
- `workspace` access applies to all ResearchProjects; `selected_projects`
  access requires an explicit non-empty receiving-project set.
- Intended purpose must match the immutable permitted-purpose set. Restricting
  or withdrawing future eligibility never invalidates prior use.

## Adaptation and evidence lineage

- Adaptations use an exact artifact version created in the receiving project;
  the originating artifact remains untouched. Project-specific editors remain
  responsible for creating a new finding/model/framework/concept/principle
  version before the transfer panel can attach it as an adaptation.
- The lineage edge records `based_on` or `derived_from`, rationale, source
  transfer, source exact version, destination exact version, project endpoints,
  actor, and time.
- Provenance previews expose source finding evidence, referenced source
  claims/segments, relevant analysis and experiment runs, and model evidence
  where those typed records exist. Missing or unknown links remain explicitly
  absent; the transfer service does not fabricate provenance.

## Frontend workflows

The Research workspace includes a transfer panel for:

- Browsing eligible exact versions and inspecting status, origin, rights,
  attribution, validation scope, limitations, and available provenance.
- Recording reviewed origin assertions and complete eligibility grants.
- Adopting an exact version with purpose, recipient, attribution, and context.
- Recording an adaptation against a distinct version already created in the
  receiving project.
- Recording exact downstream model/evaluation/analysis/experiment use.
- Restricting or withdrawing future eligibility without deleting history.
- Querying transfer history and both upstream and downstream lineage.

Errors from eligibility or permission checks are surfaced; visual state is
derived from persisted records.

## Acceptance tests

1. Project B adopts an exact Project A FindingVersion without changing its
   identity, version, or owner.
2. An origin version with unknown ownership, incomplete license/permission,
   disallowed purpose, selected-project mismatch, or disallowed lifecycle
   cannot be adopted.
3. Attribution, context, exact version, and origin survive database backup,
   restore, and reopen.
4. A Project B adaptation has a distinct identity/version and a queryable
   `based_on`/`derived_from` edge to the Project A version.
5. Forward and reverse lineage and downstream usage return the expected
   projects, versions, evidence, and transfer history.
6. Restricting/withdrawing eligibility blocks new adoptions but retains prior
   adoptions, attributions, adaptations, and usage.
7. Two opposing or contested findings remain unchanged and independently
   transferable when eligible.
8. Existing source and framework-adoption records remain intact.
9. Run focused Rust tests, the Project A → Project B integration/backup test,
   `cargo check`, `pnpm tsc --noEmit`, and a frontend production build.
