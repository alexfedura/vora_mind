# Vora Mind Card Version Control

> **Status**: Approved for implementation
> **Scope**: `canvas | db | store`

## Goal

Provide explicit immutable versions for individual cards, with a reason, history
list, content diff, and restore action. Routine card autosaves remain working
state and do not create user-visible history entries.

## Data and persistence

- Reuse `object_versions`, which stores card content as JSON containing
  `title`, `content`, and `version`.
- Parse three-part semantic version tags with an optional leading `v` into the
  existing `version_major`, `version_minor`, and `version_patch` columns.
- Explicit commits insert a new version row with a reason and timestamp.
- Auto-save rows may continue to be updated in place by `save_card_direct`, but
  they are excluded from card history.
- Restore copies the selected immutable version into a new `object_versions`
  row and updates the owning object's `updated_at`. The `objects` table does
  not store card title or content.
- Card version reads are scoped by card ID. Writes and restores verify that
  the object is a card and belongs to the supplied board where applicable.

## UI

- Clicking the card version badge opens a commit dialog with a suggested patch
  increment and a required change reason.
- The History action opens a per-card history modal showing version, date, and
  reason.
- The modal compares the selected historical content against the current
  working content or another historical version using an additions/deletions
  diff view.
- Restoring a historical version updates the working card and any open document
  editor while preserving the previous version rows.

## Validation

- Validate semantic version parsing, command registration, and typed frontend
  command payloads.
- Run `pnpm tsc --noEmit` and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
