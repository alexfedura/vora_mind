# VORA Journal

> **Status**: Planned
> **Target Scope**: `layout | store | tauri | db | types`

## Intent

Add a daily journal to the left navigation. Each entry is keyed by its local calendar date, displays that date as a non-editable heading, and stores one unrestricted description body. The interface follows the VORA dark and purple/indigo palette rather than the reference app's colors.

## User Experience

- Selecting **Journal** opens a dedicated workspace page without canvas controls.
- The default view opens today's entry. Its heading is the date, with the weekday shown as supporting text; there is no editable title field.
- Users can move to adjacent days, choose a date, or open a month view.
- The month view displays one card per day, with saved text previews, in a four-column date grid; selecting a card opens that day's entry.
- The description is plain text, multiline, and has no application-level length limit.
- Edits are saved automatically and remain available after restarting the app.

## Data and Boundaries

- Store one row per local date in SQLite: ISO date (`YYYY-MM-DD`) primary key, unrestricted text content, and creation/update timestamps.
- Frontend contract and Rust serialized model use matching camelCase fields.
- SQL, commands, and persistence stay in `src-tauri/src/mind/journal.rs`; Tauri commands are registered in `lib.rs`; typed calls are exposed through `dbService` and Zustand.
- The frontend cache is not the durable source of truth. Database errors are surfaced in the Journal UI.

## Acceptance Criteria

- Journal is selectable from the left menu and active styling uses VORA purple/indigo.
- Today's entry opens without an editable title; its date is the only heading.
- Month/day navigation and previews reflect saved entries.
- Empty and long descriptions can be saved and reloaded.
- TypeScript and Rust checks pass; focused Rust tests verify journal upsert, retrieval, and date scoping.
