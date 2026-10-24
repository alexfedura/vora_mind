# Vora Mind Inline Markdown Card Editor

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | markdown-editor`

## Goal

Use the Markdown editor inline in Canvas cards, without adding a route or a separate note page.

## Behavior

- Clicking or focusing a card's rendered Markdown content opens the inline Markdown editor for that card. The read-only preview and editor use the same typography and card styling.
- In the expanded card view, writing Persian or other RTL content uses the same text direction, alignment, and typography as the displayed content, so entering and leaving edit mode does not visibly shift the text.
- Typing `- ` at the start of a Markdown line displays a live bullet. Each list item's direction and alignment follow its own text, so Persian bullets appear on the right and English bullets on the left, even in a mixed-language document. Tab indents the current list item by two spaces to create a nested list, and Shift+Tab removes one indentation level.
- The editor receives the card ID and loads/saves through the existing Canvas card API and active board.
- Switching to another card flushes pending edits before loading the next card.
- Clicking outside the editor saves pending edits before returning to the rendered Markdown preview. Changes are also saved automatically while typing; a manual save/close button is not required.
- Keep the title field and non-editor card behavior intact.
- Do not add document filesystem commands, filesystem permissions, or localStorage persistence.

## Implementation boundaries

- The editor component owns and destroys its CodeMirror `EditorView`.
- The document store continues to use `dbService.getAllCards` and `dbService.saveCardDirect`.
- The active Canvas card remains the durable source of truth in SQLite.
- Markdown styles remain imported once by the application entry point.

## Validation

- Verify Markdown editor integration syntax and CSS parsing.
- Run the frontend build and Rust check when dependencies are available.
