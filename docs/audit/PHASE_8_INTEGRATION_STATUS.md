# Phase 8 Integration and Production-Hardening Status

**Verified:** 2026-11-12  
**Status:** Partial hardening completed; Phase 8 acceptance is **not met**.  
**Scope:** Repository evidence and isolated tests only. No installed workspace or user database was opened.

## Executive summary

This pass kept the Phase 1–7 domain persistence owners intact. It fixed direct
Tauri command calls from visual components, moved file-URL conversion behind
the frontend service boundary, removed a stale Phase 2 execution disclaimer,
and added a fresh-workspace schema/integrity regression test.

The request path and core database are not yet certified end to end. In
particular, Research, Analysis, Model, and Learning IPC modules call repository
functions directly rather than a distinct domain-service module; several
domain panels manage data locally rather than through the expected shared
Zustand workflow; and the three authoritative cross-domain workflows have not
been executed as complete end-to-end scenarios. The six Rust failures recorded
by the initial pass were resolved in Phase 8.2.4; the final suite result is
recorded below.

## Verified architecture and persistence

- Tauri setup opens the active workspace database and calls
  `mind::portable_db::initialize_workspace_schema`. That coordinator initializes
  Canvas, legacy workflow, Market, Source, Research, Experiment, Analysis,
  Model, Learning, Knowledge Transfer, Forge, and Vora schemas in a fixed order.
- Source, Research, Experiment, Analysis, Model, Learning, and Knowledge
  Transfer retain domain-owned schema/migration code. The scientific domains
  do not share one central migration ledger. This pass did not consolidate or
  rewrite those migration owners.
- Typed Tauri command registrations for Research, Experiment, Analysis, Model,
  Learning, and Knowledge Transfer are present in `src-tauri/src/lib.rs`.
- Dedicated frontend services exist for those domain APIs. The panels are
  composed into the Research workspace page; the source UI remains a separate
  workflow.
- Text search found no SQL statements in React `.tsx` files and no
  `@tauri-apps/api/core` import or `invoke` call remaining in `.tsx`/`.jsx`
  files. Board generation and updater commands now go through typed services;
  media asset URL conversion also goes through a service.
- The new combined-workspace test initializes the fresh schema twice, verifies
  `PRAGMA foreign_keys = 1`, and checks that both `PRAGMA integrity_check` and
  `PRAGMA foreign_key_check` are clean.

### Request-path limitations

- `research/ipc.rs`, `analysis/ipc.rs`, `model/ipc.rs`, and `learning/ipc.rs`
  call repository/domain functions directly. IPC handlers do not contain the
  scientific validation inspected in this pass, but these paths do not have a
  separate service module between IPC and repository as required by the
  requested target architecture.
- New domain workspace panels use component-local React state and call their
  typed frontend services; a shared Zustand workflow layer was not verified
  for those panels. This is not a UI-only persistence claim: the corresponding
  Rust APIs and SQLite tables exist, but the specified Zustand step is not
  consistently present.
- Typed DTOs and IPC registrations are implemented, but no generated,
  exhaustive command-contract parity test between Rust and TypeScript was
  run in this pass.

## Historical integrity and domain coverage

The full Rust library run exercised focused tests for:

- source-version/citation pinning, extraction review/correction history, and
  provenance reopen;
- research gap evidence, legacy question/hypothesis preservation, observation
  versus pattern distinction, and question/hypothesis lineage;
- experiment version pinning, dataset asset/sample/split validation, and
  failed-run persistence;
- analysis temporal leakage/report validation and migration/reopen provenance;
- model version/evaluation/validation/replication persistence;
- assessment-attempt correction history, version-pinned learning projects, and
  independent reproduction/mastery evidence;
- transfer rights, attribution, adaptation, lineage, and backup/restore.

These are focused library-level tests, not proofs that each named entity's
entire immutable lifecycle has been covered. Not every historical entity in
the Phase 8 prompt has a cross-domain mutation/reopen assertion in this pass.

## Workflow acceptance

| Workflow | Status | Evidence and gap |
|---|---|---|
| A. Acquisition and Mastery | **Partial** | Source and framework publication/correction behavior is tested; learning version pinning, evaluated attempts, and independent reproduction/mastery are tested. The entire chain from reviewed Extract through SourceClaim and FrameworkVersion into an AssessmentAttempt and Competency was not run as one integrated workflow. |
| B. Existing-Knowledge Investigation | **Partial** | Research, experiment, analysis, evidence/finding, and model/validation/replication behaviors have focused tests. No single Project → adoption → gap → question → hypothesis → run → dataset → analysis → finding → model → validation/replication workflow was executed. |
| C. First-Principles Discovery | **Not verified end to end** | Distinct observation/pattern records, typed competing hypotheses, preregistration, analysis, findings, and transfer have domain-level coverage. The complete prospective workflow, including independent validation/replication and contradictory cases, was not executed as one fixture. |

Failed, inconclusive, and contradictory records are supported by domain
contracts and representative tests, but their complete cross-domain workflows
were not certified here.

## Database reliability, migration, and assets

### Executed

- Fresh combined-workspace schema initialization twice on an isolated
  in-memory database.
- SQLite integrity and foreign-key checks on that database.
- Existing focused module migration/reopen and backup/restore tests as part of
  the library suite.
- `cargo check --manifest-path src-tauri/Cargo.toml -j 1`.
- The focused Knowledge Transfer Project A → Project B backup/restore test
  passed in the full library run.

### Not established

- Every deployed historical schema variant, partially initialized database,
  or migration interruption boundary.
- Full legacy-database matrix convergence and application recovery from a
  process interruption.
- Integrity of any installed/user database.
- External media, Trace, Parquet, or other file-backed asset availability
  after a database-only backup/restore.
- Large-dataset throughput, pagination behavior at scale, long-running work
  responsiveness, SQLite lock duration, or workspace-switch recovery.

The `.vora` backup is a SQLite database backup; companion settings and
file-backed assets have separate storage/recovery behavior. A database backup
alone is not evidence that external assets are available.

## Frontend verification and operations

- `pnpm tsc --noEmit`: passed.
- `pnpm build`: passed. Vite reported the existing `markdown-it` mixed
  static/dynamic import warning and large chunks; the largest emitted JS chunk
  was approximately 3.49 MB (about 1.04 MB gzip).
- `package.json` has no frontend test-runner script; no component, browser,
  accessibility, keyboard-navigation, or visual end-to-end suite was run.
- No performance or memory benchmark was run for the Phase 8 changes.

## Rust test results

The initial `cargo test --manifest-path src-tauri/Cargo.toml --lib -j 1` run
executed 219 tests:
**213 passed, 6 failed**. The six failures were rerun individually and remained
reproducible:

1. `market::boxes::tests::dbd_confirmed_fixed_levels`: observed end timestamp
   22, expected 25.
2. `market::boxes::tests::dbd_pending_no_engulf`: unexpected engulf timestamp.
3. `market::boxes::tests::rbr_confirmed_fixed_levels`: observed end timestamp
   22, expected 25.
4. `market::boxes::tests::rbr_pending_no_engulf`: unexpected engulf timestamp.
5. `market::db::tests::range_query_orders_by_ts`: observed first timestamp 1300,
   expected 1000.
6. `mind::canvas_db::board_version_tests::forking_a_board_version_creates_an_independent_board_snapshot`: observed one source version, expected two.

These are historical baseline results; Phase 8.2.4 below records their
root-cause repairs and the succeeding full-suite result.

The failing test source files were not changed during this pass. Their causes
were not investigated or fixed because they are outside the scoped integration
boundary changes; they remain release-blocking evidence against claiming a
green full suite.

The following checks passed:

- Focused
  `mind::portable_db::tests::full_workspace_schema_is_idempotent_and_integrity_checks_pass`.
- All Phase 1–7 domain tests reached during the full library run.
- Rust `cargo check`.
- Frontend type-check and production build.

Rust integration-test targets under `src-tauri/tests/` were not run. No claim
is made that IPC, desktop startup, or full application workflows passed.

## Permission, security, and local-first findings

- Knowledge-transfer eligibility enforces the locally recorded license
  identifier, permission basis, recipient, purpose, attribution, and lifecycle
  declarations. Reviewer/actor identifiers are audit metadata, not
  authenticated identities; the application does not establish that an
  external license declaration is legally valid.
- No authorization, threat-model, or IPC security audit was performed in this
  pass.
- This pass added no cloud dependency. Existing optional DeepSeek board
  generation and release-updater network behavior remain outside the
  scientific persistence domains.

## Required follow-up before Phase 8 acceptance

1. Resolve the six reproducible Rust library failures and rerun the full suite.
2. Add and execute end-to-end fixtures for workflows A, B, and C, including
   negative, failed, inconclusive, and contradictory cases.
3. Decide and implement the service/store boundaries needed for the specified
   IPC → domain-service → repository and view → workflow/store → service paths.
4. Test supported legacy migration fixtures, interruption/rollback behavior,
   backup/restore integrity, and external asset manifests.
5. Run representative desktop/UI accessibility and responsiveness checks,
   plus large-dataset and lock-duration measurements.
6. Update this report only when those checks have actual evidence; do not
   remove the listed limitations based on UI presence or inferred behavior.

## Phase 8.1 — Existing-Knowledge Investigation fixture (2026-11-16)

The Phase 8.1 slice adds a synthetic, in-memory SQLite integration test in
`src-tauri/src/research/repository.rs`:
`existing_knowledge_investigation_preserves_project_and_source_lineage`.
It exercises production Source creation/acquisition helpers, Research
operations, and repository retrieval without opening the active workspace
database. The fixture verifies the supported Existing-Knowledge Investigation
path: exact SourceVersion-pinned reviewed literature evidence, a
project-scoped Source association distinct from that citation, an
evidence-backed ResearchGap, and a ResearchQuestion linked to the accepted
gap. It also checks project-mismatch rejection, rejection of an unavailable
exact version without partial review creation, preservation of legacy
unassigned Questions/Hypotheses, and atomic rollback when a Question's
provenance-event write fails.

The fixture passed in the full library run. This is database-backed
service/repository coverage against synthetic in-memory persistence; it does
not invoke Tauri IPC handlers or mount React components, and does not verify
browser lifecycle behavior, production migrations, backend authorization, or
workflow-wide end-to-end behavior. Workspace Research retrieval remains
workspace-global; the fixture checks persisted project ownership and the
project-filtered Source-association query, not IPC transport isolation. No
backend operation cancellation or rollback beyond the tested transactional
Question write is implied.

Validation on the final test code:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib -j 1`: 284 tests;
  278 passed and 6 failed. The new fixture passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `git -c core.whitespace=cr-at-eol diff --check`: passed.
- Frontend type-check/build and Node tests were not run for this Rust-only
  slice.

The six failures are reproducible and remain unresolved; they are outside
this Research fixture and no unrelated behavior or assertions were changed:

1. `market::boxes::tests::dbd_confirmed_fixed_levels` and
   `market::boxes::tests::rbr_confirmed_fixed_levels` expect the engulf/end
   timestamp at index 25, while the current scanners select index 22 as the
   first confirming high/low and set that candle as the end.
2. `market::boxes::tests::dbd_pending_no_engulf` and
   `market::boxes::tests::rbr_pending_no_engulf` expect no engulf, while the
   current scanners emit a box with the same index-22 confirmation.
3. `market::db::tests::range_query_orders_by_ts` expects the oldest two rows
   for a limited range; `range_candles` deliberately selects the latest
   limited rows in descending order and reverses them for chronological
   output, yielding 1300 rather than the expected 1000 as the first row.
4. `mind::canvas_db::board_version_tests::forking_a_board_version_creates_an_independent_board_snapshot`
   expects two source-board versions after creating one explicit version;
   the current creation path has no initial version, so the source history
   contains one.

The first four box failures and the range-history mismatch appear to be
outdated assertions relative to current implementation behavior; the
board-history mismatch is likewise an expectation/count discrepancy. This
slice does not determine whether those independent product behaviors should
change. They remain full-suite failures and must be resolved separately
before claiming a clean Phase 8 library suite.

## Phase 8.2 - Existing-Knowledge Research-context IPC integration

The Research workspace now has a project-context request path:
`ResearchWorkspacePage` -> `useResearchProjectContext` ->
`researchService.getProjectContext` -> `research_get_project_context` ->
`research::project_context`. The backend returns a filtered context containing
the selected project, its assigned domains and Source associations, its
literature reviews and prior assessments, its ResearchGaps and project-owned
ResearchQuestions, and the exact SourceVersion references used by its
citations. Unassigned Questions remain in the global Research workspace and
are not assigned as a side effect of loading project context.

Shared Sources and their project associations remain distinct from exact
SourceVersion citations. Citation writes reject unavailable SourceVersions,
locations, and claims instead of selecting substitutes; the project-context
response preserves those exact IDs. SourceClaims remain attributed evidence,
not scientific conclusions. The Research UI displays the scoped records and
their evidence relationships, keeps unavailable exact citations explicit,
and does not infer a source association from a citation or vice versa.

The frontend service checks that the returned project identity matches the
requested project. The hook uses the existing `ProjectWorkspaceController`
for selected-project, latest-request, and cleanup invalidation. The controller
tests exercise late Project A -> Project B responses and exact provenance
preservation; these are state/controller tests, not mounted React or browser
effect tests. The Tauri mock-IPC fixture uses a fresh in-memory SQLite
workspace and synthetic Source provenance; it does not open the active
workspace database.

The projection currently filters the existing full Research workspace snapshot
in Rust; query-level pagination and large-workspace performance were not
implemented or benchmarked.

The Tauri `test` feature is enabled only for the development dependency so the
mock-runtime IPC boundary can be exercised. No crate was added and
`Cargo.lock` was not changed.

These protections prevent stale UI publication; they do not cancel or roll
back backend operations and do not provide authorization or multi-user
isolation. No mounted React, browser, desktop, production-database, or
multi-user verification is claimed for this slice.

Phase 8.2 validation in this implementation turn:

- `pnpm tsc --noEmit` passed.
- `pnpm build` passed. Vite reported the existing `markdown-it` mixed
  static/dynamic import warning and large-chunk warnings; the largest emitted
  JavaScript bundle was about 3.59 MB (1.06 MB gzip).
- The focused Node state/controller suites passed: 34 tests. The Research
  service boundary suite passed: 2 tests; Node reported that module mocking is
  experimental.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and
  `cargo check --manifest-path src-tauri/Cargo.toml -j 1` passed. Cargo check
  reported existing warnings in `mind::migration`.
- The focused Rust Tauri IPC test compiled, but the Windows test process could
  not start and exited with `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`) before
  running the test. Its IPC assertions were not counted as passed at that
  point. Later execution used the established copied-binary and external
  Common Controls v6 manifest workaround; see Phase 8.2.5.
- The CRLF-aware `git diff --check` passed. The full Rust library suite was not
  rerun at that point; the six historical failures above remained unresolved
  until the Phase 8.2.4 repair.

## Phase 8.2.4 — Rust regression repair

The six earlier failures were corrected without weakening the domain
assertions:

- Market box detection now follows the four-pivot pattern window, confirms
  engulfing after the fourth pivot, and emits base-only geometry. The fixtures
  were corrected to represent the intended candle patterns.
- The Market range test now expects the most recent limited rows in
  chronological output, matching the query's existing range contract.
- Board-fork history assertions now reflect that a fork creates an independent
  first version for the child without fabricating another source-board
  version; snapshot independence remains asserted.

The full library run after those repairs executed 285 tests: **285 passed, 0
failed, 0 ignored**. Formatting, Rust `cargo check`, and CRLF-aware diff checks
also passed.

## Phase 8.2.5 — Existing-Knowledge acceptance

### Verified call path and persisted behavior

The selected-project read path is
`ResearchWorkspacePage` → `useResearchProjectContext` →
`useProjectWorkspace` / `ProjectWorkspaceController` →
`researchService.getProjectContext` → `research_get_project_context` →
`project_context` and the Research repository. The frontend checks that the
returned project ID matches the request. Existing request-generation guards
prevent late Project A loads or failures from replacing Project B's displayed
context.

The isolated Tauri mock-IPC workflow now uses a uniquely named temporary
file-backed SQLite database. It creates two projects and synthetic shared
Sources, associates a Source with Project A, adds a reviewed citation pinned
to the exact SourceVersion, SourceLocation, and SourceClaim, creates and
reviews a prior-knowledge assessment, attaches reviewed evidence to a gap,
accepts the evidence-backed gap, and creates a Question linked to that gap.
It verifies that Project B does not receive Project A's records, that
unassigned legacy records remain unassigned, and that exact provenance and
project relationships survive closing and reopening the same database.

Negative IPC cases reject an unavailable or mismatched SourceVersion,
unavailable location/claim/evidence, cross-project evidence and Question-gap
links, missing projects, and writes to an archived project. Sources are
workspace-shared rather than project-owned; an association is a separate
many-to-many project context record, so a Source's association with another
project is not itself an ownership violation.

### Partial writes and transaction boundaries

The UI's “Save as reviewed assessment” action performs two backend operations:
create a draft assessment, then review it. Those operations are not one
transaction. The IPC fixture injects a failure into the second operation and
confirms the first remains a persisted Draft. The page now reloads the
selected project's context on that failure so the saved draft is visible while
the original error remains surfaced.

Reviewed literature creation inserts the review and exact citation in one
repository transaction. An injected provenance-event failure after both
inserts is rejected; after reload, neither the review nor citation exists.
The existing repository fixture also verifies rollback of the paired legacy
and Research Question records when its provenance-event write fails.

### Verification

- All repository Node test files: **44 passed, 0 failed** using Node's
  experimental module-mocking flag. No component-testing library or DOM
  implementation is installed, so no mounted React or browser workflow is
  claimed.
- `pnpm tsc --noEmit`: passed.
- `pnpm build`: passed. Vite emitted the existing mixed static/dynamic
  `markdown-it` import warning and large-chunk warnings; the largest JavaScript
  bundle was about 3.59 MB (1.06 MB gzip).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --no-run -j 1`:
  passed and produced
  `C:\vora-compile\cargo-target\debug\deps\vora_lib-01d7468d1d423429.exe`.
- The focused Existing-Knowledge IPC and repository fixtures passed. All
  Research module tests executed: **24 passed, 0 failed, 0 ignored**.
- The full Rust library suite executed: **285 passed, 0 failed, 0 ignored**.
  The copied test executable was run with the previously established external
  Common Controls v6 activation manifest; no executable or Windows system
  configuration was modified.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and
  `cargo check --manifest-path src-tauri/Cargo.toml -j 1` passed. `cargo check`
  reported the existing unused/dead-code warnings in `mind::migration`.
- `git -c core.whitespace=cr-at-eol diff --check`: passed.

The temporary test database, copied executable, manifest, and their sidecars
were removed. No active workspace database was opened. This evidence covers
frontend service/controller tests and a Rust mock-runtime IPC workflow; it
does not certify a mounted React/browser flow, a desktop runtime, or a
production database.

## Phase 8.3 — Acquisition / Mastery integration

### Verified call chain

The existing Acquisition / Mastery path is:
`LearningWorkspacePanel` → `useLearningWorkspace` →
`learningService` → a typed `learning_*` Tauri command →
`learning::ipc` → `learning::repository` → the Learning SQLite schema.
Acquisition remains owned by the Source domain: reviewed Source extractions,
KnowledgeItems, and published FrameworkVersions are created through the Source
services and repositories. A LearningProject separately pins an exact published
FrameworkVersion; the Learning repository does not infer ownership from a
selected application project or create an implicit Source-to-mastery link.

The Learning workspace exposes learners, pinned projects, objective versions,
assessment versions and reviews, attempts and append-only evaluations,
competencies, versioned mastery rules, competency records, reproduction tasks,
submissions, and evaluated reproduction evidence. The frontend uses only the
typed service boundary and does not persist Learning state locally.

### Domain and lifecycle boundaries

LearningProjects are independent from application/ResearchProject ownership.
They preserve their exact learner, framework ID, framework version ID, and
project-scoped objective/assessment lineage. The repository rejects
cross-project attempts and unavailable exact assessment versions. It also
preserves immutable published/pending content and submitted-response history;
mastery is produced only by an explicit competency rule evaluation over
qualifying assessment and reproduction evidence.

An acquired FrameworkVersion, opened Source, or reproduction task alone does
not imply mastery. Inconclusive and failing evaluations remain history; they
do not become passing evidence. Mastery re-evaluation creates a new immutable
CompetencyRecord, while the prior record remains available.

### Frontend async behavior

The Learning panel now uses `useLearningWorkspace`, backed by
`LatestRequestGuard`. It suppresses stale or late load success/failure,
obsolete loading-state publication, and mutation follow-up refresh publication
after cleanup. These guards are UI-publication protections; they do not claim
backend cancellation or rollback. Mutation failures are shown through the
existing error banner while successful mutations trigger a guarded refresh.

The Learning domain remains workspace-global rather than falsely
project-scoped. The existing UI also has no app-project selector for
LearningProjects. This implementation intentionally does not add one or infer
application-project ownership.

### Tests and limitations

The existing Learning repository suite verifies exact framework pinning across
coexisting versions, reopen persistence, append-only attempt/evaluation
history, inconclusive evidence, independent reproduction, and immutable
CompetencyRecords. The Phase 8.3 repository fixture adds rejection of
cross-project assessment reuse and unavailable exact assessment IDs.

The Node Learning service fixture verifies exact pinned IDs and command names,
and confirms backend persistence/lifecycle failures are propagated. Existing
`LatestRequestGuard` Node coverage exercises out-of-order loads and cleanup
suppression. No mounted React, browser, desktop, production-database,
authorization, or Tauri IPC runtime fixture is claimed for this slice.

No Acquisition/Mastery schema migration was added. No active workspace
database was opened or modified.

## Phase 8.4 — First-Principles Discovery integration

### Verified call chain

The Discovery read path is:
`ResearchWorkspacePage` → `useResearchProjectContext` /
`useProjectWorkspace` → `researchService.getProjectContext` →
`research_get_project_context` → `research::project_context` → the
workspace-wide Research projection filtered to the exact project ID.

Discovery mutations retain the existing supported path:
`ResearchWorkspacePage` → `researchService` → a typed `research_*` Tauri
command → `research::ipc` → `research` repository operations → isolated
project-owned Research SQLite records.

### Implemented operations and boundaries

The project-context DTO now includes project-scoped phenomena, observation
plans, dataset references, observations, patterns, ideas, typed hypotheses,
and experiment design versions. The UI uses these scoped collections instead
of relying only on workspace-wide arrays for Discovery forms and displays.

Existing entities remain distinct: observations are measurements under a
reviewed/preregistered plan, patterns declare observation/dataset inputs,
ideas are unvalidated proposals with exactly one typed origin, hypotheses pin
accepted questions and typed operational definitions, and design versions are
design-only records. Source/claim exact IDs continue through their existing
evidence relationships.

There is no separate problem or reasoning-step entity in the current domain.
Explicit accepted ResearchQuestions are the supported problem/question
identity. No dedicated reasoning chain was invented.

### Evidence and negative boundaries

The backend rejects observations whose plan, phenomenon, plan version, or
dataset reference belongs to another project or is not reviewed/preregistered.
Patterns reject missing inputs and mismatched project observations/datasets.
Typed hypotheses require an accepted project-owned question and preserve
immutable operational variables and exact lineage. Hypothesis evidence requires
matching project context for observations/patterns and exact existing
SourceClaims for source evidence.

Contradictory hypothesis evidence, inconclusive observations, pattern
uncertainty, failed/contradictory downstream analysis or model validation,
and unresolved model references remain represented only in their existing
domain contracts. They are not promoted to validated conclusions by this slice.

### Frontend and tests

The Node Research service suite now verifies that the project-context response
preserves exact scoped Discovery IDs and that mismatched project responses are
rejected. Existing `LatestRequestGuard` and `ProjectOperationGuard` suites
cover stale project responses and cleanup.

Existing Rust repository tests cover distinct observation/pattern records,
typed hypothesis creation/preregistration, exact evidence, cross-project
rejection, append-only states, and database reopen. No active workspace
database was opened.

### Known limitations

No mounted React, browser, desktop, or fresh Rust mock-IPC run is claimed in
this slice. The project-context IPC fixture from Phase 8.2 remains evidence for
the existing project-context transport, while this Phase 8.4 change expands
the response with Discovery collections. No reasoning-step table, scientific
problem entity, execution engine, or validation conclusion was added.

## Phase 8.4.1 — Discovery repository and IPC acceptance

### Implemented repository lifecycle

The existing isolated repository test module now contains a temporary
file-backed SQLite fixture for:

`Question -> Phenomenon -> Reviewed/Preregistered Plan -> Observation -> Pattern -> Typed Hypothesis`

The fixture creates active Project A and Project B, an accepted A question,
a project-owned phenomenon, an exact phenomenon-to-plan reference, a
preregistered plan version, an exact plan/phenomenon Observation, a Pattern
with the exact Observation ID, and a Typed Hypothesis linked to the accepted
Question. It also adds the Pattern-origin Idea and exact Observation evidence
where supported. The fixture closes and reopens SQLite and verifies the exact
stored IDs and relationships.

Cross-project Observation and Typed Hypothesis writes are asserted to fail.
An unrelated project record is created and the reopened workspace is asserted
to contain no other-project Observation in the lifecycle projection.

### Implemented IPC coverage

The existing Tauri mock-runtime fixture now registers the real Discovery
creation and review commands and creates all project-context collections:
phenomena, observation plans and versions, dataset references, observations,
patterns, ideas, hypotheses, and linked design versions.

The `research_get_project_context` IPC response is deserialized as the real
`ResearchProjectContextDto`. It asserts exact IDs and provenance fields, the
preregistered plan state, exact observation/plan/phenomenon/dataset/Pattern
links, exact Hypothesis Question lineage, exact Design and project IDs, empty
unrelated-project collections, missing-project failure, cross-project
Observation rejection, and identity after database close/reopen.

### Frontend contract

`researchService.getProjectContext` now rejects null/malformed responses and
responses whose `projectId` differs from the requested project. Empty
Discovery collections are accepted as valid empty arrays and are not replaced
with inferred records.

### Model limitation

The current model has no direct Phenomenon or Pattern foreign key on a Typed
Hypothesis definition. The lifecycle therefore verifies the supported chain:
Question -> Typed Hypothesis; Plan -> Observation; Observation -> Pattern;
Observation/Pattern -> Hypothesis evidence. It does not claim that evidence
validates or answers the hypothesis.

### Phase 8.4.2 — Rust build recovery and validation

The previous `libduckdb-sys` delay was a long bundled native compilation, not a
source error or a stalled process. Cargo was using the existing
`C:\vora-compile\cargo-target` cache and the locked `duckdb 1.20000.0` /
`libduckdb-sys 1.20000.0` configuration with `bundled` and `parquet` features.
The installed MSVC compiler was available to the build. Although CMake and
MSVC were not on the interactive shell's PATH, Cargo's native build script
located and invoked MSVC directly. The compiler continuously produced DuckDB
object files and completed the initial test build in 57m50s without a native
compiler error. The host had about 8 GiB visible RAM, low available memory
during compilation, and about 1.2 GiB free on D:. No cache, target directory,
dependency version, feature flag, or lockfile was changed.

The first test launch then failed before test execution with Windows loader
status `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`). The already documented
Common Controls v6 activation-manifest workaround was applied to a temporary
copy of the test executable outside the repository. No workspace executable
or Windows configuration was modified.

Executing the tests exposed and resolved two fixture-only Phase 8.4.1 defects:
the repository assertion had checked the original Draft plan DTO rather than
the reviewed DTO, and the IPC synthetic dataset reference lacked every
identity field required by the existing schema constraint. The repository
assertion now checks the returned reviewed version, and the IPC fixture uses a
synthetic URI. No production lifecycle or dependency configuration changed.

**Validation results:**

- `research::repository::tests::first_principles_repository_lifecycle_preserves_ids_and_isolates_projects`:
  **1 passed**. Exact IDs and relationships survived file-backed SQLite
  close/reopen, including typed observation evidence; project isolation and
  cross-project rejection passed.
- `research::ipc::tests::discovery_project_context_crosses_ipc_with_exact_scoped_collections`:
  **1 passed** through the real Tauri mock-runtime command. The project-context
  DTO collections, exact IDs/provenance and typed observation evidence,
  Project A/B isolation, and missing-project error contract passed.
- Related Research repository tests: **24 passed, 0 failed**.
- Related Research IPC tests: **2 passed, 0 failed**.
- Full Rust library suite: **288 passed, 0 failed, 0 ignored**.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: **passed**.
- `cargo check --manifest-path src-tauri/Cargo.toml -j 1`: **passed** with
  existing unused/dead-code warnings in `mind::migration`.
- `pnpm tsc --noEmit`: **passed**.
- Required focused Node suites: **33 passed, 0 failed**.
- `pnpm build`: **passed**; existing mixed `markdown-it` import and large-chunk
  warnings remain.
- CRLF-aware `git diff --check`: **passed**.

All focused and full Rust tests requested for this validation were executed.
The compiler delay and test-loader workaround are resolved for this run; no
external installation or source-level production fix was needed.
