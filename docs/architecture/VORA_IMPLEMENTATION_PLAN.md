# Vora Implementation Plan

## 1. Current-state baseline

The app already has meaningful domain modules, but the research and scientific lifecycle remains partially implemented. The major issues are:

- project-scoped workspace queries are not uniformly enforced
- shared sources do not yet have explicit project links in all relevant paths
- unassigned/legacy records require a safe assignment policy
- scientific workflows are present in pieces but are not fully connected and verified
- board, library, trace, market, and worker subsystems need explicit boundary rules

## 2. Target architecture

See `VORA_TARGET_ARCHITECTURE.md`.

## 3. Dependency graph

1. Project ownership and isolation
   - informs all later scientific entity creation and validation
2. Legacy data and migration safety
   - is prerequisite for any cross-project record linking
3. Source and provenance integration
   - enables references, evidence, and citations
4. Scientific workflow integration
   - connects design, run, analysis, evidence, and model lifecycle
5. Experiment and model integrity
   - locks behavior around reproducibility and lineage
6. Knowledge transfer
   - extends beyond project boundaries with auditable semantics
7. Remaining subsystem boundary enforcement
   - ensures workspace-global systems remain separate from project-owned records
8. Verification and hardening
   - completes the acceptance matrix and regression testing

## 4. Implementation stages

### Stage 1 — Project ownership and isolation

- backend query scoping
- mutation validation
- project relationship integrity
- archived-project protection
- model workspace filtering

Status: in progress. The project-scoped model workspace fix was applied in `src-tauri/src/model/repository.rs`.

### Stage 2 — Legacy data and migrations

- unassigned record management
- assignment workflow
- source/project association support
- migration coordination and backup validation

### Stage 3 — Source and provenance integration

- shared source semantics
- project-source links
- immutable version references
- extraction lineage and citation integrity

### Stage 4 — Scientific workflow integration

- discovery workflow
- existing-model understanding workflow
- model evolution workflow
- cross-domain entity linking

### Stage 5 — Experiment and model integrity

- reproducibility
- version pinning
- immutable history
- evaluation and validation consistency

### Stage 6 — Knowledge transfer

- explicit origin and receiving project validation
- lineage and adaptation tracking
- cross-project authorization checks

### Stage 7 — Remaining subsystem integration

- workspace-global vs project-linked classification for boards, library, Trace, Forge, worker, market data

### Stage 8 — Verification and hardening

- focused and broad Rust tests
- frontend type-check/build verification
- migration regression tests
- documentation review

## 5. Affected files

- `src-tauri/src/model/repository.rs`
- `src-tauri/src/model/schema.sql`
- `src-tauri/src/research/repository.rs`
- `src-tauri/src/research/schema.sql`
- `src-tauri/src/source/*`
- `src-tauri/src/analysis/*`
- `src-tauri/src/experiment/*`
- `src-tauri/src/knowledge_transfer/*`
- `docs/architecture/*`

## 6. Risks

- mixed legacy and typed scientific records may create ownership ambiguity
- existing board and trace subsystems may be mistaken for scientific projects
- migration ordering across domain-level migration tables could conflict if not coordinated
- source and evidence records may leak across projects if project association is only frontend-filtered

## 7. Acceptance criteria

- project-scoped operations reject mismatched entity ownership
- archived projects reject write and execution attempts
- shared sources remain global while project usage is explicit
- workflows A/B/C are testable end to end
- migrations are idempotent and fail safely without data loss

## 8. Rollback strategy

- keep all schema changes additive unless a destructive migration is explicitly approved
- preserve legacy tables and data until a verified remap is complete
- prefer reversible migration steps over broad table rewrites
- validate each stage with targeted tests before moving to the next stage
