# ADR-001: Typed Scientific Entity Registry and Relation Integrity

- **Status:** Accepted for Phase 0 architecture baseline; product/scientific-owner ratification and implementation approval pending.
- **Date:** 2026-11-11
- **Decision owners:** Architecture proposal; named product, science, and database owners not recorded.
- **Scope:** Cross-domain scientific identity and relation enforcement in the existing workspace SQLite file.

## Context

Vora currently has distinct graphs and soft references: source links use `(target_kind, target_id)` strings without target FKs; hypothesis and canvas references are partly soft; Trace is file-backed; Forge has a code-specific graph. The target needs cross-domain typed provenance, version pinning, and queries across knowledge, learning, research, evidence, models, and transfer. SQLite cannot declare a foreign key from a polymorphic ID to one of many subtype tables.

## Decision

Use one `scientific_entities` identity table with `(entity_id, entity_type)` uniqueness and one concrete subtype table per canonical entity family. The registry stores identity, workspace, ownership scope/project and creation identity only; it is **not** a generic JSON entity table. Every subtype has a restrictive composite FK to its matching registry ID/type and must satisfy an exactly-one-subtype consistency check.

Use a single `scientific_relations` assertion store for cross-domain semantic links. Both endpoints have composite FKs to the registry; the relation's `(predicate, subject_type, object_type)` has an FK to a versioned, controlled `scientific_relation_rules` allowlist. Required endpoint version pins are checked against the endpoint identity through composite version FKs and defensive triggers. Rust domain operations apply the same validation for actionable errors.

Use concrete typed junctions for structural composition with stable cardinality, such as project-domain membership, framework-version composition, assessment-objective links, and run/analysis input slots. These are not an alternate unvalidated cross-domain graph.

Relation rows are immutable assertions. `relation_id` and canonical `assertion_key` are unique; exact retry is idempotent. Scope/version differences create distinct assertions. Multiple provenance items attach to the same assertion. Retraction/reinstatement is recorded as append-only lifecycle/provenance events.

## Alternatives considered

1. **One dedicated table for every typed predicate pair.** Strong concrete FKs and good local constraints; rejected as the only graph because the expanding cross-domain vocabulary creates many junction tables/migrations and makes common lineage traversal cumbersome.
2. **Raw generic `(entity_type, entity_id)` columns with service-only validation.** Rejected because endpoint existence is not enforceable in SQLite and per-connection FK omissions would still permit dangling science links.
3. **Unvalidated generic JSON object store.** Rejected because lifecycle/query fields, subtype identity, ownership, and FK behavior become implicit and cannot be consistently enforced.
4. **Uncontrolled hybrid.** Rejected; it lets teams choose incompatible semantics. The chosen rule is a registry-backed relation graph plus narrowly defined typed junctions for structural ownership/cardinality.

## Consequences

- Cross-domain endpoint existence is enforceable without separate database files or copying canonical records.
- Type-pair validity requires a maintained predicate catalog and triggers; Rust-only checks are insufficient as the sole integrity layer.
- Registry/subtype consistency, delete restrictions, version ownership, trigger behavior and relation traversal require dedicated migration and integration tests.
- Queries pay an extra join to resolve subtype data. Indexes on identity, endpoint, predicate, context and version pins are mandatory.
- Unknown legacy target strings remain outside the registry until a reviewed mapping proves identity/type; they are preserved unresolved.
- Existing source links remain readable during compatibility; no implicit backfill or reclassification is authorized.

## Revisit when

Measured query performance, endpoint cardinality, SQLite trigger limitations, or product scope demonstrates that a specific high-volume relation needs a dedicated materialized/typed table. Such an optimization must preserve the canonical registry/relation contract and have an ADR plus consistency tests.
