# Vora Mind Path Segments and Library

> **Status**: IMPLEMENTED
> **Target Scope**: `canvas | library | db | tauri | types`

## Intent

Make a Path a continuous, editable canvas object whose individual intervals
between adjacent anchor points can be selected, named, described, recolored,
detached, saved to the shared Library, and rejoined without losing their
identity or styling.

## Confirmed behavior

- Existing Paths remain visually and structurally compatible. Their adjacent
  point pairs are treated as independently addressable segments; old Paths
  receive stable segment metadata when first edited.
- Clicking a segment selects that segment while keeping the Path node selected.
  Segment name, details, and color can be edited without changing neighboring
  segments. Anchor points remain draggable.
- Clicking a segment without a modifier replaces the segment selection across
  all Paths. Ctrl-click or Cmd-click toggles only that segment in the
  selection, preserving other selected sections and deselecting it when clicked
  again. Modifier-clicking never cuts, clones, or splits a Path.
- Shift-clicking a segment adds it to the current selection without clearing
  other selected sections; repeated Shift-clicks on an already selected
  segment keep it selected. Shift-clicking never cuts, clones, or splits a Path.
- Ctrl-dragging or Cmd-dragging a selected Path moves it without creating a
  clone. Explicit copy/paste remains available.
- Cutting a segment detaches it as its own Path node and preserves the
  remaining connected sections and all segment metadata. Cutting is available
  only through the explicit **Cut section** action. Undo restores the original
  Path.
- Rejoining selected Path nodes is allowed when their endpoints meet. Joining
  preserves the individual segments, names, descriptions, colors, and Library
  identity; incompatible selections are not modified.
- A segment can be saved as a named Library item. An inserted Library segment
  is an independent canvas instance linked to the Library item identity.
- Library search can show the number of active Board usages and, for each use,
  the Board name and segment's canvas location. Historical versions and hidden
  Library-owned Boards are not counted as active Board usages.
- Segment metadata is part of the existing typed canvas-node snapshot data;
  no separate source-of-truth or frontend-only persistence is introduced.

## Non-goals

- Curves, automatic routing, or changing Path creation gestures.
- Editing one Library item's contents to mutate all its existing instances.
- Treating every anchor point as an independent Library asset.

## Data and persistence

- Add typed per-segment identity, name, details, color, and optional Library
  item reference to Path node data, while retaining the existing `points`
  representation for backward compatibility.
- Reuse the existing shared Library item-group storage for single-segment
  Library entries.
- Add a typed backend query for current active uses of a Library segment. Read
  the active resolved snapshot of each ordinary Board and return Board title,
  node identity, and exact canvas coordinates for both segment endpoints.
- Persist cut/join operations through the existing canvas history and
  autosave/version flow.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Test legacy Path defaults, per-segment styling, cut/join metadata retention,
  Library insertion identity, and active-Board usage lookup.
- Verify Ctrl-click and Cmd-click only toggle section selection; the number of
  Path nodes and the original connected geometry remain unchanged. Verify the
  explicit **Cut section** action still splits a Path and that undo restores it.
- Verify Shift-click adds multiple sections to the selection without toggling
  off sections that were already selected.
- Verify Ctrl-drag and Cmd-drag move a Path without creating another node.
