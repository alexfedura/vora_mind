# Vora Mind Canvas Layer Management

## Goal

Let users raise or lower the clicked canvas node's visual stacking order from
the node context menu, with the result surviving autosave and board reload.

## Behavior

- The node context menu offers **Bring to Front** and **Send to Back**.
- Each action targets the node that opened the menu, even if a multi-selection
  was already active.
- Bringing forward assigns one more than the maximum current node `zIndex`;
  sending backward assigns one less than the minimum current node `zIndex`.
- Layer changes are recorded in canvas undo history and the autosave signature.

## Persistence

- Serialize each node's optional `zIndex` in the frontend node payload.
- Accept and store it as an optional Rust `NodeInput.z_index`.
- Write it to checkpoint and delta placement rows, defaulting missing values to
  zero for older clients and existing snapshots.
- Restore the placement z-index into the React Flow node when applying a
  semantic snapshot.

## Scope

This change affects semantic board snapshots and autosave. It does not add
layer controls to node floating toolbars or change the ordering of edges.
