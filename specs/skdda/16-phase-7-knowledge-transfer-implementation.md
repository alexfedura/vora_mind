# Phase 7 — Knowledge Transfer and Lineage Implementation

## Status and scope

This specification implements the cross-project transfer contracts in the
authoritative Vora scientific architecture and the Phase 0 domain, relationship,
and lifecycle specifications. It covers controlled eligibility, exact-version
adoption, adaptation, provenance traversal, restrictions, and workspace backup
compatibility. It does not start general consolidation.

The user selected bounded typed transfer relationships and directed that legacy
artifacts with unknown ownership, license, or access permission remain
ineligible until those facts are explicitly recorded. This avoids implementing
the shared registry described by ADR-001 before its recorded implementation
approval.

## Transferable artifact allowlist

Phase 7 supports exact versions of:

- Research finding versions.
- Concept versions.
- Principle versions.
- Published framework versions.
- Released model versions.

The persisted artifact kind is a closed Rust/SQLite/TypeScript enum. Each
supported kind has a concrete foreign key; arbitrary `(kind, id)` references
are not accepted. Adding another kind requires an explicit domain decision,
schema FK, Rust enum/validation, TypeScript contract, UI presentation, and
tests. Source claims are not in this allowlist because existing claims have no
project ownership and source license/access metadata is not normalized.

Concept and principle identity records remain intact. New immutable,
project-attributed versions provide exact transfer pins. Existing concepts and
principles are not automatically assigned an origin or backfilled into a
version history.

## Origin and eligibility

An immutable eligibility record identifies one exact artifact version, its
origin project, declared license, access scope, permitted use, attribution,
permission basis, and reviewer/time. Eligibility is not inferred from source
text, creator names, project titles, or the presence of a published version.

Eligibility is unavailable when any required ownership, license, access, or
permission field is unknown. A legacy artifact whose schema has no origin
field requires a separately reviewed origin assertion; the assertion records
the missing legacy fact and does not rewrite the artifact. Every transfer pins
the eligibility record used at acceptance.

Eligibility state changes are append-only events (`eligible`, `restricted`,
`withdrawn`). They affect future adoption only. Restoring eligibility creates
a new eligibility record, and a new adoption creates a new transfer ID that
may explicitly reassert an earlier transfer. Existing adoptions and lineage
remain intact.

The app enforces declared license/access rules and required explicit permission
within the local workspace. Actor IDs are audit metadata, not authenticated
authorization; this local workflow does not claim to verify external legal
rights or provide multi-user access control.

## Adoption, adaptation, and lineage

An accepted transfer pins one eligibility record, one origin project, one
receiving project, intended use, attribution, context, actor, and timestamps.
Acceptance revalidates the current eligibility state, recipient scope, and
intended use inside the same transaction. Rejection and withdrawal append
events; they do not delete the request, adopted version, or historical use.

An adaptation links the accepted transfer to a distinct, exact artifact version
owned/created in the receiving project. It records rationale, attribution,
upstream transfer, and time. The adaptation relation is a typed provenance
edge; it does not change the originating version or imply scientific
validation. Downstream usage records likewise pin typed artifact versions so
forward and reverse traversal can be queried.

The existing framework-adoption API and rows remain readable. New transfer
adoptions are recorded in the transfer tables and exposed alongside legacy
adoptions; a legacy adoption is not silently assigned a transfer, origin, or
license. Existing adoption uniqueness/history is preserved.

## Persistence and migration

Transfer tables use restrictive foreign keys, one-of typed target columns,
append-only event records, and transactionally enforced project/type
consistency. Published/released source versions cannot be changed through
transfer operations. No migration infers legacy ownership or rights. Backup
and restore use the existing complete SQLite database path, so transfer and
lineage tables must survive a full database backup/reopen unchanged.

## Acceptance tests

Tests must prove:

1. A Project A artifact version is eligible only after explicit, complete
   ownership/license/access/permission declarations.
2. Unknown metadata, an out-of-scope recipient, a disallowed intended use, an
   unreviewed/unreleased source artifact, and dangling/wrong-kind IDs are
   rejected by backend/database validation.
3. Project B adopts the exact Project A version without changing the source
   identity/version or ownership.
4. Attribution, context, reviewer, and timestamps survive restart/backup.
5. Project B adaptations use distinct exact versions and retain the upstream
   transfer; forward and reverse lineage and downstream usage are queryable.
6. Contradictory/contested source findings remain unchanged and visible.
7. Restriction/withdrawal blocks future use but retains accepted and historical
   transfer/adaptation records; a later regrant/re-adoption uses new IDs.
8. Existing source and framework-adoption data remain accessible after
   migration.
