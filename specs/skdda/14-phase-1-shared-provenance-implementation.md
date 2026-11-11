# Phase 1 — Shared Provenance and Knowledge Acquisition

## Scope

Expose and complete the existing knowledge-acquisition persistence foundation in
the desktop application. The work is limited to Sources, immutable SourceVersions,
version-specific locations, typed extractions, source claims, concepts, principles,
procedures, typed knowledge links, provenance history, and versioned frameworks.
Research, Experiment, and Learning entities and workflows remain out of scope.

## Compatibility rules

- Keep the legacy `sources`, `segments`, `extracts`, `links`, and
  `source_revisions` tables and their existing commands operational.
- Do not infer a SourceVersion, version-specific location, extraction kind,
  source claim, or scientific truth from legacy rows. Legacy mappings remain
  explicitly unknown.
- Use the already-installed additive acquisition migration. Do not edit its
  checksum-bearing SQL; any future schema change requires a new numbered
  migration and a supported fixture profile.
- Domain behavior stays in Rust acquisition services/repositories. IPC handlers
  only dispatch typed requests and return explicit errors.
- Machine-created and newly created extractions start unreviewed. Review confirms
  extraction quality only; it does not confirm the scientific truth of a claim.
- Corrections create a new extraction and retain the original with a
  supersession edge and provenance events.
- Published framework versions and their ordered memberships are immutable.
  Published edits start a new draft version with a mandatory change class,
  rationale, and affected-claim/assumption links.
- Conflicting claims remain separate and may be linked as contradicting.

## User workflows

The Sources workspace must allow a user to browse a source and its legacy
segments, create and inspect immutable source versions and citations, create and
review exact-version extractions, correct an extraction without overwriting it,
create source-attributed claims only from accepted source-claim extractions,
inspect claims and reusable concepts/principles/procedures, and create, edit,
review, compare, and publish framework versions.

Unreviewed extraction state and unpublished framework state must be visible from
persisted backend values. Provenance history and exact source/version/location
must remain queryable after restart.

## Verification

Rust repository tests cover source-version coexistence, exact location and
extraction lineage, unreviewed-by-default behavior, review gating for source
claims, correction/supersession retention, contradictory claim coexistence,
framework publication/review requirements, immutable publication, and new
version creation after publication. A legacy-schema fixture confirms original
source records remain readable and unchanged after the additive migration.

The frontend must type-check/build. Its source workflow must call typed IPC
wrappers and render persisted review/publication state and source provenance.
No Research, Experiment, or Learning tables or workflows are introduced.
