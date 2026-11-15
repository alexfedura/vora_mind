# Stage 3.3 — Project Knowledge Lifecycle Integration

## Status and scope

Stage 3.3 adds a read-only lifecycle navigator to the Stage 3.2 Project
Knowledge Workbench. It connects existing Research, Experiment, Analysis,
Model, and Knowledge Transfer DTOs in the selected Research project context.
It adds no persistence records, schema, migrations, backend commands, or
write operations.

The navigator derives only explicit typed references. It preserves exact
Question, HypothesisVersion, SourceClaim, SourceVersion, SourceLocation,
FrameworkVersion, ExperimentDesignVersion, ExperimentRun, DatasetVersion,
AnalysisDefinitionVersion, AnalysisMethodVersion, MetricDefinitionVersion,
AnalysisRun/input/output, Evidence, FindingVersion, ModelVersion, evaluation,
validation, replication, and transfer-lineage IDs where those records expose
them. Missing exact references remain visible as unavailable or unverified;
another version is never selected as a substitute.

Project-scoped Research, Experiment, Analysis, and Model records are filtered
to the selected project. The Transfer workspace remains global, but only
explicit transfers involving the selected project and their related origin,
eligibility, adaptation, lineage, and downstream-use records are included.
When an artifact is known to belong to another project, the navigator labels
that boundary and offers an explicit project switch when the project is
available in Research's project list.

FrameworkVersion adoption is displayed as a reference to a workspace-shared
version. It does not imply ownership transfer. SourceClaims remain
source-attributed evidence, not scientific conclusions. No automatic
assignment, cross-project reuse, or Knowledge Transfer is introduced.

The existing Experiment `targetModelVersionId` is an optional recorded ID,
not proof that the target exists or that the project is authorized to use it.
The navigator resolves it only against available typed workspace data and
keeps unresolved IDs explicit.

## Navigation and asynchronous behavior

`ProjectLifecycleNavigator` provides cross-domain connections and links to
stable anchors in the existing panels. Clicking an anchor opens containing
`details` elements so the exact record can be reviewed. Shared Source
references continue to use Source Browser exact-ID query parameters.

`useProjectLifecycleWorkbench` loads Analysis, Experiment, Model, and the
global Transfer workspace through their existing typed services. Each domain
reports its own loading/error state. `LatestRequestGuard` prevents an older
request from publishing after refresh, project change, cleanup, or React
Strict Mode effect reactivation. A failed workspace is reported as an error;
unavailable references are not treated as successful resolution.

The Stage 3.2 Provenance close race is corrected: closing the panel advances
its request identity, so a pending result cannot reopen or repopulate it. A
subsequent intentional open creates a fresh request identity.

These are UI publication guards only. They do not cancel IPC calls, roll back
backend writes, provide authenticated authorization, or enforce project
membership.

## Tests and limitations

- `projectLifecycleModel.test.mjs` verifies project filtering, exact dataset
  and evidence provenance, missing ModelVersion non-substitution, shared
  FrameworkVersion adoption, and distinct cross-project Transfer origin and
  adaptation identities.
- `researchAsyncGuards.test.mjs` exercises the production guard classes,
  including close-while-loading invalidation, out-of-order refreshes, and
  lifecycle cleanup/reactivation. Its pending request cases manually
  simulate publication.
- React components and effects are not mounted by these tests. Browser event,
  scrolling, and mounted effect integration remain unverified.
- No workspace database was accessed and no backend mutation, migration, or
  schema change was made.
- Stage 3.4 is outside this scope.
