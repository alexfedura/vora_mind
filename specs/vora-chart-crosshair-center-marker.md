# Feature Specification: VORA Chart Crosshair Center Marker

> **Status**: IMPLEMENTED
> **Target Scope**: `chart`

## Intent

Show a visible plus at the mouse hotspot while the pointer is over the chart drawing area. The marker improves identification of the exact chart point without changing crosshair tracking, drawing behavior, axes, or floating controls.

## Implementation

Use a small inline SVG CSS cursor with a centered hotspot and contrasting crosshair/plus strokes for the chart's existing hidden-cursor and crosshair modes. Keep the default cursor used by chart chrome and floating controls unchanged.

## Acceptance Criteria

- Moving over the chart shows a clearly visible plus centered at the pointer hotspot.
- The crosshair drawing tool retains its crosshair behavior.
- Chart axes, toolbar, and floating controls retain their current cursor behavior.
- `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml` pass.