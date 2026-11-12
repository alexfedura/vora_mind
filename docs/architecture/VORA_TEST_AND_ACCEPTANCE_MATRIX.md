# Test and Acceptance Matrix

## Required verification categories

| Category | Status | Notes |
|---|---|---|
| frontend type check | planned | `pnpm tsc --noEmit` remains the baseline check |
| frontend build | planned | `pnpm build` remains the baseline build check |
| Rust compilation | planned | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust unit tests | partial | Existing domain tests exist; runtime verification is not yet complete |
| migration tests | planned | use disposable DB fixtures |
| IPC tests | planned | backend-side validation remains a must |
| workflow tests | planned | A/B/C integration verification required |
| archived project policy tests | planned | write and experiment execution must be rejected |
| project isolation tests | in progress | includes mismatched project/entity IDs |

## Minimum acceptance checks

1. project creation and lifecycle
2. project isolation and cross-project rejection
3. archived project write rejection
4. legacy unassigned record handling
5. source linking and unlinking
6. source version preservation
7. migration idempotency
8. migration failure recovery
9. experiment design and execution
10. dataset sealing and immutability
11. analysis reproducibility
12. evidence and finding lineage
13. model versioning and validation
14. knowledge transfer
15. contradictory evidence handling
16. failed/inconclusive experiment handling
17. workflow A/B/C end-to-end tests

## Current status note

The repo contains substantial module tests and integration tests, but a complete verification cycle has not yet been executed against the repository current baseline for all scientific lifecycle criteria. The model workspace regression fix is the starting point for the staged work.
