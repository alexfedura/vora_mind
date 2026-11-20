# Phase 8.4 - First-Principles Discovery Integration

**Status:** IMPLEMENTED
**Target Scope:** research, tauri, frontend hooks/services

## 1. Intent

Integrate the existing first-principles Discovery workflow through the
project-context read path and preserve the repository's real Research entities:
phenomena, observation plans, observations, dataset references, patterns,
ideas, typed hypotheses, and experiment design versions.

## 2. Existing domain boundaries

- `ResearchWorkspaceDto` remains the workspace-wide snapshot.
- `research_get_project_context` is a project-filtered read projection.
- Phenomena, plans, observations, patterns, ideas, hypotheses, and designs
  remain project-owned where that project is explicitly recorded.
- Dataset references and source claims preserve exact IDs and versions.
- An experiment design is design-only; creating or linking it does not execute
  an experiment or validate a hypothesis.
- Hypothesis evidence is typed supports/contradicts/qualifies evidence. It does
  not establish scientific validity.
- The repository does not implement a separate reasoning-step entity.

## 3. Implemented integration

- Added Discovery collections to `ResearchProjectContextDto`.
- The Rust project-context projection filters each collection by the exact
  selected project ID.
- `ResearchWorkspacePage` uses the project-context Discovery collections for
  plans, observations, datasets, patterns, ideas, hypotheses, and designs.
- Existing project switching and `ProjectOperationGuard` protections remain
  unchanged.
- Existing mutations continue through typed `researchService` methods,
  `research_*` Tauri commands, IPC handlers, and Research repositories.

## 4. Unsupported transitions

No separate problem entity or reasoning-step lineage is introduced because the
current domain has no such implementation. Questions remain the explicit
problem/question identity. Contradictory, inconclusive, failed, and unresolved
outcomes are retained only where existing Experiment, Analysis, and Model
contracts support them.

## 5. Verification scope

Tests cover project-context DTO scoping and exact frontend service commands.
Repository and IPC tests use synthetic isolated SQLite state. No active
workspace database is opened.
