# Phase 8 — Full System Integration and Production Hardening

## Status and scope

This specification consolidates the implemented Phase 1–7 domains without
replacing their persistence owners, changing scientific semantics, or inferring
legacy ownership and provenance. It covers evidence-based integration audits,
focused reliability fixes, acceptance-test composition, and verified
documentation. It does not certify unexecuted user-workspace, UI, performance,
or migration scenarios.

The Phase 0 audits and domain contracts remain the compatibility authority:
[repository audit](01-repository-audit.md), [canonical domain contracts](04-canonical-domain-contracts.md),
[migration strategy](07-migration-strategy.md), and the Phase 1–7 contracts.
All audit findings in this document's eventual implementation report must cite
current code or test evidence, not infer completion from screens or type
definitions.

## Verified baseline at Phase 8 start

- `mind::portable_db::initialize_workspace_schema` initializes canvas,
  workflow, Market, Source, Research, Experiment, Analysis, Model, Learning,
  Knowledge Transfer, Forge, and Vora schemas in a fixed sequence.
- Source, Research, Experiment, Analysis, Model, Learning, and Knowledge
  Transfer retain domain-owned migration code; several domains maintain
  independent checksums/ledgers.
- Tauri registers typed IPC operations for Research, Experiment, Analysis,
  Model, Learning, and Knowledge Transfer in `lib.rs`.
- Dedicated frontend workspace panels exist for Research, Experiment,
  Analysis, Model, Learning, and Knowledge Transfer. Source has separate
  source views and service.
- The prior audit is explicitly historical and does not certify runtime
  behavior or deployed workspace integrity. Phase 8 must update current-state
  statements only after direct verification.

## Non-negotiable integration invariants

1. Retain the canonical path:
   React view → workflow/store → typed service → typed IPC → Rust service →
   repository/transaction → SQLite.
2. Keep SQL out of React and Tauri handlers; keep scientific transitions and
   validation in Rust domain services/repositories.
3. Keep canvas and Trace as navigation/visualization or their existing
   separate domains, not as the scientific source of truth.
4. Do not combine or replace independent migration ledgers in this phase.
   Any migration fix must be additive, checksummed, transactional where
   supported, and preserve the original database on failure.
5. Preserve unknown or unverified provenance as unknown. Report unresolved
   asset references and incomplete legacy mappings explicitly.
6. Do not claim production readiness, full workflow completion, migration
   compatibility, performance, or asset availability without the relevant
   executable evidence.

## Work plan

### A. Backend and frontend boundary audit

- Trace representative create/read/review/version operations through the full
  frontend-to-SQLite path for each domain.
- Inspect component imports and SQL-like statements; distinguish textual
  false positives from actual direct invocation or persistence.
- Verify registration coverage for the typed IPC surface and ownership
  boundaries between services/repositories.
- Record any missing paths or unsupported workflow links as limitations unless
  a narrow, safe fix is established.

### B. Database reliability

- Exercise fresh initialization, repeated initialization, domain migration
  checksum behavior, foreign-key configuration, backup/restore, and
  `integrity_check`/`foreign_key_check` on isolated temporary databases.
- Use sanitized legacy fixtures already present in tests; do not inspect or
  modify real user databases.
- Add focused tests for any reproduced defect. Do not claim all historical
  migration paths or interrupted process states unless each is directly
  tested.
- Verify external asset references separately from SQLite backup coverage.

### C. Historical integrity and acceptance workflows

- Inspect existing immutability and lifecycle constraints/tests for Framework,
  Model, Experiment Design, Dataset, Assessment, Attempt, Analysis, Evaluation,
  Correction, Retraction, and Transfer.
- Compose representative end-to-end paths only where typed APIs and test
  fixtures support the relationship. Preserve failed, inconclusive, and
  contradictory records.
- If an authoritative workflow edge is absent, document it as incomplete;
  do not synthesize scientific records or bridge domains with unvalidated JSON.

### D. Frontend and documentation

- Verify consistent navigation and error/loading/empty/validation handling by
  inspection and available frontend checks.
- Ensure detail panels expose persisted provenance already supplied by their
  DTOs; add no UI-only epistemic states.
- Update the architecture, migration/backup guidance, and audited current state
  with verified capabilities, test commands/results, known limitations, and
  unsupported scenarios.

## Verification gates

Run focused Rust tests first, then `cargo check`, `pnpm tsc --noEmit`, and
`pnpm build`. Run broader suites only when targeted checks are green and the
environment permits them. Report every attempted command and its actual
result. UI workflows, all supported legacy databases, large-dataset scaling,
and production filesystem assets remain unverified unless separately exercised
in a representative environment.

## Completion report contract

The report must separate:

- implemented versus verified capabilities;
- executed tests versus unexecuted tests;
- supported migration fixtures versus unverified legacy variants;
- database-resident records versus external assets;
- operational/security findings versus assumptions;
- remaining incomplete authoritative workflow links.

No phase-wide completion claim is permitted unless all requested workflow
edges and integrity conditions have executable evidence.
