# Vora Mind Canvas Freehand Drawing

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | types | spec`

## Goal

Add a persistent freehand pen tool to the Vora Mind Board canvas, with the
English `P` shortcut and the existing shape styling, selection, history, and
snapshot pipeline.

## Behavior

- The canvas toolbar exposes a Freehand tool; pressing `P` activates or
  deactivates it when focus is not in an editable control.
- While active, pointer-drag gestures create separate smooth, rounded strokes.
  The tool stays active for successive strokes and works over existing canvas
  items. Escape or right-click cancels the in-progress stroke and exits the
  tool.
- Strokes use the Freehand style defaults and existing shape toolbar controls
  for line color, opacity, width, and style. Completed strokes can be selected,
  moved, duplicated, styled, deleted, grouped, and undone/redone like other
  canvas shapes.
- Stroke geometry is stored as normalized points in a typed `voraShape` node.
  Existing Board snapshots, autosave, version history, and Board JSON
  import/export preserve it; no SQLite schema or backend command is added.
- Existing Rectangle, Path, text, card, and diagram interactions remain
  unchanged.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Manually verify shortcut focus guards, repeated strokes, selection/style,
  cancellation, undo/redo, and Board reload/import/export.
