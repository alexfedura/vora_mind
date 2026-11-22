# VORA Phase 3.9 Security and Data Risks

## Critical

- Legacy product data may exist outside the clean Go database. Destructive migrations or legacy database retirement could lose data.
- The 333 Rust commands include broad domain operations; only direct frontend invokes were mapped, not every indirect path.

## High

- Project ownership is not universal across legacy or future scientific entities.
- MCP fixed permissions and caller identity require continued review.
- Approval parameter substitution is not verified across every approval-required operation.
- Restore and backup manifest semantics differ between Rust and Go implementations.

## Medium

- 284 Rust commands require indirect/runtime caller review.
- HTTP filesystem parity is incomplete.
- Independent third-party MCP acceptance is blocked.
- Windows symlink privilege testing is incomplete.

## Low

- Wails official documentation retrieval was blocked in the environment; repository evidence remains authoritative for this phase.

## Data controls

- Read-only database inspection only.
- Additive migrations only.
- Preserve legacy tables and foreign keys.
- No secret output in capability results or logs.
- Every migrated side effect requires project, permission, approval, and audit evidence.
