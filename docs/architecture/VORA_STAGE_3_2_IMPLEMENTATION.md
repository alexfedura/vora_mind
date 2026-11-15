# Stage 3.2 — Project Knowledge Workbench

## Status

Stage 3.2 implements a read-oriented Project Knowledge Workbench within the
Planning → Research workspace. It connects existing Research, Source, and
Framework records without changing domain ownership, persistence contracts, or
database schema.

## Workflow

- The overview filters Questions, gaps, observations, patterns, ideas,
  hypotheses, design versions, literature reviews, evidence, and adoptions to
  the selected Research Project. Unassigned and other-project records are not
  presented as belonging to the selected project.
- A ResearchProjectSource association is displayed separately from
  SourceVersion citations. Associations without citations remain unversioned
  project context; citations without associations remain Research references.
- Literature citations and SourceClaim hypothesis evidence preserve exact
  Source, SourceVersion, optional SourceLocation, and SourceClaim IDs.
  Evidence without a recorded SourceVersion is visible as unresolved and is
  not assigned a substitute.
- Selecting an exact reference inspects that SourceVersion, its locations,
  extractions and review states, claims, recorded claim relationships, and
  provenance. SourceClaim statements are identified as source-attributed,
  not verified scientific conclusions. Extraction review is not truth
  validation.
- Links to the Source Browser carry `sourceId`, `sourceVersionId`,
  `sourceLocationId`, and `sourceClaimId` when available. The destination
  selects only the requested exact version and reports an unavailable version,
  location, or claim rather than silently substituting another record.
- Project FrameworkVersion adoptions remain separate from workspace-shared
  published versions. The workbench inspects the exact adopted version and
  members, shows recorded claim matches to the project's Research references,
  and links to the existing adoption form and Source Browser workflows.
  Publication, adoption, and Knowledge Transfer are not conflated.

Source acquisition, extraction review/correction, Research mutations,
Framework publication, adoption, and Knowledge Transfer continue to use their
existing domain UI and services. The workbench adds navigation and context;
it does not create new records or perform implicit assignments or transfers.

## Asynchronous behavior

`useProjectKnowledgeWorkbench` uses `LatestRequestGuard` for Source, exact
SourceVersion, claim-relationship, provenance, and FrameworkVersion reads.
The Source Browser also protects list/bundle selection requests, and its
acquisition view only loads locations/extractions for the selected exact
SourceVersion. Obsolete responses and errors cannot publish into a newer
selection or after hook cleanup.

These are UI publication guards. They do not cancel IPC calls, undo completed
backend writes, provide authenticated authorization, or enforce project
membership.

## Implementation and tests

- `specs/vora-stage-3-2-project-knowledge-workbench.md` defines the feature
  contract and acceptance checks.
- `src/shared/layout/projectKnowledgeModel.ts` derives project-scoped
  references and distinguishes association from citation.
- `src/shared/layout/ProjectKnowledgeWorkbench.tsx` renders the overview and
  exact-reference navigation.
- `src/hooks/useProjectKnowledgeWorkbench.ts` loads Source, location,
  extraction, claim, relationship, provenance, and FrameworkVersion details
  with latest-request guards.
- `src/pages/SourcesPage.tsx`, `src/pages/sources/SourceDetailView.tsx`, and
  `src/pages/sources/AcquisitionWorkspace.tsx` support exact-reference
  navigation in the existing Source Browser.
- `src/shared/layout/ResearchWorkspacePage.tsx` hosts the workbench and
  provides stable navigation targets to existing Research and Framework
  controls.
- `tests/projectKnowledgeWorkbench.test.mjs` verifies project filtering,
  exact-reference preservation, and separation of Source association from
  Research citation.

The focused Node tests exercise the production model and the existing
production async-guard/state helpers. Some state tests manually simulate
publication. React components and effects were not mounted; browser event and
effect lifecycle integration remains unverified.

## Validation evidence

Validated during this implementation:

- `node --experimental-strip-types --test tests/projectKnowledgeWorkbench.test.mjs tests/researchAsyncGuards.test.mjs tests/projectWorkspaceState.test.mjs tests/projectSourcesState.test.mjs`
  — 30 tests passed.
- `pnpm tsc --noEmit` — passed.
- `pnpm build` — passed; Vite reported existing dynamic-import/chunk-size
  warnings.
- `git -c core.whitespace=cr-at-eol diff --check` — used for the repository's
  CRLF source files.

No backend, schema, migration, database, dependency, lockfile, release, or
version changes are part of Stage 3.2.
