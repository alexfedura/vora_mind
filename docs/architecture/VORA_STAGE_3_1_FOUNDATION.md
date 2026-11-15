# VORA Stage 3.1 — Project Knowledge Loop Foundation

## Baseline and inspected implementation

The pre-change checkout was clean on `main` at
`8c566118433c8425e7f2d783859a56f26e3e81ee`. The package, Tauri, and Cargo
versions were all `0.24.2`.

The inspection covered:

- Frontend workspaces: `src/shared/layout/ResearchWorkspacePage.tsx`,
  `AnalysisWorkspacePanel.tsx`, `ExperimentWorkspacePanel.tsx`,
  `ModelWorkspacePanel.tsx`, and `projectSourcesState.ts`.
- Frontend service contracts: `src/mind/services/researchService.ts`,
  `analysisService.ts`, `experimentService.ts`, `modelService.ts`, and
  `knowledgeTransferService.ts`; `src/hooks/useResearchWorkspace.ts`;
  `src/types/index.ts`.
- Backend ownership and operations: `src-tauri/src/research/repository.rs`,
  `analysis/repository.rs`, `experiment/repository.rs`, `model/repository.rs`,
  `source/acquisition.rs`, `source/acquisition/repository.rs`, and
  `knowledge_transfer/repository.rs`.
- Contracts and registration: `src-tauri/src/research/mod.rs`,
  `experiment/mod.rs`, `model/mod.rs`, `knowledge_transfer/mod.rs`,
  `source/acquisition_ipc.rs`, `research/ipc.rs`, `analysis/ipc.rs`,
  `experiment/ipc.rs`, `model/ipc.rs`, `knowledge_transfer/ipc.rs`, domain
  schemas, `src-tauri/schema_acquisition.sql`, and `src-tauri/src/lib.rs`.
- Existing state tests: `tests/projectSourcesState.test.mjs`; existing
  architecture references under `docs/architecture/`.

The Research page's main Research workspace is loaded globally rather than by
project ID. Its project-Source association loader already uses cancellation,
request IDs, and project-keyed reducer state. It was not converted to a
project-workspace loader. Its project-specific panel subtree is now keyed to
the selected project; project-specific selections are cleared on switch, and
late Research responses cannot set the new project's plan/dataset selection.

## Architecture contract established

`VORA_PROJECT_KNOWLEDGE_LOOP.md` records entity ownership, identifiers,
versioning and immutability, allowed references, explicit/user-mediated edges,
and prohibited inferences for the Source → Research → Framework → reuse loop.
It distinguishes shared Source identity from project-specific context; capture
metadata from local asset preservation; extraction review from truth;
SourceClaim attribution from scientific conclusions; Research design records
from typed Experiment specification/preregistration; Research dataset
references from sealed Experiment DatasetVersions; Framework publication from
ownership/adoption/transfer eligibility; and Research Projects from Learning
Projects.

No new entity, persistence relationship, or ownership rule was introduced.

## Race conditions found and addressed

| Workspace | Previous behavior | Change |
|---|---|---|
| Analysis | Project loads wrote workspace/error/loading state without request identity. A prior load, failure, or finalizer could affect a later project; form resets followed the awaited mutation. | Loads and mutations now use project/request identity. A failed load clears workspace and renders an error instead of empty-success counts. Creation forms reset only after a current-project mutation response. |
| Experiment | Refresh and `act` had no request identity. Dataset sample, file-picker, form, and selection state could be updated by an obsolete operation. | Shared controller gates load/mutation publication; the file picker checks current-project identity; stale action results do not set sample state or reset forms. The workspace reports loading/saving state and exposes no empty-success workspace on load failure. |
| Model | A `Promise.all` refresh could publish results/errors or busy finalizers after a project switch. Create handlers changed selected model/version and reset forms inside the pending action. | Model, Analysis, and Experiment responses are published as one project-scoped snapshot; the create handlers apply selection/form changes only for a current successful response. |
| Research page project-local UI | Global Research data is not a project-scoped workspace request. Some delayed mutation completions could reset a form or publish a failure after the user changed projects. | Project and Question-project contexts use separate request identities. Domain/adoption, Research record, Question/Hypothesis, plan/dataset, observation, and Source-association completion/error updates are guarded before changing project-local UI. Successful backend operation sequences remain intact. |
| Global Research workspace refresh | Overlapping workspace refreshes could publish out of order, and completion after unmount could update hook state. | `LatestRequestGuard` lets only the newest active refresh publish workspace, loading, and error state; cleanup invalidates outstanding requests. A stale failure is suppressed. |

`src/shared/layout/projectWorkspaceState.ts` is the production external-store
controller used by `useProjectWorkspace`. It rejects responses from prior
project generations, rejects older refreshes for the same project, tracks
loading/error/mutation state, and exposes mutation results as `completed`,
`failed`, or `stale`. Hook cleanup deactivates the project. This prevents
obsolete UI publication; it does not cancel an IPC operation already sent to
the backend or roll back a write that legitimately completed in its original
project.

## Test strategy and results

`tests/projectWorkspaceState.test.mjs` uses deferred promises and synthetic
workspace records. It asserts the production snapshot consumed by
`useSyncExternalStore`, including workspace contents, status, error, and
mutation state after a project switch, out-of-order refresh, failed request,
late create/update, and cleanup. Existing project-Source reducer tests were
run alongside it.

Validation:

| Command | Result |
|---|---|
| `node --experimental-strip-types --test tests/projectWorkspaceState.test.mjs tests/projectSourcesState.test.mjs` | Passed: 11 tests, 0 failures (7 new controller cases, including separate create and update cases; 4 existing Source-state cases). |
| `pnpm tsc --noEmit` | Passed. |
| `git -c core.whitespace=cr-at-eol diff --check` | Passed. The repository files use CRLF; plain `git diff --check` treated added CRLF endings as trailing whitespace, so the check was rerun with Git's `cr-at-eol` setting. |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml` | Passed. It emitted existing migration-module unused-variable/dead-code warnings; no Rust source or schema files changed. |
| Rust formatting/tests | Not run; no Rust source or schema files changed. |

No React DOM test framework, DOM emulator, or React test renderer is installed.
The tests therefore exercise the production workspace snapshot directly, not a
mounted browser DOM. They validate the state that drives the three panels'
loading, error, and workspace rendering branches; visual layout and browser
event behavior remain untested.

## Changed files

- `docs/architecture/VORA_PROJECT_KNOWLEDGE_LOOP.md`
- `docs/architecture/VORA_STAGE_3_1_FOUNDATION.md`
- `src/hooks/useProjectWorkspace.ts`
- `src/shared/layout/projectWorkspaceState.ts`
- `src/shared/layout/AnalysisWorkspacePanel.tsx`
- `src/shared/layout/ExperimentWorkspacePanel.tsx`
- `src/shared/layout/ModelWorkspacePanel.tsx`
- `src/shared/layout/ResearchWorkspacePage.tsx`
- `tests/projectWorkspaceState.test.mjs`
- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

## Existing documentation needing a later narrow refresh

No older document was rewritten in this stage. These statements predate the
current implementation and should be revised in a separate documentation
change:

- `VORA_IMPLEMENTATION_PLAN.md`, “Current-state baseline,” says
  “shared sources do not yet have explicit project links in all relevant
  paths.” Stage 2.4 now has explicit Research project–Source association
  records; revise this bullet to describe the actual supported association and
  its remaining scope.
- `VORA_CURRENT_STATE_BASELINE.md` records HEAD `fc61617` and its VORA-003 row
  says “No universal source-to-project linking, immutable source version
  ledger.” The repository now has immutable SourceVersion rows and explicit,
  project-scoped ResearchSource associations; update the baseline and narrow
  the remaining gap to the exact paths not covered.
- The same baseline's VORA-008 row says architecture documentation is missing.
  The Stage 3.1 contract and foundation documents now exist; update that row's
  status and identify any remaining documentation gaps rather than treating
  the old baseline as current.

## Known limitations

- No React-mounted DOM or end-to-end project-switch test was available. The
  focused tests assert the production snapshot and not browser-rendered markup.
- Request identity prevents stale UI updates but does not cancel Tauri IPC or
  undo a successful write to the project where it began.
- The Research workspace API remains workspace-wide; this stage does not
  convert it to project-scoped transport.
- Refresh ordering prevents stale publication, but does not cancel obsolete
  service requests. The global Research page error still reports failures from
  the workspace-global refresh itself.
- Stage 2 migration-startup, legacy-profile, external-writer/WAL, and
  platform-specific durability limitations remain unchanged.
- The trusted-local application has no authenticated principal or
  project-membership authorization boundary; actor IDs remain audit metadata.

## Stage 3.1.2 corrective audit

The Stage 3.1.2 implementation guarded delayed domain creation/assignment and
Framework adoption forms; Research record create/reset flows; Question-project
creation and refinement; plan/dataset selection; observation, pattern, idea,
hypothesis, evidence, model-reference, and design forms; gap exception review;
and Source association success/error updates. Main-project and Question-project
contexts were tracked separately, and context generations rejected A→B→A stale
completions. The initial implementation did not invalidate mutation tokens on
page unmount and did not key legacy Question/Hypothesis assignment errors to
their submitted destination; Stage 3.1.4 closes those two gaps.

The Research workspace DTO, source catalog, and workspace-refresh error are
global. Project-bound action errors and local form/selection updates are
project-scoped. Project creation remains a global operation whose follow-up
selection is guarded by the originating selected-project context. Stage 3.1.4
also uses a workspace lifecycle token for global project-transition actions,
without treating those actions as project-selection changes. Backend calls are
not cancelled or rolled back by these UI guards.

`useResearchWorkspace` uses the production `LatestRequestGuard` for latest-wins
refresh publication and unmount cleanup. Synthetic state tests exercise that
guard and the project-operation guard directly. They are state-level tests;
the repository still has no mounted React DOM test environment, so rendered
form and browser-event behavior is not covered.

Stage 3.1.2 validation:

| Command | Result |
|---|---|
| `node --experimental-strip-types --test tests/researchAsyncGuards.test.mjs tests/projectWorkspaceState.test.mjs tests/projectSourcesState.test.mjs` | Passed: 18 tests, 0 failures. The 7 new cases cover stale/current and A→B→A project mutations, stale errors, latest global refresh ordering, older failure after newer success, and cleanup. |
| `pnpm tsc --noEmit` | Passed. |
| `git -c core.whitespace=cr-at-eol diff --check` | Passed for the final diff. |

## Stage 3.1.4 mutation lifecycle and assignment identity correction

`ProjectOperationGuard` captures a lifecycle generation in every token.
`ResearchWorkspacePage` activates the guard in an effect and invalidates it in
effect cleanup. Invalidation is idempotent and rejects all outstanding
mutation tokens. React Strict Mode's setup/cleanup/setup cycle reactivates the
guard with a new generation; tokens from before cleanup remain stale. Project
selection changes continue to advance only their own context generations.

Question and Hypothesis assignments use distinct record-specific guard
contexts. Each handler captures the exact destination project ID submitted to
the unchanged service call. Changing the destination select updates that
context, so a delayed result for the former destination cannot publish an
error. An unchanged destination still permits its result/error. Existing
Question-project and main-project identities remain separate. Actions that
previously had no project context use a workspace lifecycle token so their
late errors are suppressed after unmount. These guards do not cancel, roll
back, or authorize backend writes.

Focused synthetic tests exercise the production guard and assignment-context
mechanism. They cover success and failure after invalidation, invalidation of
multiple tokens, idempotency, reactivation, Question and Hypothesis
destination changes, and a failure with an unchanged destination. Tests remain
state-level; no mounted React DOM test environment is available, so actual
component effect and browser-event execution are not covered.

## Stage 3.1.6 createProject lifecycle error guard

The `createProject` handler now passes the currentness predicate for its
captured operation token to `useResearchWorkspace.run`. A failed request can
publish the page error only while the token remains current; success retains
the existing guarded project-selection and form-reset behavior. Project
creation remains global and the backend operation is unchanged.

The focused regression tests exercise `ProjectOperationGuard` and manually
simulate the hook's conditional error publication and the success commit.
They do not execute the `createProject` React callsite or mount the component,
so wiring and browser/effect behavior are not component-level tested.

## Stage 3.2 prerequisites

Before a Project Knowledge Workbench is built:

1. Approve a feature specification and keep persistence changes out of the
   workbench proposal until the exact entity relationships are agreed.
2. Decide the user flow for selecting/reviewing exact SourceVersions,
   SourceLocations, extractions, and SourceClaims.
3. Preserve explicit project association context and exact citation pins; test
   two-project isolation and stale responses with synthetic data.
4. Decide whether local preservation of captured Source bytes is required;
   current hash/URI metadata is not proof of preservation.
5. Add a mounted UI test strategy if browser-rendered visibility and event
   behavior become acceptance requirements.

## Scope and repository-state confirmation

No schema, migration, database data, dependency, or lockfile changed. The three
version declarations were advanced from `0.24.2` to `0.24.3` as a process
exception to the Stage 3.1 scope. The implementation baseline was
`8c566118433c8425e7f2d783859a56f26e3e81ee`; the release commit and publication
status must be reported from the final Git result. No active workspace database
was opened, initialized, migrated, or modified.
