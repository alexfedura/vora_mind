# Vora Mind Shared Axis Zoom

## Goal

Add shared horizontal and vertical zoom controls to the Vora Mind canvas,
using a default level of zero, and relocate the AI Agent action to the top-left
canvas controls.

## Behavior

- Display only one shared X/Y zoom control in a compact bottom-right panel. Do not
  include Frames, Help, Resources, a minimap, time, or price.
- X and Y share one zoom level and scale. Display one zoom control and one
  readable signed level that represents both axes. Level zero is the default
  on entering Vora Mind.
- Zoom controls remain usable in both directions without disabling a control
  at zero; keep the level independent from the chart's time/price scales.
- Vertical wheel scrolling without Ctrl over any canvas content zooms both
  axes together around the pointer and never pans the canvas. Ctrl + wheel
  over the canvas does not change zoom. Zoom buttons preserve the canvas
  center and change both axes together.
- Holding Space temporarily enables panning with left-button drag over the
  canvas, including while drawing tools are active. Releasing Space restores
  the normal canvas interaction mode.
- Show the exact shared X/Y zoom level, rounded to three decimal places, and
  update it immediately with each zoom change.
- Preserve node selection, movement, creation, shape drawing, and canvas
  persistence while applying axis zoom.
- Move AI Agent to an icon-only square control directly below the Boards/menu
  toggle. Match the toggle's hit area and use an enlarged icon. Keep equal gaps
  between the toggle, AI control, and tool rail by moving the rail down.

## Implementation notes

- React Flow exposes one viewport zoom value; compose the shared X/Y scale onto
  its viewport as a uniform axis transform.
- Convert pointer-derived canvas positions and node drag deltas back into the
  unscaled canvas coordinate system.
- Use the transformed DOM bounds for marquee selection so selection remains
  aligned with what the user sees. Start custom marquee selection only on the
  blank pane; node and edge pointer events must reach React Flow normally.
- Avoid `fitView` so the zero X/Y levels retain a neutral scale of 1 instead of
  inheriting an automatic uniform zoom.
- Keep zoom levels as view-only UI state; do not change board/node data or the
  chart's persisted viewport.
- Keep floating editing toolbars and their open menus at a fixed screen size,
  independent of horizontal and vertical canvas zoom.

## Verification

- Run `pnpm tsc --noEmit`, `pnpm build`, and `cargo check --manifest-path
  src-tauri/Cargo.toml`.
- Verify zero default, shared X/Y updates, centered zoom, marquee
  selection, shape creation, node dragging/resizing, and the revised top-left
  controls. Verify wheel scrolling over blank space and canvas items zooms both
  axes at the pointer without panning, while Ctrl + wheel leaves both ranges
  unchanged. Verify Space + left-button drag pans in all directions, then
  releasing Space restores selection and drawing interactions.
