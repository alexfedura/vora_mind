# Vora Mind Board Versioning

> **Status**: Approved for implementation
> **Scope**: `boards | db | store`

## Goal

Give every Board an immutable, Git-like history that a user explicitly creates and can inspect or check out, without creating a history entry for every canvas autosave.

## Confirmed behavior

- Every newly created Board starts at version `0.1.0`.
- Release numbers stay three-part semantic versions. Branch ancestry is shown as a second three-part version after a dash (for example, `0.2.1 - 0.3.0`), not as a fourth dot-separated numeric component.
- Existing Boards receive one baseline version `0.1.0` containing their current state.
- A version is a full Board snapshot: cards and their content, canvas nodes and shapes, positions, and connections.
- Creating a version is an explicit user action. Ordinary edits and autosaves do not add version-history entries.
- The Board list displays the active version next to each Board and offers a version-history dropdown.
- The dropdown lists all immutable versions. Selecting one checks it out into the Board's working state; the user can continue editing from there.
- A new version can be created by explicitly incrementing major, minor, or patch. Versions cannot be deleted or downgraded by creating a lower release.
- When a new version is created after checking out an older version, record both:
  - its content parent: the checked-out version;
  - its fork point: the Board's latest version that existed when the older version was checked out.
- Checking out an older version changes the working state, not the immutable history. Creating a release from that state increments the checked-out version and records the fork point, so it is not a downgrade.
- Display a branched version as `VERSION - FORK_POINT`, for example `0.2.2 - 0.3.0`. A version on the uninterrupted history displays only `VERSION`.
- Checking out a version does not mutate or delete any version snapshot. The current working Board remains autosaved through the existing Board persistence flow.
- Version snapshots are scoped to a Board and are deleted only when that Board is deleted.

## Data model

- Add a Board-version table in SQLite with a stable ID, `board_id`, semantic version components, content-parent ID, fork-point ID, full snapshot payload, and creation timestamp.
- Enforce uniqueness of a displayed semantic version within a Board; reject conflicting version creation with an explicit error.
- Store snapshots in SQLite, not in frontend storage.
- Create initial version records transactionally when a Board is created.
- Migrate existing Boards by taking a consistent full-state baseline snapshot at `0.1.0`. Preserve existing Board IDs and current Board content.
- Keep the complete snapshot payload self-contained so restore does not depend on a later working state.

## UI

- Show the active version beside each Board in Boards.
- Provide a per-Board history selector listing version, branch origin when applicable, and creation time.
- Selecting a history entry checks out that snapshot after saving pending changes to the currently active Board.
- Provide explicit Major, Minor, and Patch version-creation actions. The chosen increment starts from the checked-out version and stores the version against the complete current Board state.
- Show actionable errors if a snapshot cannot be saved/restored, or if the requested version label conflicts.

## Validation

- Rust tests cover baseline versions, version increments, branch ancestry/fork-point metadata, full snapshot restoration, Board isolation, and Board deletion cleanup.
- Frontend checks: `pnpm tsc --noEmit`, `pnpm build`.
- Backend check: `cargo check --manifest-path src-tauri/Cargo.toml`.
