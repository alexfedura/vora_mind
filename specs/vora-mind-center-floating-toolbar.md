# Vora Mind Center Floating Toolbar Shortcut

## Behavior

- Pressing the physical Backquote key centers the selected drawing or text
  toolbar on the current pointer position, independent of keyboard layout.
- The shortcut does not run while typing in an input, textarea, select, or
  contenteditable element.
- Toolbar centering accounts for canvas zoom and viewport zoom so its visible
  center aligns with the pointer.

## Verification

- Run `pnpm tsc --noEmit` and `cargo check --manifest-path
  src-tauri/Cargo.toml`.
- Verify centering on a selected shape and text node at multiple zoom levels
  with the English and Persian keyboard layouts.
