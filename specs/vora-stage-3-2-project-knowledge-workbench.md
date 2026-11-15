# Stage 3.2 — Project Knowledge Workbench

## Purpose

Provide a project-centered entry point for navigating existing Source,
Research, and Framework records through the Project Knowledge Loop. Reuse
existing records and service contracts; do not create a duplicate graph or
new persistence relationship.

## Contract

- A project Source association is workspace-shared Source context only. It
  does not imply a SourceVersion citation.
- Exact SourceVersion, SourceLocation, SourceClaim, LiteratureReviewSource,
  HypothesisEvidence, and FrameworkVersion IDs remain explicit. A selected
  exact version must not silently fall back to another version.
- The project view includes only records explicitly scoped to that project,
  including its Source associations, literature citations, hypothesis
  evidence, and FrameworkVersion adoptions. Shared catalog visibility is not
  project ownership.
- SourceLocation and extraction review state, SourceClaim attribution,
  recorded relationships, and provenance are presented distinctly. Review
  state does not establish scientific truth.
- Framework publication, project adoption, and Knowledge Transfer eligibility
  remain separate. No transfer or cross-project copy is performed.
- Existing source-acquisition and Research services are reused. No database
  schema, migration, domain ownership, or backend contract change is planned.
- Project/source/version/claim reads use latest-request and lifecycle guards;
  obsolete results, errors, and finalizers cannot replace the current view.

## User workflow

From Planning → Research, select a Research Project and inspect its explicit
Source associations and exact Research citations. Select a Source or follow
an exact citation to inspect its SourceVersions, locations, extractions,
claims, claim relationships, and provenance. Review project Questions,
Hypotheses, evidence, and FrameworkVersion adoptions in the same project
context. Existing Source Browser controls remain the place for acquisition
and review mutations.

## Acceptance checks

- Project filtering excludes unrelated and unassigned Research records.
- A Source association with no citation is shown as unversioned project
  context; no citation or ownership is inferred.
- Exact citation selection preserves its SourceVersion and optional location
  and claim IDs.
- Missing source/version/relationship data is reported as missing or
  unavailable, not synthesized or substituted.
- Out-of-order project/source/version requests cannot publish stale data.
- Existing Source, Research, and Framework services remain the persistence
  boundary; no schema or database changes are made.
