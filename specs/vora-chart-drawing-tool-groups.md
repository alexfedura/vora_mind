# Feature Specification: VORA Chart Drawing Tool Groups

> **Status**: IN PROGRESS
> **Target Scope**: `chart`

## Intent

Add the TradingView-referenced left drawing toolbar to VORA Chart with grouped menus, tool selection, utility controls, keyboard accessibility, and chart-appropriate interactions. Use VORA's own icons/style rather than copying TradingView's proprietary SVG artwork.

## Reference Groups

1. Cursors: Cross, Dot, Arrow, Demonstration, Magic, Eraser.
2. Trend tools: lines, channels, pitchforks, and related trend constructions.
3. Gann and Fibonacci: retracements, extensions, fans, arcs, circles, time zones, wedges, and Gann tools.
4. Patterns: XABCD, Cypher, Head and shoulders, ABCD, Triangle, and Three drives patterns; Elliott impulse, correction, triangle, double-combo, and triple-combo waves; Cyclic lines, Time cycles, and Sine line.
5. Forecasting and measurement: Long position, Short position, Position forecast, Bars pattern, Ghost feed, Sector, Anchored VWAP, Fixed range volume profile, Anchored volume profile, Price range, Date range, and Date and price range.
6. Geometric shapes: Brushes (Brush, Highlighter), Arrows (Arrow marker, Arrow, Arrow mark up, Arrow mark down), and Shapes (Rectangle, Rotated rectangle, Path, Circle, Ellipse, Polyline, Triangle, Arc, Curve, Double curve), in that order.
7. Annotation: text and labels.
8. Icons: chart markers and symbols.
9. Utility controls: Measure, Zoom, Magnet, Keep drawing, Lock, Hide, Remove.

## Phased Implementation

### Selected Drawing Floating Toolbar

- Keep the selected-drawing toolbar separate from the left tool-family rail and show exactly one toolbar whenever one or more drawings are selected.
- For multi-selection, show only controls supported by every selected drawing: color and opacity, line style, line width, lock/unlock, hide, and remove. Hide type-specific controls such as fill, templates, settings, alerts, and ordering; never render one toolbar per selected drawing. When selected drawings have different colors, opacities, widths, or line styles, show a mixed state rather than presenting the first drawing's value as shared.
- Apply shared-property changes and hide, lock, and remove actions to the complete selected set in one undoable operation. A mixed lock selection locks all selected drawings; an all-locked selection unlocks them.
- Keep the single-selection toolbar controls and behavior unchanged.
- Match the reference's compact 11-control order: templates, line color, fill color, text color, line style, line width, settings, alert, lock, remove, and More.
- Target the reference geometry: approximately 423px wide by 34px tall, 35px-wide by 34px-tall controls, and a 24px-wide drag handle. Separate the line-style/width group from the settings-and-actions group with subtle dividers.
- Center every toolbar icon horizontally and vertically within its cell; active-state indicators must not shift the icon within that cell.
- Keep VORA-owned Lucide icons; use GripVertical at 16px for dragging and LayoutTemplate, Pencil, PaintBucket, Type, Minus, Settings2, BellPlus, LockKeyhole, Trash2, and MoreHorizontal at 18px for actions. The line-style preview is 20px by 1px, color indicators are 16px by 3px, and dividers are 1px by 16px.
- Match the dark palette and hover/pressed states, color palettes, width menu, settings dialog, alert dialog, and More menu.
- Preserve pointer-captured drag for the toolbar's drag handle, viewport clamping, persisted position, and cleanup on cancel, blur, resize, and unmount.
- Make line/fill/text color and opacity, width, style, lock, delete, templates, and supported text/coordinate/visibility settings change the selected drawing and survive reload.
- Implement the reference More actions: visual ordering, interval visibility, clone, copy/paste, and hide; expose state in the object tree.
- Add alerts as typed SQLite-backed records, evaluate selected-drawing entry/price conditions against chart candles, and show/manage them in the Alerts dock.
- Keep all controls keyboard-accessible with real labels, pressed/disabled states, focus handling, and Escape/outside-click dismissal.
- Extend legacy drawing migration with defaults so existing saved drawings remain visually unchanged.

### Phase 1 — Toolbar and Existing VORA Tools

- Replace the flat floating tool strip with a fixed left rail and accessible grouped popovers.
- Give each tool family a split control: the primary button reflects and reselects the active/first supported tool; the 11px trailing arrow opens its menu.
- Match the reference menu's 276px width, compact dark rows, section headings, selected-row contrast, supported keyboard shortcuts, and hover/focus favorite controls. Persist drawing-tool favorites locally.
- Match the geometric menu's Brushes, Arrows, and Shapes sections and reference ordering; retain VORA-owned Lucide icons and keep unsupported tools disabled without an availability badge.
- Render family menus in a viewport-clamped portal so long groups remain scrollable and are not clipped by the rail.
- Connect existing working tools: cursor, trendline, horizontal/vertical lines, horizontal ray, rectangle, and measure.
- Connect existing magnet, lock/delete selection, hide-all, keep-drawing, and remove controls without duplicating chart state.
- Match the Remove objects control at 52x38px with its 28x28 trash icon and 289x153px menu. Show count-based Remove drawings, Remove indicators, and Remove both actions, followed by the persisted "Always remove locked drawings" switch (on by default). Keep the primary-series data intact; drawing removals are undoable and persist per symbol, and their SQLite-backed alerts are removed transactionally.
- Match the Hide objects control's 28x28 eye icon and 187x128px menu: Hide drawings, Hide indicators, Hide positions and orders, and Hide all. Wire supported VORA drawing visibility without affecting the primary chart series.
- Match the Lock objects control's open/closed 28x28 icons and 123x108px menu: Lock drawings, Lock indicators, and Lock all. Main-button toggling and supported drawing lock actions update the existing lock model and undo history.
- Match the Magnet control's 28x28 weak/strong icons and 203x137px menu. Weak magnet snaps to an OHLC point only within 30px; Strong magnet snaps to OHLC on the candle under the cursor. Outside a candle's logical slot, both modes behave as Off. Persist Off/Weak/Strong mode and keep Snap to indicators visibly disabled until indicators exist.
- Use the supplied 28x28 Zoom in SVG in the 52x38 utility button and preserve the existing chart time-scale zoom action.
- Use the supplied rotated-ruler 28x28 SVG in the 52x38 Measure button; preserve the existing two-click measurement behavior and toolbar pressed state.
- Use the supplied 28x28 pencil-and-lock SVG for Keep drawing while preserving its current pressed state and keep-drawing toggle behavior.
- Do not present unimplemented drawing tools as active; expose them in subsequent phases when their interactions are implemented.

### Phase 2 — Drawing Families

- Standardize editable drawing handles: show the same blue (#2962ff) circular anchors on hover and fill them blue on selection; rectangles keep four circular corner handles and four square edge handles. Hover and selection must not brighten or thicken the drawing itself.
- Implement each menu tool end-to-end before enabling it: drawing model, chart-coordinate rendering, hit testing, selection/editing, undo/redo, persistence, object-tree naming, and selected-drawing controls.
- Build the Icons picker as a viewport-clamped 354px popover with a 48px top category toolbar, a scrollable 38px glyph grid containing all groups, and bottom tabs for Emojis, Stickers, and Icons. The active category and tab must be exposed accessibly; category buttons scroll to their group, and switching tabs resets to its first group. Selecting a glyph arms placement; the next chart click creates a chart-time/price anchored symbol that renders, hit-tests, selects, drags, locks, deletes, undoes/redoes, persists per symbol through the existing chart drawing store, appears in Object tree, and round-trips through snapshots. Use native Unicode glyphs and VORA-owned icons rather than copying third-party artwork; no new SQLite or IPC contract is required for this picker.
- Build the Text and notes menu at the reference's compact width with the heading and these ten ordered tools: Text, Note, Price note, Pin, Table, Callout, Comment, Price label, Signpost, and Flag mark. Use VORA-owned Lucide icons and remove Content, Image, Post, and Idea. Each tool creates a named chart-time/price icon drawing with its matching glyph and reuses existing selection, drag, lock, delete, undo/redo, per-symbol persistence, text settings, object-tree, and snapshot behavior. Do not add a parallel database or IPC path.
- Fibonacci tools (implemented), in reference order: Fib retracement (2 anchors), Trend-based Fib extension (3 anchors), Fib channel (3 anchors), Fib time zone (2 anchors), Fib speed resistance fan (2 anchors), Trend-based Fib time (2 anchors), Fib circles (2 anchors), Fib spiral (2 anchors), Fib speed resistance arcs (2 anchors), Fib wedge (2 anchors), and Pitchfan (3 anchors). Store chart-time/price anchors and render deterministic levels, paths, and labels through the shared geometry used by preview and hit-testing.
- Gann tools (implemented), in reference order: Gann box (2 opposite corners), Gann square fixed (origin plus scale point), Gann square (2 opposite corners), and Gann fan (2 anchors). Render their grids and rays from persistent chart anchors so they follow chart pan/scale changes.
- All 15 Fib/Gann tools support staged creation, selection/anchor editing, hit-testing, undo/redo, per-symbol reload, object-tree naming, selected-drawing settings, clone, alerts, and snapshot round-trip. Keep future family entries disabled until they satisfy the same end-to-end contract; no new backend table is needed for these local drawing records.
- Visible Trend tools backlog, in reference order:
	1. Trendline (existing)
	2. Ray (implemented: two anchors, one-way viewport extension, endpoint editing, persistence, clone/settings/object tree, and price alerts)
	3. Info line (implemented: two anchors with price, percentage, and elapsed-time readout)
	4. Extended line (implemented: two anchors with bidirectional viewport clipping, editing, persistence, object tree, and alerts)
	5. Trend angle (implemented: two anchors with an angle arc/readout and editable endpoints)
	6. Horizontal line (existing)
	7. Horizontal ray (existing)
	8. Vertical line (existing)
	9. Crossline (implemented: persistent one-click price/time intersection, editing, object tree, and dual-axis alerts)
	10. Parallel channel (implemented: three-anchor staged preview, two parallel boundaries, editing/settings, alerts, and persistence)
	11. Regression trend (in progress: least-squares fit of closes between two time anchors with ±1 standard-deviation residual rails)
	12. Flat top/bottom (pending: anchor count and horizontal-boundary construction)
	13. Disjoint channel (pending: independent-boundary anchor topology)
	14. Pitchfork (implemented: three anchors, median and tine rays, editing, settings, alerts, and persistence)
	15. Schiff pitchfork (implemented: midpoint-shifted first-pivot price, parallel tines, editing, and alerts)
	16. Modified Schiff pitchfork (implemented: first-pivot midpoint shift in time and price)
	17. Inside pitchfork (blocked: keep disabled until its distinct anchor/tine construction is verified; official reference fetch was unavailable)
- The Patterns menu catalog contains the chart-pattern, Elliott-wave, and cycle entries in reference order. Keep each entry disabled until its drawing model, chart-coordinate rendering, hit testing, selection/editing, undo/redo, persistence, and object-tree behavior are implemented.
- Continue in the same one-tool-at-a-time manner for pattern, annotation, and icon drawing behavior.
- Provide chart-coordinate rendering, hit testing, selection/editing, undo/redo, and per-symbol persistence for each model.
- Update shared frontend types and backend persistence contracts before persisting new domain data.

### Geometric Tools — Full Contract

Implement the visible Geometric shapes family as one complete feature set, in this exact menu order and with these interactions:

1. Brush: pointer-drag freehand stroke, sampled in chart time/price coordinates.
2. Highlighter: same freehand interaction as Brush, with a wide translucent stroke.
3. Arrow marker: two-anchor directed marker oriented from the first anchor toward the second.
4. Arrow: two-anchor directed line, with the arrowhead at the second anchor.
5. Arrow mark up: one-click upward marker anchored to chart time/price.
6. Arrow mark down: one-click downward marker anchored to chart time/price.
7. Rectangle: retain existing two-opposite-corner implementation.
8. Rotated rectangle: three anchors; first two define the baseline and the third defines perpendicular width.
9. Path: click to add chart-time/price vertices; preview the segment and arrowhead to the pointer; double-click or Enter finishes; Escape cancels. Render an open path with configurable start/end markers (None or Arrow), defaulting to no start marker and an end arrow.
10. Circle: center followed by radius point.
11. Ellipse: two opposite corners of its axis-aligned bounds.
12. Polyline: click to add vertices; double-click or Enter finishes; Escape cancels.
13. Triangle: three chart-coordinate vertices.
14. Arc: three anchors (start, end, and bulge point).
15. Curve: three-anchor quadratic Bezier.
16. Double curve: four-anchor cubic Bezier.

All added drawings use a discriminated, typed geometry model in the existing shared drawing union and the existing per-symbol drawing store. They must render in chart coordinates, have screen-space hit testing, support anchor editing and whole-shape dragging, selection, lock, delete, undo/redo, clone, object-tree naming, settings, and snapshot round-trip before their menu entries are enabled. Keep path selection forgiving with a stroke-width-aware hit radius of at least 12 CSS pixels, and use a 12 CSS-pixel hit tolerance around rectangle borders; hover cursors must use the same targets as selection. Brush/highlighter samples are simplified deterministically while preserving the rendered stroke. Path stores its start/end marker choices with the drawing and exposes them in Style settings. Use the supplied Path glyph consistently in the chart rail, menus, favorites, and object tree; use VORA-owned icons for the remaining tools. Existing rectangle appearance and stored records must remain unchanged.

### Forecasting and Measurement Tools — Full Contract

Implement these 12 tools in this menu order and section grouping:

**Forecasting**

1. Long position: three anchors define entry, stop, and target. Render TradingView-style risk/reward zones and four square handles for target, entry start/end, and stop. Show target/stop price distance, percentage, ticks, and amount, plus open P&L, quantity, and risk/reward ratio. Until a configurable position size exists, use quantity 1 and show amounts per one unit.
2. Short position: mirror the Long position icon, interaction, colors, handles, and readouts. Reward/target is below entry, risk/stop is above entry, and open P&L uses the inverse price direction.
3. Position forecast: two anchors define a projected price/time move; render the measured projection as a dashed continuation with delta, percent, and elapsed-bar readouts.
4. Bars pattern: first two anchors select a source candle interval; the third sets the destination. Copy source OHLC/volume samples into the projected interval and render the copied candles in a subdued overlay.
5. Ghost feed: click to add future time/price vertices, move to preview, Enter or double-click to finish, Escape to cancel. Render an unfilled, translucent projected path with ghost candles interpolated between vertices.
6. Sector: three anchors define center, radius start, and radius end. Render the bounded circular sector and its angle/area labels.

**Volume-based**

7. Anchored VWAP: one anchor starts a volume-weighted average-price line through the latest available candle. Recalculate from persisted chart candle OHLCV data when candles update.
8. Fixed range volume profile: two anchors select a candle interval. Bin actual candle volume across price levels and render a horizontal histogram beside the selected range.
9. Anchored volume profile: one anchor starts a price-binned volume histogram through the latest available candle, updating as chart data changes.

**Measurers**

10. Price range: two anchors render absolute price change, percentage change, and signed direction.
11. Date range: two anchors render elapsed time and candle count.
12. Date and price range: opposite-corner anchors render the time/price rectangle plus both sets of measurements.

Each tool uses a distinct VORA-owned Lucide icon and becomes active only with its complete implementation. Store chart-time/price anchors in the existing drawing store; keep Bars pattern source samples in the drawing record so it survives reload and snapshot round-trip. Volume-based overlays use the chart's actual candle volume, never synthetic volume. All drawings support selection, anchor editing, dragging, locking, deletion, undo/redo, templates/settings, object-tree naming, per-symbol reload, and snapshot round-trip. Price/date measurements and forecast labels must update after chart zoom, pan, and timeframe changes without changing the anchor data.

## Acceptance Criteria

- In cursor mode, Ctrl+left-drag draws a marquee over the plot and selects every visible drawing whose rendered bounds intersect it; axes and floating controls do not start a marquee. Ordinary left-drag continues to pan the chart.
- Phase 1 rail and groups match the reference hierarchy and remain usable at compact chart widths.
- Group buttons open/close their menu; selecting an implemented tool updates the active state and chart interaction.
- Utility controls update the existing chart/drawing state and expose pressed/disabled states accurately.
- Unimplemented tools are not represented as working controls.
- Phase 2 tools each create, render, select, edit, and persist the expected drawing without regressing existing tools.
- `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml` pass.