# Vora Chart Drawing Undo and Redo

> **Status**: APPROVED
> **Target Scope**: `chart`

## Intent

Undo and redo must restore complete chart drawing states, including each
drawing's position, geometry, color, presentation settings, and existence.
Snapshots must be independent of later changes to nested drawing data.

## Keyboard Behavior

- `Ctrl+Z` and `Command+Z` undo the latest drawing change.
- The same shortcuts work when the active keyboard layout produces Persian `ز`
  or another character for the physical Z key.
- `Ctrl+Shift+Z` and `Command+Shift+Z` redo; existing `Ctrl+Y` redo remains.
- Shortcuts do not intercept typing in editable fields.

## History Behavior

- Existing drawing operations keep their snapshot-before-mutation behavior.
- Undo restores the prior complete drawing list; redo restores the state that
  was active immediately before undo.
- Existing bounded in-memory history and symbol-change clearing remain intact.

## Verification

- Run `pnpm tsc --noEmit`.
- Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Manually verify create, move, resize, recolor, undo, redo, and Persian-layout
  `Ctrl+ز` / `Ctrl+Shift+ز`.
