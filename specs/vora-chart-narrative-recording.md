# Vora Chart Narrative Recording

## Goal

Extend chart snapshots with optional drawing narratives that can be replayed on
the existing chart. Standard snapshots keep their current behavior and stored
shape.

## Data Contract

- `ActiveScale` stores the chart's canonical horizontal and vertical zoom offsets
  (`x_off_milli`, `y_off_milli`) as `x` and `y`, plus the active M level as `s`.
- A `FULL_VIDEO` narrative is a sequence of timestamped full drawing states and
  their active scale.
- A `COMPACT_STEPS` narrative stores the final state of each added or modified
  drawing after a 200 ms quiet period. Deletions are omitted.
- Narrative properties on `VoraSnapshot` are optional so existing snapshots
  remain readable. Narrative logs use the existing IndexedDB object store.

## Recording Flow

- Clicking the recording toolbar control starts full-state capture immediately;
  clicking it again stops recording and opens the save dialog.
- The save dialog offers standard snapshot, full narrative, and compact-step
  output modes. Full mode samples the latest chart state every 50 ms.
- Compact steps are derived from the full recording log by detecting drawing
  states that remain unchanged for 200 ms. Deletions are omitted.
- Save includes only the selected output: current drawings for standard, the
  full timeline for full narrative, or derived steps for compact output. The
  in-memory full log is cleared after a successful save.
- Drawing tools remain unchanged; recording observes the chart page's drawings
  state.

## Playback Flow

- The snapshot list opens narrative snapshots through the existing chart restore
  route with a playback query flag.
- After restoring the snapshot, the chart page opens `NarrativePlayback` and
  passes it controlled drawing and scale callbacks.
- Full mode plays recorded states at 0.5x, 1x, or 2x. Compact mode appends one
  committed drawing per step and can reset to the initial snapshot drawings.
- The in-chart player places its seek bar above the transport controls and
  persists independent speed/counter font sizes through Policy Panel settings,
  defaulting both to 20 px.
- The player can switch between Standard, Compact, and Full display modes.
  Compact can be derived from a full log; Full is unavailable for snapshots that
  only contain compact steps, avoiding duplicate narrative storage.
- Closing playback leaves the chart in its current state and removes the
  playback query flag.

## Verification

- Run `pnpm tsc --noEmit` and `cargo check --manifest-path src-tauri/Cargo.toml`
  as required by `AGENTS.md`.
- Confirm standard snapshots remain buildable and narrative fields flow through
  the builder and IndexedDB storage.