# Vora Mind Unified Library Index

> **Status**: Approved for implementation
> **Scope**: `canvas | db | store | layout`

## Goal

Make the Library a searchable index over persisted canvas objects (cards,
shapes, text, and media) with current placement and review status, while
retaining Library item groups and the existing card workflows.

## Data and backend

- Add `objects.review_status`, defaulting to `active`; supported statuses are
  `active`, `completed`, and `not_needed`.
- Search current, non-deleted objects using SQLite text matching over the
  latest title and content. Current card working content takes precedence over
  committed versions.
- Return object identity, type, title, content, owning Board, placement in the
  latest checkpoint, and review status.
- By default, search only active objects. Allow callers to request one status
  or all statuses; reject unknown status values.
- Update review status through a validated command and refresh the current
  search results.
- Placement is determined by membership in the owning Board's latest
  checkpoint placements.

## Frontend behavior

- Expose typed service and Zustand actions for searching objects and updating
  review status.
- Debounce Library search by 300 ms and show unified search results for cards,
  shapes, text, and media.
- Show the type, title/content preview, placement state, and review controls.
  Completed and not-needed items are hidden by default and available using an
  archived/status filter.
- Hovering a placed item highlights and centers its canvas node temporarily.
  Selecting an item from another Board switches to its owning Board first;
  unplaced objects can switch Boards but are not otherwise selected.
- Preserve existing Library item-group, card editing, and trash workflows.

## Validation

- Test the migration on both fresh and pre-existing `objects` tables.
- Test searching every supported object type, default and explicit status
  filters, latest content selection, and latest-checkpoint placement.
- Run `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml`.
