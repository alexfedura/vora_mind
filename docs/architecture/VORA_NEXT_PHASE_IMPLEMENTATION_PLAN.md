# VORA Next Phase Implementation Plan

## Recommended next phase

**Phase 3.9 — Legacy Capability Inventory and Shared Go Contract Completion**

Do not start this phase without explicit approval.

## Objective

Convert the active Rust/Tauri product surface into a dispositioned capability inventory and implement the first shared Go contract set.

## Scope

- Inventory all frontend Tauri invokes and Rust command declarations by active caller.
- Classify every command as migrate, merge, preserve, or retire.
- Select the first high-value domains, likely project-aware sources and evidence.
- Define Go domain/repository/service contracts.
- Add Wails, HTTP, MCP, and workflow tests for the first migrated domains.

## Prerequisites

- Phase 3.8 rebaseline accepted.
- Database-preservation requirements confirmed.
- Product domains selected for migration first.
- Existing Rust behavior and schema contracts collected.

## Acceptance criteria

- Every in-scope Tauri command has a documented disposition.
- Each migrated capability has exactly one Go implementation.
- Wails, HTTP, MCP, and workflows delegate to it.
- Permission and project scope are enforced before side effects.
- Audit and execution history capture the real channel.
- Legacy behavior has contract tests.
- No legacy database, file, or domain is deleted.

## Risks

- Rust business rules may be embedded in UI-adjacent repositories.
- Legacy schemas may contain ambiguous ownership semantics.
- Scientific provenance contracts may require compatibility fields.

## Rollback

Keep legacy code and data untouched. Revert only the added Go contract and tests if the migration fails.
