# Vora Mind Diagramming

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | types | spec`

## Goal

Extend the existing local-first Vora Mind canvas into a practical diagramming
workspace without replacing its Boards, cards, text, shapes, connections,
selection, history, or snapshot model.

This adds a native flowchart symbol set, consistent node styling and labels,
reusable diagram templates, and portable Board JSON. It uses the existing
canvas and persistence pipeline; it does not copy Miro's proprietary visuals
or require a Miro account.

## Confirmed scope

- Diagramming lives inside Vora Mind and the active Board.
- Include flowchart symbols, freeform shapes, text, grouping, templates, and
  import/export.
- Keep the feature local-first and single-user. Realtime collaboration,
  accounts, network sharing, and remote presence are out of scope.

## Behavior

- A diagram shape palette provides process, decision, start/end, input/output,
  document, database, and predefined freeform/rectangle/path tools.
- Placed diagram nodes use typed shape data, editable labels, existing
  resize/move/select/delete/duplicate/group/undo/redo behavior, and persist in
  Board snapshots without a new SQLite table.
- Shapes expose connection handles so diagrams can connect shape-to-shape as
  well as to existing cards and text nodes. Connections retain their endpoints
  through snapshots and Board changes, using the existing canvas connector
  styling.
- New and existing shapes share the current style controls and saved style
  templates. Defaults and custom styles remain local to the current
  installation.
- A small set of useful flowchart templates can be inserted into the active
  Board as editable nodes and connections. Insertion does not replace existing
  Board content.
- A Board can be exported to a versioned JSON file and imported from that
  format. Import validates the complete payload before changing canvas state,
  creates fresh node/edge/card identifiers, retains internal connections and
  shape data, and is one undoable operation. Malformed or unsupported files
  leave the current Board unchanged and surface a clear error.
- Existing Boards, saved snapshots, cards, links, Library groups, and
  non-diagram canvas behaviors remain backward-compatible.
- The diagram editor remains usable with RTL/Persian text and keyboard
  navigation in its controls.

## Non-goals

- Realtime collaboration, remote sharing, accounts, and presence.
- Exact recreation of Miro's UI, proprietary templates, or branded assets.
- Specialized UML, BPMN, cloud-provider, or engineering symbol catalogs.
- New SQLite tables, migrations, or backend services.
- Raster/PDF export in this iteration; the portable, editable Board JSON is
  the import/export contract.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Verify JSON import validation, ID remapping, and one-step undo for imports.
- Manually verify symbol creation, handle-based connections, templates,
  persistence, snapshot replay, RTL labels, and preservation of existing Board
  content.
