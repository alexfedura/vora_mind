# Phase 8.3 - Acquisition / Mastery Integration

**Status:** IMPLEMENTED
**Target Scope:** learning, source, tauri, frontend hooks/services

## 1. Intent

Verify and preserve the existing Acquisition / Mastery lifecycle from the
acquired and published FrameworkVersion through the Learning project,
objective, assessment, attempt, evaluated evidence, competency, mastery rule,
and independent reproduction domains.

This slice does not create a new learning engine or relationship. It integrates
only existing domain entities and operations.

## 2. Existing boundaries

- Acquisition owns Source, KnowledgeItem, and FrameworkVersion records.
- A published FrameworkVersion is the exact immutable version selected by a
  LearningProject.
- Learning owns Learner, LearningProject, ObjectiveVersion,
  AssessmentVersion, Attempt, Competency, MasteryRuleVersion,
  ReproductionTask/Submission/Evaluation, and CompetencyRecord.
- A LearningProject is independent from the Research or application project
  ownership model. It is not silently assigned to a selected app project.
- Mastery is evidence-based only. Content acquisition, a view, a planned
  activity, an inconclusive assessment, or a reproduction without evaluated
  evidence does not imply mastery.

## 3. Implementation

- Use the existing `learning_get_workspace` IPC and typed
  `learningService.getWorkspace` service.
- Guard Learning workspace loads and post-mutation refreshes with
  `LatestRequestGuard`, preventing stale loads, failures, and loading-state
  publication after unmount.
- Keep all mutations in the existing typed frontend service and backend
  repository; do not add frontend persistence or inferred progress.
- Add focused service and repository tests for exact version pinning,
  persistence/reopen, cross-project mismatch, unavailable evidence, append-only
  corrections, inconclusive outcomes, and independent reproduction evidence.

## 4. Non-goals

- No app-project foreign key or LearningProject ownership migration.
- No changes to Existing-Knowledge lifecycle semantics.
- No authorization or multi-user isolation claims.
- No first-principles discovery behavior.
- No active workspace database access.

## 5. Verification

- Repository/persistence tests use isolated in-memory or temporary SQLite.
- Frontend service tests verify command names and backend error propagation.
- Hook/state tests verify stale-response and unmount publication guards.
- Frontend type-check/build and Rust format/check suites run for touched code.
