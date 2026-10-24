# Vora Chart Drawing Selection

## Behavior

- Right-clicking blank chart space deselects selected drawings, including
  rectangles and paths.
- Right-clicking directly over a drawing preserves its selection.
- Right-clicking while a path, multi-anchor drawing, rectangle, or measurement
  is unfinished cancels the preview and exits that drawing operation without
  creating or retaining a partial drawing.
- Cancelling an unfinished drawing suppresses the chart context menu. Existing
  drawing selection is cleared only when the pointer is over blank chart space.
- Right-clicking when there is no unfinished drawing preserves the chart
  context-menu behavior; right-clicking an existing drawing preserves its
  selection.
- Existing chart context-menu behavior remains available after a blank-space
  right-click.
- Selected or hovered drawing anchors use enlarged hollow blue handles filled
  with the chart background; their 14px hit radius makes them easier to grab.

## Verification

- Run `pnpm tsc --noEmit`, `pnpm build`, and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
- Manually verify blank-space deselection, right-clicking a selected drawing,
  and cancelling unfinished path, multi-anchor, rectangle, and measurement
  previews.
