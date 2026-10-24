# Vora Mind Boards and Independent Canvases

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | boards | db`

## Goal

Add real, persistent Boards to Vora Mind. Each Board owns an independent canvas and independent card content. Users can create, rename, delete, and switch Boards without losing canvas nodes, card content, shape data, or connections.

## User-confirmed scope

- Implement only the Boards and canvas capabilities from the supplied reference images.
- Do not add presentation, sharing, export, image, code, table, or other new insertion tools in this change.
- Every Board has its own cards and canvas; cards are not shared between Boards.
- Support Board creation, renaming, deletion, and switching.

## Behavior

- A Boards panel lists persisted Boards, identifies the active Board, and provides create, rename, and delete actions.
- When no Boards exist, show an empty-canvas prompt whether the Boards panel is open or closed. Selecting "Create New Board" opens the Boards panel and immediately focuses the new-Board name field; no Board is persisted until the user enters a name and saves it.
- While creating the first Board, hide the empty-canvas prompt for as long as the name field is open, including while the Boards panel is visible. Cancelling the name entry restores the prompt; successfully creating the first Board removes it by leaving the empty state.
- Selecting "Add New Card" activates a placement tool without creating a card; the next click on empty canvas space creates and selects the card with the cursor at the midpoint of its top border.
- Selected cards do not use a bright yellow border or glow; each card provides orange link handles at the midpoint of all four edges.
- The text-link picker opens in a centered, fixed-size modal attached to the document viewport, independent of canvas pan and zoom. Its result list scrolls inside the modal; clicks on the backdrop do not dismiss it, and users close it with the close control or Escape, or create a link with Confirm.
- A fresh database starts with no Board. The Boards panel lets the user create the first Board.
- Migration preserves existing data: legacy canvas records without a Board are assigned to `board-1`, and the migration creates that legacy Board only when such records exist.
- Creating a Board creates an empty independent canvas.
- Switching Boards saves pending changes to the current Board, then loads the selected Board's latest saved canvas and its card content.
- Cards, text nodes, shape nodes, placements, and connections remain isolated per Board. Existing card editing, canvas drawing, connecting, delete, undo/redo, and snapshot replay continue to work within the active Board.
- `Ctrl+A` / `Cmd+A` selects every node and edge on the active Board. The physical A key shortcut also works when a Persian keyboard layout reports `ش`; editable fields retain their normal text-selection behavior.
- Copy selected canvas nodes with `Ctrl+C` / `Cmd+C` (or `Ctrl+ز` / `Cmd+ز` on a Persian keyboard) and paste with `Ctrl+V` / `Cmd+V` (or `Ctrl+ر` / `Cmd+ر`). Pasted cards are independent cards, node IDs are unique, internal connections between copied nodes are retained, and pasted nodes are placed at the current mouse cursor.
- Cut selected canvas nodes with `Ctrl+X` / `Cmd+X` (or `Ctrl+ط` / `Cmd+ط` on a Persian keyboard). Cutting removes the selected nodes and their connected edges from the active canvas as one undoable action, while preserving them in the clipboard; pasting with the existing shortcuts recreates the nodes at the current mouse cursor and retains internal connections.
- The canvas uses the standard arrow cursor over empty space and a grab-hand cursor over draggable items. Dragging the left mouse button over empty canvas selects nodes intersecting the selection rectangle.
- Holding `Ctrl` (or `Cmd` on macOS) while dragging a selected node duplicates the selected nodes and their internal connections, then moves the copies while leaving the originals in place. Duplicated cards have independent card records.
- Paste remains available when the pointer is over the canvas and does not intercept text-entry shortcuts in editable controls.
- Canvas autosave and manual snapshots are associated with the active Board. Reopening Vora Mind restores the last saved canvas for the previously active Board.
- Deleting any Board, including `board-1` and the last Board, deletes its Board-owned cards, snapshots, and versions atomically. If the active or last Board is deleted, the canvas returns to its empty state; users can create a new Board at any time.
- With no Boards, the canvas is empty and does not accept Board-owned content until a Board is created. The My desk menu and Boards panel remain available.
- Board state and canvas state are persisted through typed frontend service calls and Tauri commands into SQLite.

## Data and migration

- Add a `mind_boards` table with stable ID, name, and timestamps.
- Add a `board_id` ownership column to cards/objects and snapshots, migrate existing rows to `board-1` without creating an unused default Board, and ensure all new canvas objects and snapshots are assigned to their Board.
- Scope card listing, card writes, snapshot listing, snapshot commits, and snapshot resolution to Board ownership.
- Preserve the existing snapshot JSON payload and backward compatibility for existing cards and snapshots.
- Do not add frontend-only persistence as a substitute for SQLite.

## Validation

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Rust tests cover legacy-data migration without data loss, fresh empty-database initialization, deletion of any Board including the last, per-Board snapshot/card isolation, and preservation of existing snapshot round-trip behavior.
