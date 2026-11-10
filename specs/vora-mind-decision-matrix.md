# Vora Mind Decision Matrix

## Goal

Add a dedicated canvas node for comparing options against weighted criteria.

## Data and persistence

- Store the matrix title, criteria columns and weights, and option rows with
  per-criterion numeric scores in typed `VoraMatrixData`.
- Persist the matrix in the existing React Flow node `data` JSON through
  normal Board snapshots and autosave. Do not add a separate database table.
- Map `voraMatrix` to the generic `canvas_node` object type.

## Interaction

- Register `voraMatrix` as a custom canvas node and provide a canvas toolbar
  action that inserts a default matrix near the current viewport center.
- Render a resizable dark table with editable title, criteria names and weights,
  option names, and scores from 1 to 10.
- Allow users to add criteria and options; calculate each row's total as the
  sum of score multiplied by criterion weight.
- Use React Flow `nodrag` and `nowheel` classes on inputs and controls so
  editing cells does not drag nodes or zoom the canvas.
- Expose the standard source/target handles on all four sides.

## Validation

- Run `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- Verify the toolbar action creates a matrix node and its values are stored in
  node data for normal canvas persistence.
