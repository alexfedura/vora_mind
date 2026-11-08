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
- Every explicitly created version records a non-empty reason for the change. The reason is shown in version history and is part of the stored snapshot metadata.
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
- A user can create an independent Board branch from any version. The branch copies that version's full canvas snapshot, receives its own Board ID and initial `0.1.0` version, and records the source Board in `parent_board_id`.
- Branch snapshots remap node, card, edge, and in-snapshot references to new IDs so the new Board owns independent canvas objects while retaining the selected version's content and layout.
- A branch's initial version records the source version as its fork origin. Deleting the source Board preserves the branch and clears its now-unavailable ancestry references.

## Data model

- Add a Board-version table in SQLite with a stable ID, `board_id`, semantic version components, content-parent ID, fork-point ID, full snapshot payload, and creation timestamp.
- Enforce uniqueness of a displayed semantic version within a Board; reject conflicting version creation with an explicit error.
- Store snapshots in SQLite, not in frontend storage.
- Create initial version records transactionally when a Board is created.
- Migrate existing Boards by taking a consistent full-state baseline snapshot at `0.1.0`. Preserve existing Board IDs and current Board content.
- Keep the complete snapshot payload self-contained so restore does not depend on a later working state.
- Provide a read-only API to fetch any version's complete snapshot without changing the active version or working Board state.

## UI

- Show the active version beside each Board in Boards.
- Provide a per-Board history selector listing version, branch origin when applicable, and creation time.
- Selecting a history entry checks out that snapshot after saving pending changes to the currently active Board.
- Provide explicit Major, Minor, and Patch version-creation actions. The chosen increment starts from the checked-out version and stores the version against the complete current Board state.
- Show actionable errors if a snapshot cannot be saved/restored, or if the requested version label conflicts.
- Prompt for the reason before creating a Major, Minor, or Patch version.
- Allow two versions to be selected for a structural comparison. Compare nodes by stable ID and report additions, removals, and title/content/position modifications; compare links by ID and endpoints and report additions/removals.
- Show title and content line differences for modified cards, plus position changes and added/removed links.
- Show branch Boards indented in the Boards list and offer a Create Branch action beside each version-history entry. Ask for the new Board name before creating the branch, then open the new Board.

## Validation

- Rust tests cover baseline versions, version increments, branch ancestry/fork-point metadata, full snapshot restoration, Board isolation, and Board deletion cleanup.
- Rust tests cover creating an independent Board from a selected version, including remapped snapshot identities and parent-board metadata.
- Frontend checks: `pnpm tsc --noEmit`, `pnpm build`.
- Backend check: `cargo check --manifest-path src-tauri/Cargo.toml`.
