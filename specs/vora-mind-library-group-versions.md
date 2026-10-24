# Vora Mind Library-Owned Schematic Boards

> **Status**: APPROVED FOR IMPLEMENTATION
> **Target Scope**: `canvas | library | db | tauri | types`

## Intent

Treat each Library item group as an independent Mind board. Open and edit the
schematic on the existing Mind canvas, while keeping Library-owned boards out
of the normal My Desk list.

## Confirmed behavior

- Each Library item group owns one persistent board in the existing
  `mind_boards` model. It opens from the Library group detail and is not listed
  with ordinary My Desk boards.
- The existing Mind canvas and its tools, node editors, selection,
  connections, undo/redo, and zoom are used to edit the Library board.
- Opening a saved Library version loads that exact schematic into an isolated
  editing session and never alters the active My Desk board.
- Changes remain drafts until the user explicitly enters a new three-part
  numeric version and commits it. Autosave, Board snapshots, card writes, and
  other persistence paths must not store an uncommitted Library draft.
- Closing without committing discards the draft and restores the previous My
  Desk canvas, selection, and viewport.
- Committing appends an immutable Library version, updates the group's current
  schematic, and synchronizes the owned Board's active version in one
  database transaction. Duplicate versions are rejected.
- Opening an older version and committing it creates a new version; it never
  overwrites a historical snapshot.
- Existing groups are migrated to Library-owned boards while retaining their
  current and historical Library snapshots.

## Data and persistence

- Add a nullable `library_item_group_id` ownership field to `mind_boards`, with
  a unique constraint for Library-owned boards. Ordinary Boards keep this
  field null.
- Create an owned board and initial Board version when a Library group is
  created. Backfill owned boards for existing groups during database
  initialization.
- Keep Library group version history as the Library version index and mirror
  each committed schematic into the owned Board's immutable
  `mind_board_versions`.
- Filter Library-owned boards from `get_all_boards` and ordinary Board
  management. Owned boards and their versions remain included in database
  backup/restore.
- Use the existing typed Library group snapshot and Board version snapshot
  models. Editing uses the full application canvas, not a temporary Board.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Rust tests cover owned-board creation, migration of existing groups,
  Library-only filtering, synchronized version commits, duplicate rejection,
  and preservation of previous versions.
- Verify opening, editing, committing, and discarding a Library board never
  changes the active My Desk board or persists an uncommitted draft.
