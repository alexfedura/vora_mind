# Vora Current State Baseline

## Repository baseline

- Repository: `alexfedura/vora_mind`
- Working directory: `D:\workspace\vora_mind`
- Branch: `main`
- Current head: `fc61617` (`feat: enhance repository model with additional functionality`)
- Status at verification: worktree clean at the start of inspection; no user DB changes were modified during the audit.

## Architectural baseline

The inspected implementation is a local-first Tauri + React + Rust + SQLite desktop application. The runtime already includes:

- desktop shell and Tauri IPC registration in `src-tauri/src/lib.rs`
- React/TypeScript pages and feature workspaces under `src/`
- persisted SQLite schema modules for Sources, Research, Experiment, Analysis, Model, Learning, and Knowledge Transfer
- separate worker queue and market/Parquet paths
- board, library, snapshots, versioning, and trace subsystems

The repository does not yet provide a single, fully verified scientific research graph with universal project ownership and source provenance. The strongest existing evidence is in the domain modules and the task-specific SQL schemas, not in a fully test-verified end-to-end scientific workflow.

## Current-state implementation matrix

| Finding ID | Status | Relevant files | Existing implementation | Missing behavior | Dependencies | Data migration | Required tests | Acceptance criteria |
|---|---|---|---|---|---|---|---|---|
| VORA-001 | IMPLEMENTED_BUT_UNVERIFIED | `src-tauri/src/lib.rs`, `src-tauri/src/research/*`, `src-tauri/src/model/*`, `src-tauri/src/experiment/*`, `src-tauri/src/analysis/*` | Domain modules exist with Rust schema+IPC and typed DTOs | Cross-domain workflow coverage, end-to-end validation, and full project ownership enforcement | Research, source, experiment, analysis, model | Low to moderate | Rust integration tests; project-scoped workspace loads | Workspaces load only authorized project data and all domain entities can be linked through a valid project path |
| VORA-002 | IMPLEMENTED_BUT_NOT_FULLY_VERIFIED | `src-tauri/src/model/repository.rs`, `src-tauri/src/model/schema.sql`, `src-tauri/src/research/schema.sql` | Model domain includes identity, versions, evaluations, validations, replications, and project-bound lifecycle tracking | Global model catalog and project-specific visibility are not consistently enforced; some workspace queries remain broader than project-scoped data | Research project lifecycle, model version lineage | Moderate | Project-isolation regression tests, reopened-workspace state tests | A project only sees models and versions created in or tied to its project context |
| VORA-003 | PARTIALLY_IMPLEMENTED | `src-tauri/src/source/*`, `src-tauri/src/source/acquisition.rs`, `src-tauri/src/source/acquisition/repository.rs` | Source catalog, segments, claims, revisions and acquisition helpers exist | No universal source-to-project linking, immutable source version ledger, or explicit citation lineage for all scientific entities | Research projects, acquisition metadata, source segments | Moderate | Source linking/unlinking tests, version preservation tests | Shared sources remain global and immutable while project usage is explicit and auditable |
| VORA-004 | IMPLEMENTED_BUT_NOT_INTEGRATED | `src-tauri/src/mind/questions.rs`, `src-tauri/src/mind/hypothesis.rs`, `src/shared/layout/HypothesisDetail.tsx` | Inquiry triage and hypothesis promotion are persisted and linked to UI flows | Not integrated with first-class experiment design, dataset, analysis, evidence, or finding domain objects | Research project and experiment schemas | Moderate | Promotion and lifecycle tests | Hypothesis promotion preserves lineage and project ownership without silently turning a suggestion into a finding |
| VORA-005 | IMPLEMENTED_BUT_NOT_INTEGRATED | `src-tauri/src/experiment/*`, `src-tauri/src/analysis/*`, `src-tauri/src/model/*` | Experiment and analysis modules are present with dataset and run constructs | End-to-end scientific lifecycle not fully connected across design-run-analysis-evidence-finding-model transitions | Research, model, source data | Moderate | Workflow A/B/C tests | A scientific workflow can be executed from design through evidence and model validation |
| VORA-006 | BACKEND_ONLY | `src-tauri/src/knowledge_transfer/*` | Transfer domain models and project-based knowledge adoption logic are defined | Frontend workflow and UX integration remains incomplete | Research project registry, model/domain graphs | Low | Knowledge transfer and authorization tests | Authorized cross-project transfer records remain explicit and auditable |
| VORA-007 | FRONTEND_ONLY | `src/shared/layout/*`, `src/pages/*`, `src/mind/services/*` | UI panels and workspace screens exist for research, sources, models, investigations, and knowledge transfer | Some user flows rely on non-validated or partially connected backend state | Backend domain contracts | Low | UI workflow tests and Tauri IPC smoke tests | UI actions map to a validated backend operation and maintain project ownership |
| VORA-008 | NOT_IMPLEMENTED | `docs/architecture/` (new), legacy docs under `docs/audit/` | There are audit and design docs, but no single co-managed architecture and acceptance plan in the repo | Required architecture docs and migration/project ownership specs are missing or scattered | Repository-wide documentation governance | Low | Documentation diff audit and review | The architecture documentation matches current implementation and explicitly states unresolved gaps |
| VORA-009 | BLOCKED_BY_ARCHITECTURAL_DECISION | `src-tauri/src/mind/portable_db.rs`, `src-tauri/src/vora_trace.rs`, `src-tauri/src/market/*` | Board, market, worker, and Trace subsystems exist and keep their own logic | They are not yet explicitly mapped to research project ownership or scientific lifecycle semantics | Product/UX decisions on whether to treat them as workspace-global or project-linked | Low | subsystem boundary tests | Boundary remains explicit: workspace-global systems are not treated as project-scoped research records |

## Current known issue addressed in this pass

The project-scoped model workspace was returning all models and all model evidence regardless of the selected project, while project-targeted evaluations, validations, and replications were already project-filtered. The fix narrows the workspace to the relevant model identities and their versions/evidence for the given project, while preserving the global historical data model used elsewhere.
