# Vora Mind Multi-Links

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `canvas | cards | db`

## Goal

Allow multiple directed links between precise text ranges in card titles and card bodies, including links between cards on different Boards. A range can link to another range or to the whole card when no target range is selected.

## Confirmed behavior

- Create a link by selecting a source range and opening the link picker.
- The picker selects a destination Board, card, field (`title` or `content`), and optionally an exact destination range.
- A source range may have multiple outgoing links; a destination range may have multiple incoming links.
- Links are stored as durable records in SQLite, not embedded in displayed text or canvas edges.
- Anchors use CodeMirror-compatible UTF-16 offsets, with half-open `[start, end)` ranges.
- A text edit strictly before an anchor shifts it by the edit's length delta.
- An edit intersecting an anchor, including an insertion strictly inside it, deletes that link instead of preserving a stale or changed phrase. Edits exactly at range boundaries do not change the linked phrase.
- Navigation opens the destination card and selects/scrolls to the target range when one exists.
- Cards expose incoming links (backlinks) visibly without requiring the section to be expanded; one click opens the source Board/card and selects the exact source field and text range.
- Existing canvas graph edges remain visual node-to-node connections and are not repurposed as text links.

## Persistence and lifecycle

- Store source and target board IDs, card IDs, fields, offsets, and creation/update timestamps for each link.
- Validate that each endpoint card exists on its declared Board and that offsets are non-empty, in bounds, and valid UTF-16 boundaries for the current text.
- Create, delete, list outgoing links, and list incoming links through typed Tauri commands and frontend service methods.
- Card text saves and anchor maintenance run transactionally. For each changed field, pass the old-coordinate text edit ranges to the backend; it shifts unaffected anchors and deletes links whose source or target anchor intersects an edit.
- Deleting a card or Board removes links that refer to endpoints owned by it.
- Links are durable workspace relationships, independent from canvas snapshots and board-version history.

## UI and editor behavior

- In CodeMirror, create links through a contextual action and `Ctrl+K` / `Cmd+K`; the selected Markdown text remains unchanged.
- For card titles, a link action uses the selected title substring.
- Right-clicking a non-empty selection in either a card title or its content opens the card context menu with `Add to Library` followed by `Add Link`. `Add Link` opens the text-link picker for that exact selected range; `Ctrl+K` / `Cmd+K` remains supported.
- The text-link picker uses the Vora dark surfaces, borders, and accent colors, and offers `Projects` and `Library` tabs. Projects lets the user choose a Board and card; Library lets the user search and choose active cards across Boards. Both tabs allow choosing the destination field and optionally selecting an exact target range.
- The picker previews the destination field as selectable text so a precise target range can be selected, including a single character.
- Link marks are interactive and visually distinct. Clicking a marked title or content phrase with one destination opens the target directly; when a source range has multiple destinations, show a chooser. Keep the outgoing-link list as an alternate navigation surface.
- Newly created links refresh both endpoint cards. Provide a visible card-level incoming-links list; one click opens the source Board/card and selects the exact source field and range, so every link has a working return path.
- Opening another card to navigate must save/close the current editor first and preserve existing Canvas card and Board behavior.

## Validation

- Rust tests cover cross-Board links, multiple links for identical source/target ranges, range validation, anchor shifts, overlap deletion, and card/Board deletion cleanup.
- Frontend tests cover selecting precise ranges, link marks, navigation, and incoming/outgoing link lists.
- Run `pnpm tsc --noEmit`, `pnpm build`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
