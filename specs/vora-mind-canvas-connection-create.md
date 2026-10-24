# Vora Mind Canvas Connection Creation

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | cards | text`

## Goal

Create and connect a new card or text node directly from a card or text node's orange connection handles.

## Behavior

- Cards and text nodes expose orange connection handles at the midpoint of all four sides.
- Card toolbars and orange connection handles remain hidden on hover and appear only while the node is selected.
- Dragging from any orange handle and releasing over empty canvas opens a nearby menu with `Card` and `Text` choices.
- Choosing `Card` or `Text` creates that node at the release position and connects it to the originating handle.
- A newly created card has its title ready to edit; a newly created text node receives focus in its editor.
- Connections retain their source/target handle IDs and persist with the active Board canvas.
- Existing direct connections between nodes and canvas interactions remain available.
- Dismissing the menu creates no node or edge.

## Validation

- Run `pnpm tsc --noEmit`, `pnpm build`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
