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
Zustand workflow; the three authoritative cross-domain workflows have not been
executed as complete end-to-end scenarios; and a full Rust library run has six
reproducible failures outside this pass's changed source files.

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

`cargo test --manifest-path src-tauri/Cargo.toml --lib -j 1` ran 219 tests:
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
