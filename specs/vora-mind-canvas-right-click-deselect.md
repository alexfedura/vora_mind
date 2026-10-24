# Vora Mind Canvas Right-Click Deselect

## Behavior

- Right-clicking blank ReactFlow canvas space deselects selected cards and
  drawings, and clears any selected canvas edges.
- Right-clicking a node, edge, canvas toolbar, or other non-blank canvas content
  does not clear selection.
- Existing browser and ReactFlow context-menu behavior remains available.

## Verification

- Run `pnpm tsc --noEmit` and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
- Manually verify blank-space deselection and that right-clicking a selected
  card or drawing preserves its selection.
