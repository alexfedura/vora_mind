# ADR-005: Rust and TypeScript Scientific Contract Organization

- **Status:** Accepted for Phase 0 baseline; implementation/testing gate pending.
- **Date:** 2026-11-11
- **Decision owners:** Architecture proposal; named frontend/backend owners not recorded.
- **Scope:** Domain model ownership and Tauri DTO parity.

## Context

The existing frontend domain types are split between `src/types/index.ts`, `src/mind/types/index.ts`, and feature-local modules. Rust domain models live with source, Mind, Market, Forge, and Trace domains. The master architecture requires a canonical contract source, typed services, and Rust/TypeScript parity; no shared generation pipeline currently establishes this.

## Decision

- Rust owner-domain models and services are authoritative for validation, lifecycle and persistence.
- New scientific public TypeScript interfaces/enums are declared once in the existing approved `src/types/index.ts` location. Existing types are not bulk-refactored by this phase.
- Rust DTOs live with their canonical owner/domain contract and are re-exported to IPC; Tauri command DTOs do not duplicate domain structures or expose raw DB rows.
- Serde wire keys are `camelCase`; enum wire strings are explicit `snake_case`; stable response DTOs serialize absent `Option<T>` as `null`. Patch inputs are distinct types with documented omission semantics.
- New domain ID types are Rust newtypes serialized as strings and TypeScript branded string aliases. Exact decimals are string DTO values; system time uses safe integer microseconds.
- No code generator/runtime dependency is added now. Before each DTO is implemented, a shared JSON fixture must be tested by Rust Serde and TypeScript validation, including nullability, enum casing, IDs, numeric values and timestamps.

## Alternatives considered

- Duplicate interfaces in each feature/service/store: rejected because field/enum drift becomes likely.
- Treat frontend types as authority for scientific transitions: rejected because persistence invariants must be enforced by Rust/backend.
- Introduce code generation in Phase 0: deferred because the repository has no established generation pipeline and adding one is unnecessary for logical contract definition. Revisit after a small DTO parity test proves recurring maintenance cost.
- Use unchecked JSON casts or `any`: rejected by the repository's type-safety requirements.

## Consequences

Rust and TypeScript maintain mirrored public shapes but a shared fixture/test contract proves compatibility without a new dependency. Domain-specific types remain owner-scoped in Rust, and UI types import from one canonical location. Typecheck/build are still required for later code changes; this ADR adds no TypeScript source.
