# Feature Specification: VORA Chart Right Toolbar

> **Status**: IMPLEMENTED
> **Target Scope**: `chart`

## Intent

Add a TradingView-inspired right-side chart toolbar with Watchlist, Alerts, Object Tree, Products, and Help entries. Match the compact vertical rail and dark chart palette while keeping the chart usable at narrower widths.

## Behavior

- Clicking a toolbar entry opens its panel; clicking the active entry or its close button closes it.
- Watchlist reuses the existing market-source list and symbol selection.
- Object Tree lists current drawings and selects a drawing on click.
- Products links to existing Canvas, Chart, Sources, and Snapshots routes.
- Help shows chart navigation and drawing guidance.
- Alerts opens an honest empty state; alert persistence and triggering are out of scope because no alert domain/service exists yet.
- The toolbar and panel are session UI state only; no database or schema changes.

## Acceptance Criteria

- Five labeled, keyboard-accessible icon buttons render in a 45px right rail.
- The active panel is docked between chart and rail without covering chart controls.
- Panel open/close and active-state behavior work by mouse and keyboard.
- Existing Watchlist filtering and symbol selection continue to work.
- Object Tree reflects the current drawings and can select them on the chart.
- `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml` pass.