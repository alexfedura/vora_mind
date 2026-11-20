# Phase 8.4.1 - Discovery Repository and IPC Acceptance

**Status:** IMPLEMENTED
**Target Scope:** research, tauri, frontend services

**Acceptance:** ACCEPT — PHASE 8.4.1 RUST VALIDATION

## 1. Intent

Complete mock-runtime IPC coverage for the expanded Discovery project-context
DTO and add isolated repository coverage for the supported lifecycle:

`Research Question -> Phenomenon -> Reviewed Observation Plan -> Observation -> Pattern -> Typed Hypothesis`

## 2. Verified relationships

- A Research Question belongs to an active project.
- A Phenomenon belongs to the exact project.
- An Observation Plan belongs to an exact Phenomenon and project.
- A plan review/preregistration produces a real plan-version lifecycle state.
- An Observation pins the exact project, Phenomenon, plan, plan version, and
  optional dataset-reference ID.
- A Pattern pins exact project Observation IDs and optional dataset IDs.
- A Typed Hypothesis pins an accepted exact project Research Question.
- A Hypothesis can attach exact Observation or Pattern evidence; that evidence
  does not itself establish validation or scientific conclusion.
- A Linked Experiment Design pins the exact project and Hypothesis definition
  version but remains design-only.

## 3. Project isolation

Project-context RPC responses filter all Discovery collections by project ID.
Other-project and unassigned Discovery records do not enter the response.
Observation creation rejects project mismatches. Typed Hypothesis creation
requires the exact accepted question to belong to the requested project.

## 4. Persistence

The repository lifecycle fixture uses a temporary file-backed SQLite database,
closes and reopens it, and verifies exact IDs and relationships. The IPC fixture
uses the existing Tauri mock runtime and the real
`research_get_project_context` command.

## 5. Unsupported relationships

The current model does not support a direct Phenomenon or Pattern foreign key
inside typed HypothesisDefinition. The supported relationship chain is instead:
Question -> typed Hypothesis, Observation -> Pattern, and typed evidence links
that point to exact Observation or Pattern IDs. No reasoning-step entity or
automatic validation result is introduced.

## 6. Validation status

Phase 8.4.2 completed the previously blocked Rust validation. The bundled
`libduckdb-sys` build was exceptionally slow, not stuck: its MSVC compiler
continued producing DuckDB C++ object files and ultimately completed without a
native compiler error. The machine had about 8 GiB of visible RAM, very little
free memory during compilation, and approximately 1.2 GiB free on the workspace
drive. The existing Cargo target cache on `C:\vora-compile` was retained.

The compiled Windows test executable initially could not start directly and
returned `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`). Running a copy with the
Common Controls v6 activation manifest, the workaround already documented for
Phase 8.2, allowed the actual test binary to run without modifying the
workspace executable or Windows configuration.

The first real executions exposed two Phase 8.4.1 fixture defects, not
environment failures: the repository assertion compared the original Draft
plan DTO after review instead of the reviewed DTO; the IPC dataset reference
omitted all identity fields rejected by its existing schema constraint. The
assertion now checks the reviewed result, and the synthetic IPC fixture now
uses a synthetic URI. No production lifecycle behavior or dependency
configuration was changed.

### Results

- Repository lifecycle test
  `research::repository::tests::first_principles_repository_lifecycle_preserves_ids_and_isolates_projects`:
  **passed (1 passed, 0 failed)** against its temporary file-backed SQLite
  fixture, including exact typed observation-evidence persistence across
  close/reopen and project isolation.
- Real Tauri mock-runtime IPC test
  `research::ipc::tests::discovery_project_context_crosses_ipc_with_exact_scoped_collections`:
  **passed (1 passed, 0 failed)**. The actual command and DTO were exercised;
  exact IDs/provenance and typed observation evidence, collection serialization,
  project isolation, and the missing-project error contract were checked.
- Related Research repository tests: **24 passed, 0 failed**.
- Related Research IPC tests: **2 passed, 0 failed**.
- Full Rust library suite: **288 passed, 0 failed, 0 ignored**.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: **passed**.
- `cargo check --manifest-path src-tauri/Cargo.toml -j 1`: **passed**.
  It emitted two pre-existing unused/dead-code warnings in
  `src/mind/migration.rs`.
- `pnpm tsc --noEmit`: **passed**.
- Focused frontend Node tests: **33 passed, 0 failed**.
- `pnpm build`: **passed**; existing mixed `markdown-it` import and large
  chunk warnings remain.
- `git -c core.whitespace=cr-at-eol diff --check`: **passed**.

All requested Rust tests and checks were executed; none are being represented
as skipped. The external test-executable manifest workaround was needed only
to launch Windows tests and did not alter source, dependencies, or OS
configuration.
