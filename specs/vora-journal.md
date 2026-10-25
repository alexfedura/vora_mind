# VORA Journal

> **Status**: Implemented
> **Target Scope**: `layout | store | tauri | db | types`

## Intent

Add a daily journal to the left navigation. Each entry is keyed by its local calendar date, displays that date as a non-editable heading, and stores one unrestricted description body. The interface follows the VORA dark and purple/indigo palette rather than the reference app's colors.

## User Experience

- Selecting **Journal** opens a dedicated workspace page without canvas controls.
- A Home button in the Journal header returns directly to the main canvas.
- A fixed shortcut in the upper-left canvas toolbar opens the Journal from Home.
  While the Journal is open, the same shortcut switches back to Home.
- The default view opens today's entry. The selected date and weekday are shown in the top navigation bar, replacing the separate Workspace/Journal title block; there is no editable title field.
- Users can move to adjacent days, choose a date, or open a month view.
- The month view displays one card per day, with saved text previews, in a four-column date grid; selecting a card opens that day's entry.
- The description is Markdown, multiline, and has no application-level length limit. The editor provides live formatting for headings, lists, emphasis, links, and code while preserving the Markdown source in the database.
- Edits are saved automatically and remain available after restarting the app.

## Data and Boundaries

- Store one row per local date in SQLite: ISO date (`YYYY-MM-DD`) primary key, unrestricted text content, and creation/update timestamps.
- Frontend contract and Rust serialized model use matching camelCase fields.
- SQL, commands, and persistence stay in `src-tauri/src/mind/journal.rs`; Tauri commands are registered in `lib.rs`; typed calls are exposed through `dbService` and Zustand.
- The frontend cache is not the durable source of truth. Database errors are surfaced in the Journal UI.

## Acceptance Criteria

- Journal is selectable from the left menu and active styling uses VORA purple/indigo.
- The Journal header Home button returns to the main canvas in one click.
- Today's entry opens without an editable title; its date is the only heading.
- Daily view has no divider below the date or decorative editor frame; the editor uses the available content width without a highlighted active-line background.
- On medium and wider screens, the daily editor occupies 95% of its content container, leaving balanced 2.5% side margins.
- Month/day navigation and previews reflect saved entries.
- Empty and long descriptions can be saved and reloaded.
- Typing Markdown list markers (for example, `- `) and pressing Enter continues lists; headings and other supported Markdown render in the live editor.
- TypeScript and Rust checks pass; focused Rust tests verify journal upsert, retrieval, and date scoping.
