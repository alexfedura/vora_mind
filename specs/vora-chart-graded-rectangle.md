# VORA Chart Graded Rectangle

> **Status**: APPROVED
> **Target Scope**: `chart | store | db`

## Intent

Add a Graded Rectangle chart drawing that is independent from the existing
two-click Simple Rectangle tool. It stamps from detected swing pivots, keeps a
fixed price height, permits horizontal resizing, measures correction from a
later swing, and persists its template and drawings through the existing
database-backed settings path.

## Contracts and units

- `GradedRectangle` is a separate `DrawingAny` variant with `type:
  "graded-rectangle"`, `structureType`, `anchorPrice`, `startTime`, `endTime`,
  `topPrice`, `bottomPrice`, `baselineSide`, optional `correctionStartPrice`,
  and the correction pivot time needed to draw its line.
- The Zustand template is `{ height, width, defaultBaseline }`. Height is
  measured in price units. Width is measured in chart bars and converted to
  timestamps using the chart's logical time scale.
- The initial template is 1 price unit high and 10 bars wide. RBR stamps use a
  top baseline; DBD stamps use a bottom baseline.
- The template is stored as a JSON value in the existing app settings store;
  the existing settings IPC persists it to SQLite. Drawings are stored in the
  existing per-symbol drawing setting alongside other chart drawings.

## Interaction

- Add a separate Graded Rectangle tool; do not alter Simple Rectangle behavior.
- While active, show the nearest existing swing pivot within 50 screen pixels.
  A swing high stamps an RBR box below the high; a swing low stamps a DBD box
  above the low. Center the configured bar width around the pointer's logical
  chart position.
- After stamping, clicking a later eligible opposite swing pivot records the
  correction start. Draw a line from that pivot to the latest candle close.
  Measure the absolute move as a percentage of the fixed box height. Crossing
  the far box boundary displays an engulfed label and highlights the box.
- Graded rectangles have purple styling and an anchor marker. They can be
  selected and moved; only left/right handles resize them. Horizontal resize
  updates and persists the global width in bars. The price height is immutable
  for each drawing.
- Right-clicking a graded rectangle offers “Set Top as Baseline” and “Set
  Bottom as Baseline”.
- Changing baseline changes the per-drawing percentage origin; baseline menu
  changes do not affect the standard rectangle tool.

## Persistence and non-goals

- Hydrate and validate graded rectangles from the current per-symbol drawing
  setting, preserve them across restart, and retain them in chart snapshots.
- Persist template updates through the database-backed settings flow.
- Do not add a new database table or Rust command: existing settings and
  per-symbol drawings already use the established Tauri/SQLite persistence.
- Raw exchange tick-by-tick input, custom tick aggregation, and changes to the
  Simple Rectangle tool are out of scope.

## Verification

- Run `pnpm tsc --noEmit` and `pnpm build`.
- Run `cargo check --manifest-path src-tauri/Cargo.toml` because persistence
  relies on the existing Rust settings IPC and it must remain compatible.
- Manually verify pivot snapping, RBR/DBD placement, correction/engulf labels,
  fixed-height and width-only resizing, baseline context actions, per-symbol
  reload, and unchanged Simple Rectangle behavior.
