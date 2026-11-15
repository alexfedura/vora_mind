# Stage 3.3 — Project Knowledge Lifecycle Integration

## Purpose

Connect the Project Knowledge workflow to existing Research, Experiment,
Analysis, Model, and Knowledge Transfer records through read-only navigation.
Preserve the exact IDs already stored by each domain; do not infer missing
relationships or create a generic graph/persistence layer.

## Verified contracts

- Research questions, hypothesis definition versions, observations, and
  Research design versions carry their own IDs and project context.
- Experiment designs preserve exact Research question, hypothesis definition,
  optional ModelVersion, and DatasetVersion references; Experiment runs and
  dataset pins are separate records.
- Analysis runs preserve exact definition, input, output, evidence, and finding
  version references. Evidence links a typed proposition to a typed artifact.
- ModelVersions carry a creating project and exact parent, hypothesis, finding,
  experiment, dataset, and evidence references. Evaluations, validations, and
  replications are separate records.
- Knowledge Transfer has distinct origin, eligibility, transfer, adaptation,
  and downstream-use records, and may reference an artifact owned by another
  project.
- The Experiment target ModelVersion is an optional ID field. Do not treat
  presence as proof of existence or project authorization; show its exact
  recorded ID and resolve it only against available typed workspace data.

## Requirements

1. Add a read-only lifecycle navigator in the Project Knowledge Workbench.
   Include explicit links to the existing Experiment, Analysis, Model, and
   Knowledge Transfer panels.
2. Load existing workspace DTOs through their typed services. Filter
   project-scoped records by the selected project. Keep the global Transfer
   workspace visible only as explicit origin/recipient lineage.
3. Derive connections only from persisted typed fields. Preserve exact IDs,
   version IDs, relation kinds, and record IDs. Never substitute a latest or
   different version when a referenced ID is absent.
4. Distinguish an unrecorded link from a recorded but unavailable reference,
   an unverified reference when its workspace failed to load, and a known
   artifact from another project. Where supported, let the user explicitly
   switch to that project's context.
5. Add stable in-page anchors to the existing domain panels for records
   surfaced by the navigator. Use the existing Project Knowledge Source and
   FrameworkVersion navigation for those records.
6. Preserve current Research assignment rules, project boundaries, and all
   Stage 3.1 asynchronous guards. New reads must reject stale responses on
   project change, refresh ordering, and component cleanup, including Strict
   Mode setup/cleanup/setup.
7. Fix the Stage 3.2 Provenance-close race first: closing Provenance while a
   request is pending must prevent that request from reopening or repopulating
   the panel; a subsequent intentional open remains possible.
8. Add focused model/guard tests for exact references, unavailable targets,
   project isolation, empty/partial data, and stale-response behavior. Record
   that tests using pure models or simulated state do not prove mounted React
   effect integration.

## Non-goals and limitations

- No schema, migration, DTO, service-write, dependency, or persistence changes.
- No automatic assignment, reuse, transfer, claim synthesis, evidence scoring,
  or ownership conversion.
- No scientific conclusion is inferred from SourceClaims or evidence.
- No backend operation is canceled or rolled back by UI stale-response guards.
- These guards are not authorization or multi-user isolation boundaries.
- Do not begin Stage 3.4.

## Validation

Run focused tests for the changed behavior, `pnpm tsc --noEmit`, and
`pnpm build`. Review the final diff and check line endings without normalizing
unrelated files. Do not access the workspace database.
