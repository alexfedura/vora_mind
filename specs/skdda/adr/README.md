# Vora Scientific Architecture ADRs

These records capture decisions selected for the Phase 0 architecture baseline. “Accepted” means selected as the technical design in this repository; **product/scientific-owner ratification and implementation approval remain outstanding**. No ADR here implements a schema or authorizes Phase 1.

| ADR | Decision | Status |
|---|---|---|
| [ADR-001](./ADR-001-typed-scientific-entity-registry.md) | Typed entity registry, concrete subtypes, validated scientific relations | Accepted for Phase 0 baseline |
| [ADR-002](./ADR-002-identity-versioning-and-epistemic-boundaries.md) | Opaque IDs, separate immutable versions, epistemic class boundaries, canvas boundary | Accepted for Phase 0 baseline |
| [ADR-003](./ADR-003-versioned-migration-and-connection-policy.md) | Ordered transactional migration chain, staged upgrades, per-connection FK enforcement | Accepted for Phase 0 baseline |
| [ADR-004](./ADR-004-time-numeric-and-asset-contracts.md) | UTC microseconds, decimal strings and units, immutable asset manifests | Accepted for Phase 0 baseline |
| [ADR-005](./ADR-005-rust-typescript-contract-organization.md) | Rust owner-domain contracts with canonical TypeScript contract location and fixtures | Accepted for Phase 0 baseline |

The normative consolidated contract is [09-phase-0-domain-contracts.md](../09-phase-0-domain-contracts.md). If an ADR and that contract disagree, do not implement from either until the conflict is resolved in a new ADR/revision.
