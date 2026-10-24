# Vora Mind Library Item Groups

> **Status**: Approved for implementation
> **Scope**: `canvas | library | db`

## Goal

Allow users to save a selection of canvas items as a reusable, version-labeled item group in the shared Library, then inspect and add a copy of that group to any Board.

## Confirmed behavior

- Users can select canvas nodes, including cards, text, shapes, and future node types, and save the selection as an item group only through `Ctrl+G`/`Ctrl+ل` or the node context menu. Selecting nodes alone never opens the group form.
- Right-clicking a node exposes an `Add to Library` action. If the clicked node is not selected, that action creates a group from that node; an existing selection is preserved when its selected node is right-clicked.
- A group contains a self-contained snapshot of selected nodes and the edges whose endpoints are both selected. Node positions are stored relative to the selection bounds.
- Saving a group opens a floating form with a required name and a manually editable version, defaulting to `0.1.0`.
- Groups are shared across all Boards.
- Library filters are `All`, `Cards`, `Item Groups`, and `Trash`, in that order. `All` includes active cards and groups; `Trash` includes trashed cards and groups.
- The Library has a secondary refinement filter, independent of the item-type filter, with `All`, `Today`, and `Tags` in that order. It defaults to `All`. `Today` narrows the selected item-type filter to items created on the current local calendar day. `Tags` exposes the distinct tags assigned to cards; selecting a tag narrows the current item-type filter to matching cards, and item groups do not match tag filters.
- The Library shows a group's name above one spatial preview of its complete contents, preserving node positions and internal connections, including rectangle, path, text, and card items. Persian titles and content use right-to-left layout.
- Users can drag an active group from its Library card to a Board to insert a copy at the drop location, or use the explicit insert action in the group preview. While dragging a card or group, a floating preview follows the pointer and shows the card's complete title and content or the group's complete schematic. Dragging alone never changes the Board; dropping inside the Board inserts a copy, and dropping elsewhere cancels. Clicking a group card still opens its preview. Trashed groups cannot be dragged; active groups can be moved to Trash or restored directly from their Library cards.
- Each group owns a hidden, persistent Board. Opening a group or one of its saved versions loads that exact schematic into the main canvas as an isolated draft; it never changes the active My Desk Board.
- Draft edits are never autosaved. Closing without committing discards them and restores the previous My Desk canvas, selection, and viewport.
- Committing edits requires a manually entered three-part numeric version. The new version becomes current on both the group and its owned Board, while every prior snapshot remains available for exact preview and editing as the base of another version.
- Card titles appear above their content in Library cards. A direct HTTP(S) image/video URL or Markdown image URL in the content is rendered as an in-place preview. Active cards and groups can be moved to Trash without opening their detail view.
- Inserting a group creates new node and card IDs, preserves internal connections, and leaves the library group and original items unchanged.
- Dragging a Library card onto its Board creates a new card and canvas node with independent IDs and a copy of the source title, content, and version; it never reuses or mutates the source card. The copied Board card retains its source identity and is not listed as a separate Library item.
- Groups can be moved to and restored from Trash.
- Groups, their hidden owned Boards, and immutable version snapshots are persisted in SQLite; group snapshots remain independent of subsequent canvas edits and ordinary Board deletion. Full and selective backups preserve version history.

## Data and persistence

- Add a global item-group table with stable ID, name, version, serialized snapshot, and timestamps, including a soft-delete timestamp.
- Keep snapshot serialization typed in Rust and TypeScript. Preserve node data, type, dimensions, position, style, and internal edges.
- Persist the source Library card identity in the copied card's existing object metadata so it stays excluded from the Library across reloads.
- Add typed Tauri commands and frontend service methods for listing, creating, trashing, and restoring groups.
- Validate nonempty names and a three-part numeric version label. Surface persistence and validation errors in the UI.
- Never persist a group only in frontend storage.

## Validation

- Frontend: `pnpm tsc --noEmit`, `pnpm build`.
- Backend: `cargo check --manifest-path src-tauri/Cargo.toml`.
- Rust tests cover group creation/listing, global visibility, snapshot persistence, and soft-delete/restore.
