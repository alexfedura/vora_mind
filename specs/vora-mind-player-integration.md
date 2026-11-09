# Vora Mind Player Integration

> **Status**: APPROVED FOR IMPLEMENTATION
> **Scope**: `mind | canvas | tauri | db`

## Goal

Bring the existing Vora Player experience into Vora Mind as a native
workspace, using Vora Player's mpv-based playback engine. Let users create
time-range references to local audio/video files and open those exact ranges
from text on any Board.

## Confirmed requirements

- Integrate the existing project at `D:\workspace\vora_player`; do not create
  an unrelated browser-only player.
- The player workspace is part of Vora Mind. Selecting a time reference from
  any Board opens the workspace, resolves the same local source, and seeks to
  its stored time.
- Sources stay at their existing paths. Vora Mind does not copy, transcode,
  trim, overwrite, or otherwise modify the original media.
- Bundle the mpv runtime needed for standalone/offline playback. Do not bundle
  FFmpeg or expose clip-export/cutting actions in Vora Mind.
- Audio and video playback use mpv and retain source quality subject to the
  source format and available system decoding hardware.

## Behavior

- Vora Mind includes an in-app Player workspace with file selection,
  playback/pause, precise seeking, elapsed/duration display, playback speed,
  volume, fullscreen, keyboard controls, and the existing Vora Player slice
  list and editor.
- Newly opened media starts paused. A canvas video node hides native controls;
  a single left-click toggles inline playback, and a double left-click opens
  its original file in the Vora Player workspace.
- Open media using a native file picker. Record its normalized absolute path,
  file size, modification time, and media duration for source provenance.
  Missing or changed sources are reported explicitly; they are never silently
  substituted.
- A slice in Mind is an annotation containing source identity, in/out times,
  title, and optional note/reason. It is not a media file and does not imply
  that any frames or bytes were cut.
- Each saved slice offers a copyable Markdown link with a stable Vora media
  URI. Clicking the link from a card preview or Markdown editor on any Board
  opens the exact source, seeks to the slice start, and pauses at its end.
  Selecting a saved slice also activates its start/end playback range.
- Links preserve the full-precision time values, source path, and source
  fingerprint fields. The visible label includes the title and time range.
- Slice metadata is stored through Vora Mind's SQLite-backed media service,
  independently of the media file. Board snapshots and card text retain their
  references without storing video bytes.
- The Player workspace remembers the previously selected source and playback
  position but does not autoplay a source when an unrelated Board opens.
- Opening a cross-Board media link does not change the active Board or discard
  its canvas selection/state. A clear return-to-Board action is provided.
- MPV is bundled with Vora Mind for offline use. The installer will grow by
  approximately the MPV runtime size (about 120 MB, excluding compression).
- Build-time MPV assets are pinned in `vendor/mpv` and SHA-256 verified by
  `scripts/prepare_mpv.ps1`; generated unpacked resources stay out of Git.
  `pnpm prepare:mpv` prepares the runtime before Tauri development/builds.
- The integration must not call `slice_export`, `chapters_export`, or FFmpeg.
  No action in Vora Mind may alter, replace, trim, or rewrite the selected
  source media.
- Slice playback uses the original source path and MPV seeking. End-time
  enforcement pauses playback and seeks to the exact saved end position; no
  derivative or rendered clip is created.

## Non-goals

- Remote media hosting, account-based sharing, or cross-device file syncing.
- Creating derivative clips, transcoding, chapter export, or editing source
  files.
- Exact pixel-for-pixel reproduction of another product's interface.
- Replacing or modifying the standalone Vora Player application.

## Compatibility and persistence

- Use the current Vora Mind database/service/resolver path for all persisted
  media source and slice metadata; do not write beside or inside the source
  media file.
- Store native file paths as paths, not web URLs, and validate paths before
  playback. Keep media operations local-only.
- Add a safe migration for the new media tables; existing Boards, cards,
  snapshots, and text links remain unchanged.
- Keep media-link parsing strict and reject malformed paths, IDs, non-finite
  timestamps, negative times, and end times less than or equal to start times.
- Playback and link failures must be surfaced in the Player UI with an
  actionable message.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Rust tests cover media path/fingerprint validation and slice range validation.
- Verify opening audio and video, accurate seek from a link on another Board,
  persistence across restart, missing/changed source feedback, and return to
  the original Board.
- Verify newly opened media is paused, single-click toggles playback, and
  double-click enters/exits fullscreen.
- Verify canvas video controls are hidden, single-click toggles inline
  playback, and double-click opens the original path in Vora Player.
- Verify slice playback stops at the stored end time and leaves source bytes
  unchanged.
- Verify by code path and test that no clip-export/FFmpeg command is
  registered or invoked from Vora Mind.
