# Feature Specification: TradingView-Inspired VORA Chart Layout

> **Status**: IN PROGRESS
> **Target Scope**: `chart`

## Intent

Align the VORA chart shell with the supplied TradingView reference while preserving VORA's market model and existing controls. This is a layout adaptation, not a copy of TradingView artwork or unsupported behavior.

## Phase 1 — Chart Shell

- Open the live Chart page in candlestick mode by default, regardless of an older saved live display-mode preference; explicitly restored snapshots continue to use their saved display mode.
- Use a compact, single-row chart header with the symbol, VORA's real M0-M10 scale choices, chart style, undo/redo, coordinate controls, and existing snapshot/panel actions.
- Render Cross as the primary cursor action with a separate 11px Cursors dropdown arrow and accessible cursor menu.
- Match the Cursors menu's 262px by 298px desktop geometry, item ordering, separators, 28px icons, hover favorites, and long-press values switch.
- Persist cursor favorites and expose supported favorites in the bottom drawing toolbar.
- Persist the long-press setting and show chart time/price values after a stationary long press.
- Keep cursor choices without implemented chart interactions disabled.
- Keep the existing left drawing rail and right dock in their current ownership boundaries.
- Dock the global drawing toolbar at the bottom center of the chart by default, while preserving its drag behavior and stored user position after the user moves it.
- Keep selected-drawing editing controls attached to the selected drawing; do not merge them into the global drawing toolbar.
- Preserve chart sizing, drawing interactions, and narrow-window overflow behavior.

## Phase 2 — Chart Interval Selector

- Use `1m` as the initial live-chart interval when no interval preference is
  saved; continue restoring an explicitly saved interval and snapshot interval.
- Replace the always-visible M0-M10 chips with a compact current-interval split dropdown. Keep VORA's M0-M10 intervals in a dedicated `VORA M-scale` group inside the picker.
- Match the TradingView menu hierarchy: Ticks, Seconds, Minutes, Hours, Days, Weeks, Months, and Ranges. Use VORA-owned Lucide icons, selected-row contrast, persisted favorites, accessible menu rows, Escape/outside dismissal, and viewport-clamped scrolling. Selecting a row's star adds or removes its interval shortcut beside the picker in the chart header.
- Implement time intervals as canonical keys (`1s`, `5m`, `4h`, `1w`, `1mo`). Existing cached timeframe candles are preferred. Missing fixed-second intervals are aggregated in memory from the closest available finer candle series, falling back to M0; do not persist duplicate aggregates.
- Implement custom time intervals with a positive integer and unit (seconds, minutes, hours, days, weeks, or calendar months). Custom values are clamped to one second through twelve calendar months, validated in the backend, and kept in a local saved-interval list. Calendar months bucket by UTC calendar boundaries, not fixed 30-day periods.
- Persist the selected interval locally and in snapshots. Restore existing `M0`-`M10` snapshots unchanged. Keep policy/drawing scale state mapped to the nearest VORA M-scale step when selecting a standard interval.
- Keep Ticks and Ranges visible but disabled with an explanatory accessible title while the source only provides one-second OHLC candles. Exact tick bars require raw ordered tick IPC; range bars additionally require instrument tick-size metadata. Do not synthesize either from lossy M0 candles.
- Add Rust tests for interval parsing, custom validation, UTC month bucketing, fixed-period aggregation, ordering, and empty-source errors. Extend the existing IPC tests for direct cache and fallback aggregation.
- Preserve chart viewport across interval changes and keep request race guards keyed by the full interval key, not only its mapped M-scale step.
- Keep the live ChartPage mounted while navigating to the Snapshots list and back so its exact interval, viewport/zoom, drawings, and overlay state remain unchanged. Pause active replay while hidden and resume it on return. Opening a snapshot remains an explicit restore action.
- Every explicit Snapshot selection, including reopening the same ID, reapplies its saved symbol, interval, candles, display mode, drawings, policy/registry, theme, and viewport.

## Phase 3 — Bar Replay

- Add an icon-only Bar Replay toggle to the chart header immediately after Redo, matching the supplied 38px button and 28px icon. When active, show a compact replay toolbar in a dedicated bottom dock inside the chart shell. The dock reserves layout space rather than overlaying the plot and keeps the 48px dark toolbar centered: Select bar with a mode menu, play/pause, step forward, playback speed (default `1x`), update interval (default active chart interval), jump to the latest bar, and exit.
- Clicking the header Replay button or `Select bar` starts a fresh bar-selection session. Show the blue cursor guide only during selection and remove it after a candle is selected.
- Re-entering bar selection preserves the current replay cutoff; only exiting Replay restores candles hidden by the cut.
- Selecting a bar or date/time establishes the replay start. Keep the complete fetched candle series in page state, but render only candles through the replay cursor; restore the full series on exit.
- The starting-point menu has `Bar`, `Date`, `First available date`, and `Random bar` modes. Date selection uses UTC and snaps to the nearest available candle.
- Draw a blue vertical replay cursor at the active candle. Future candles to its right are hidden from the rendered series while the replay cursor advances through the complete source series.
- The update-interval menu contains `1 tick`, `1 second`, and the active chart interval, plus an `Auto select interval` switch. Tick updates remain disabled until tick data exists; auto-select follows the chart interval. The visible interval label follows the selected/automatic interval.
- Playback speed options include `0.25x`, `0.5x`, `1x`, `2x`, `3x`, `5x`, and `10x`. Play advances at the selected speed and update interval, pauses at the end, and is cleaned up on exit or unmount.
- Persist the selected playback speed locally and restore it when the chart or Replay mode is reopened.
- Toolbar controls expose accessible labels, pressed/disabled states, and keyboard support. Space toggles playback; ArrowLeft/ArrowRight step backward/forward. Selection mode must not create chart drawings or measurements.
- Include the replay range strip (`1D`, `5D`, `1M`, `3M`, `6M`, `YTD`, `1Y`, `5Y`, `All`, and a date picker) and UTC clock inside the same integrated dock. Keep the `Replay Trading` dock visible with its available chart-tool actions.
- Preserve a useful chart viewport while replay advances, keep the replay price marker synchronized to the last visible candle, and leave normal chart state unchanged after replay exits.

## Acceptance Criteria

- Header controls fit on one row at desktop width and expose the active M scale with pressed states.
- The Cursors arrow reveals on hover/focus, opens its menu, and keeps unsupported cursor choices disabled.
- Cursor menu favorites persist and are reflected in the favorite toolbar.
- The values switch persists and controls a long-press chart value tooltip.
- The global drawing toolbar starts centered at the bottom of the chart and can still be repositioned.
- Existing tool selection and chart actions remain functional.
- The interval dropdown opens all sections, standard intervals load candles, custom intervals validate and load, favorites/custom intervals persist, favorited intervals appear as selectable header shortcuts, and the Ticks/Ranges rows remain disabled until their backend contracts exist.
- Interval changes preserve chart viewport, map the policy step to the nearest M-scale step, and snapshot restore round-trips the selected interval. During a manual interval change in Bar Replay, keep Replay open, remap its start and current bar by timestamp, preserve playback state, and do not auto-follow away from the preserved viewport.
- All drawings remain visible across every timeframe unless explicitly hidden.
- Bar Replay appears after Redo in the chart header with the reference button/icon sizing. Its controls occupy an integrated bottom dock that does not overlap the chart plot; Replay supports all four starting-point modes, marks its cursor in blue, hides future candles until playback reaches them, advances manually and automatically at selectable speed/update interval, and restores the full chart on exit.
- `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml` pass.