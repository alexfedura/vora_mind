# Vora Chart Path Style Popup

> **Status**: APPROVED
> **Author**: Copilot
> **Target Scope**: `chart`

## 1. Intent & System Goals

* **Objective**: Replace the generic Path drawing settings dialog with a compact popup anchored to the drawing toolbar's settings button, following the supplied reference recording.
* **Target Workflow**: Open settings for a selected Path drawing, edit its line color, opacity, width, dash style, and endpoint arrows in the popup, then apply with **Ok** or discard with **Cancel**. Keep the existing template menu available; close the popup on outside click or Escape.
* **Non-Goals**: Redesign settings for other drawing types or implement the Visibility tab.

## 2. Affected Architectural Boundaries

* [x] `src/components/market/` (Path settings UI and toolbar wiring)
* [x] `src/App.css` (Path popup and controls)

## 3. Type Contracts & Data Schema Modifications

No domain, IPC, or database contract changes. Path style values already exist in `DrawingGeometry`.

## 4. Implementation Plan

1. Add a Path-specific compact, toolbar-anchored style popup while preserving the existing settings dialog for other drawing types.
2. Show the color palette, opacity, line width/style, and start/end arrow controls in the popup, backed by the existing drawing draft.
3. Preserve existing apply/cancel behavior and template selection; leave Visibility out of scope.
4. Run the required TypeScript, production build, and Rust checks.

## 5. Verification & Acceptance Criteria

* [ ] **TypeScript Check**: `pnpm tsc --noEmit` exits with status `0`.
* [ ] **Build**: `pnpm build` exits with status `0`.
* [ ] **Rust Backend Check**: `cargo check --manifest-path src-tauri/Cargo.toml` completes with `0` errors.
* [ ] **Runtime Validation**: Path settings show the compact Style-only popup anchored to the toolbar; color, opacity, width, line style, and endpoint changes apply on **Ok** and are discarded on **Cancel**. The popup closes on outside click or Escape.
