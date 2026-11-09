# Smart Library Item Dispatcher

## Goal

Distinguish standalone Library items from Library groups while preserving the existing snapshot, version-history, and Library Board workflows.

## Dispatch behavior

- One selected canvas node without `data.canvasGroupId` is saved as `item`.
- Multiple selected nodes are saved as `group`.
- One selected node with `data.canvasGroupId` expands to all nodes with that same ID and is saved as `group`.
- The snapshot includes only edges whose source and target are both included.

## Persistence and UI

- Store `item_type` (`item` or `group`) on `library_item_groups`; migrate existing records to `group`.
- Include the type in serialized snapshots and typed frontend/Rust Library models.
- Preserve the type when committing a new Library Board version.
- The create panel and Library list visibly distinguish item and group records.
- Existing one-node save flows explicitly identify whether the saved record is an item or a group.

## Verification

- Add or update Rust coverage for type persistence and existing-record migration defaults.
- Run `cargo check --manifest-path src-tauri/Cargo.toml` and `pnpm tsc --noEmit`.
