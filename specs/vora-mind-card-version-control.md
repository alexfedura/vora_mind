# Vora Mind Card Version Control

> **Status**: Approved for implementation
> **Scope**: `canvas | db | store`

## Goal

Provide explicit immutable versions for individual cards, with a reason, history
list, content diff, and restore action. Routine card autosaves remain working
state and do not create user-visible history entries.

## Data and persistence

- Reuse `object_versions` for explicitly committed card history. Current
  working content is stored on the owning `objects` row separately.
- Parse three-part semantic version tags with an optional leading `v` into the
  existing `version_major`, `version_minor`, and `version_patch` columns.
- Explicit commits insert a new version row with a reason and timestamp.
- `save_card_direct` updates working content without inserting an
  `object_versions` row.
- Restore copies the selected immutable version into working content without
  creating another history entry.
- Card version reads are scoped by card ID. Writes and restores verify that
  the object is a card and belongs to the supplied board where applicable.

## UI

- Clicking the card version badge opens a commit dialog with a suggested patch
  increment and a required change reason.
- A card with no committed history defaults to `v0.1.0`; its first commit
  allows a custom semantic version tag. Its initial reason is fixed to
  `Initial Version (Baseline)`.
- Routine working-content saves do not create or count as committed history.
- Later commits suggest a patch increment from the card's current version and
  allow the user to enter a reason.
- The backend validates semantic version tags and enforces the baseline reason
  so clients cannot bypass the initial-version rule.
- The card history action shows an indicator until the first explicit version
  is committed.
- The History action opens a per-card history modal showing version, date, and
  reason.
- The modal compares the selected historical content against the current
  working content or another historical version using an additions/deletions
  diff view.
- Restoring a historical version updates the working card and any open document
  editor while preserving the immutable version rows.

## Validation

- Validate semantic version parsing, command registration, and typed frontend
  command payloads.
- Run `pnpm tsc --noEmit` and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
