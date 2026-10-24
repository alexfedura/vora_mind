# VeraMind Home Canvas Shape Tools

> **Status**: IN PROGRESS
> **Target Scope**: `canvas | types | db`

## Intent

Add Rectangle and Path drawing tools to the VeraMind Home canvas, using the same core interactions and visual language as the VeraChart drawing tools while storing geometry in ReactFlow node coordinates.

## Behavior

- The Home canvas tool rail exposes mutually exclusive Rectangle and Path tools alongside existing Card and Text tools.
- My desk (Boards) opens by default whenever Vora Mind is entered, including navigation from VeraChart.
- All buttons in the Home canvas tool rail and the left/right panel toggles share one square hit area, centered 16px icons, and consistent idle, hover, and active colors; Rectangle and Path artwork matches the Card and Text icons.
- The panel switch button is separate from the Home canvas tool rail and remains at the same position outside either open panel. With menus closed, it remains at the top-left, level with the right panel toggle. The Boards panel matches the Vora panel width; its Create Board button sits to the left of the panel switch with a clear gap.
- The Boards title, Create Board icon, panel switch icon, and right panel toggle icon share the same vertical centerline.
- Left-clicking the panel switch changes between Boards and Vora menus; right-clicking opens the last active menu or closes the currently open menu.
- Rectangle is created by dragging across the canvas. Its geometry is stored as a typed shape node with normalized bounds; it supports selection, move, delete, and undo/redo, and resizing through eight visible handles at the corners and side midpoints.
- Rectangle and Path live previews use the saved style defaults for their respective tools, matching the finished shape's color, opacity, width, line style, fill, and path markers from the start of drawing.
- Rectangle and Path nodes are selectable and draggable from a hit area extending 5 screen pixels on each side of their visible border/path stroke; clicking the shape interior or its empty bounding area does not select it.
- Pressing Delete removes the selected canvas node or edge; Backspace is not used as the canvas delete shortcut.
- Canvas undo/redo records each completed user action as one step, including node movement, shape resizing, styling changes, duplication, deletion, and connections. Repeated moves or edits are undone in reverse order without skipping an action.
- The shape toolbar matches the compact dark single-row chart drawing toolbar treatment and includes functional label, line/fill, width/style, settings, lock, duplicate, delete, and overflow actions.
- The selected-shape toolbar uses the same compact dark single-row 11-slot layout and controls as the Chart drawing toolbar for both rectangles and paths. Templates, line color, fill color, text color, width, style, settings, lock, remove, and more keep the Chart order; fill is disabled for paths and the price-alert slot remains disabled because Mind shapes have no price source.
- Floating-toolbar icons are centered horizontally and vertically within their control cells; the width control centers its icon and value together without shifting neighboring cells.
- Default shape names are not rendered inside rectangles or paths; custom labels remain visible with configurable color and opacity. Style templates are saved separately per shape type and can be applied or removed from the toolbar.
- Line and rectangle-fill controls use the same chart color palette, including its saved custom colors, custom HSV/hex picker, and editable opacity percentage. Paths do not expose fill controls.
- Path is created by clicking vertices, previews the next segment and end arrow, and finishes on double-click or Enter without changing canvas zoom; Escape cancels. Its typed node stores normalized points and start/end marker settings.
- Right-clicking blank canvas while a rectangle or path is in progress discards the unfinished preview and exits the active drawing tool; right-clicking while the text-placement tool is active also cancels that tool. No partial shape or text node is created.
- A text node with empty or whitespace-only content is automatically removed when the user clicks or right-clicks anywhere outside that text node on the canvas, including blank space or another canvas item. Text nodes with content remain unchanged.
- Selected rectangles and paths expose a TradingView-inspired floating editing toolbar with drag, line/fill color and opacity, label, width, line style, settings, lock, duplicate, and delete actions. Its layout is consistent for both shape types; fill controls are disabled for paths.
- The selected-shape toolbar, including the toolbar for multi-selection, can be repositioned by dragging its grip or with keyboard arrow keys; single- and multi-selection positions are remembered separately.
- The selected-shape settings panel stays at a fixed screen size at every canvas zoom level. It remains interactive while changing controls and closes when its toolbar is toggled off or its shape is deselected.
- A multi-selection of shapes shows exactly one floating toolbar initially above the topmost selected shape. Its grip can move the toolbar independently of the selected shapes. Shared styling, lock, and remove actions apply to every selected shape; type-specific fill is available only when every selected shape is a rectangle.
- Rectangle and Path style defaults are stored separately in local storage and reused for subsequently created shapes.
- Paths have no visible React Flow node or selection-overlay frame or resize handles. Path anchor points can be dragged beyond their current bounds; the invisible node bounds expand with the path so endpoint editing is not constrained by a selection box.
- Selected Path anchors use larger hollow blue handles with a dark canvas-colored center, a stronger hover state, and a zoom-independent 28px pointer target.
- Shape nodes use `voraRectangle` and `voraPath` node types and remain distinct from cards and text nodes.
- Mind snapshots preserve arbitrary node type and data through the existing JSON `object_versions.content` field and restore the original node type/data and placement. Existing card snapshot payloads remain backward-compatible.
- The latest saved canvas snapshot loads automatically when Vora Mind opens. Canvas changes are debounced and saved as snapshots, including before leaving the page, so shapes remain present after reopening Vora Mind or switching between application sections.
- Restored shape nodes reapply their shape-specific CSS classes so their expected drawing and hit-testing behavior is retained.
- No new SQLite table is introduced.

## Verification

- Frontend: `pnpm tsc --noEmit` and `pnpm build`.
- Backend: `cargo check --manifest-path src-tauri/Cargo.toml`.
- Snapshot tests cover shape type/data round-trip and existing card resolution.
