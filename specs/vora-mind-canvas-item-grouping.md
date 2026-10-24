# Vora Mind Canvas Item Grouping

> **Status**: APPROVED
> **Target Scope**: `canvas | types`

## Intent

Allow users to combine selected canvas items into a logical group, move and select
the group as one unit, and later ungroup it without changing its contents.

## Behavior

- The existing floating selection toolbar provides **Group** when multiple
  canvas nodes are selected and **Ungroup** when the selection contains grouped
  nodes. An active group also has an **Ungroup** action available beside its
  shared outline while its member nodes remain deselected.
- The selection outline stays positioned around the visible bounds of the
  selected items or active group.
- Grouping deselects every member and selects the group as a single unit, shown
  by one shared outline around all members. Clicking a group member selects the
  group without selecting its individual nodes.
- Dragging a group moves all members together while preserving their relative
  positions.
- Double-clicking a member enters individual-item selection mode so that member
  can be edited without selecting the other group members. A subsequent
  single-click returns to group selection.
- Group and ungroup are available from the node context menu when applicable.
- Group membership is stored as typed node data and is preserved by canvas
  snapshots. It does not require a new database table or IPC command.
- Copying or inserting a group creates fresh group identifiers so copied items
  never join the originals.
- Ungroup removes membership metadata while leaving node content, positions,
  connections, and styling unchanged.
- Existing Library item-group creation and its `Ctrl+G` shortcut remain
  unchanged.

## Non-goals

- Nested groups, group resizing, rotation, or editing all member contents as a
  single item.
- Group membership between items on different Boards.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Manually verify grouping, re-selection, movement, ungrouping, undo, persistence,
  and duplicate/Library insertion isolation.
