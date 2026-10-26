# VORA Questions Board

> **Status**: Implemented
> **Target Scope**: `layout | store | tauri | db | types`

## Intent

Add a Questions Board directly below Journal in the left navigation. It behaves
like a Journal workspace, but stores individually selectable questions instead
of date-based entries.

## User Experience

- Selecting **Questions Board** opens a dedicated workspace without canvas
  controls.
- The board lists question cards. Each card shows a short title and a preview
  of the full question.
- Users can create a question with a required title and description, select a
  card to open and edit its title and Markdown description, and return to the
  card list.
- Changes persist automatically and remain available after restarting the app.
- Use the existing VORA dark and purple/indigo visual language.

## Data and Boundaries

- Persist each question in SQLite with a stable ID, title, description, and
  creation/update timestamps.
- TypeScript and Rust serialized models use matching camelCase fields.
- SQL and persistence belong to `src-tauri/src/mind/questions.rs`; commands are
  registered in `lib.rs`; typed calls flow through `dbService` and Zustand.
- Database errors must be visible in the Questions Board UI.

## Acceptance Criteria

- Questions Board appears immediately below Journal and has active navigation
  styling.
- Users can create, list, open, edit, and delete persisted questions.
- Selecting a question opens its editor; returning restores the list.
- Empty descriptions and long question text can be saved and reloaded.
- TypeScript and Rust checks pass; focused Rust tests cover persistence and
  CRUD behavior.
