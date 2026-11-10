# Vora Mind Library Card Actions

## Goal

Restore drag-and-drop for searchable Library cards and provide direct card
editing, deletion, and owning-Board navigation from each card result.

## Behavior

- Pointer-down on a Library card starts the existing drag workflow from the
  card body, including its main button. Clicking status and overflow controls
  must not start a drag.
- Clicking the card body opens an edit dialog with title and content fields.
  Saving uses the existing card-content store action and synchronizes canvas
  node data.
- The card overflow menu provides edit, move-to-trash, and owning-Board
  navigation actions.
- Cards returned by Library search may belong to a Board whose card records
  are not currently in the Zustand cache. Load the card from its owning Board
  before invoking existing update or trash actions.
- Non-card Library objects retain their existing open/locate behavior.

## Validation

- Run `pnpm tsc --noEmit`.
- Verify Library card dragging still dispatches the existing canvas drop event.
- Verify edit, delete, and owning-Board actions are available from a card.
