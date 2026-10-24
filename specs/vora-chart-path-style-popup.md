# Vora Chart Path Style Popup

> **Status**: APPROVED
> **Author**: Copilot
> **Target Scope**: `chart`

## 1. Intent & System Goals

* **Objective**: Replace the generic Path drawing settings dialog with a compact popup anchored to the drawing toolbar's settings button, following the supplied reference recording.
* **Target Workflow**: Select a whole Path with a normal click, or select one/more individual segments by holding **Ctrl** and clicking the segment between adjacent hollow blue anchors. Open settings for the selected Path, edit its line color, opacity, width, and dash style for the whole Path or only the selected segments, and edit arrows only at the Path's two endpoints. Apply with **Ok** or discard with **Cancel**. Keep the existing template menu available; close the popup on outside click or Escape.
* **Non-Goals**: Redesign settings for other drawing types or implement the Visibility tab. Segment selection does not add arrowheads at interior anchors.

## 2. Affected Architectural Boundaries

* [x] `src/components/market/` (Path settings UI and toolbar wiring)
* [x] `src/App.css` (Path popup and controls)

## 3. Type Contracts & Data Schema Modifications

Persist per-segment style overrides on `DrawingGeometry`; keep the selected segment indexes as transient drawing-selection state.

## 4. Implementation Plan

1. Add a Path-specific compact, toolbar-anchored style popup while preserving the existing settings dialog for other drawing types.
2. Show the color palette, opacity, line width/style, and start/end arrow controls in the popup, backed by the existing drawing draft.
3. Preserve existing apply/cancel behavior and template selection; leave Visibility out of scope.
4. Support whole-Path selection with a normal click and Ctrl+click segment selection (first Ctrl+click isolates a segment; subsequent Ctrl+clicks add or remove segments). Apply floating-toolbar and popup style changes to the selected segment set, while endpoint arrows remain Path-level.
5. Run the required TypeScript, production build, and Rust checks.

## 5. Verification & Acceptance Criteria

* [x] **TypeScript Check**: `pnpm tsc --noEmit` exits with status `0`.
* [x] **Build**: `pnpm build` exits with status `0`.
* [x] **Rust Backend Check**: `cargo check --manifest-path src-tauri/Cargo.toml` completes with `0` errors.
* [ ] **Runtime Validation**: Path settings show the compact Style-only popup anchored to the toolbar; color, opacity, width, line style, and endpoint changes apply on **Ok** and are discarded on **Cancel**. The popup closes on outside click or Escape.
* [ ] **Segment Selection**: Normal click selects the whole Path; Ctrl+click selects one or more segments, renders the selected segments distinctly, and each floating-toolbar style change affects only those segments. Ctrl+click toggles segments; clearing the final segment selection returns to whole-Path selection. Arrow controls affect only the Path endpoints.
