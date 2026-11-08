# Vora Mind Canvas Media Assets

> **Status**: Approved for implementation
> **Scope**: `mind | canvas | tauri | db`

## Goal

Import local image, audio, and video files as durable, movable, resizable nodes on a
Vora Mind Board. Imported files are copied into Vora Mind's application data
directory so canvas nodes do not depend on the original file remaining in
place. This feature is separate from Vora Player's path-based audio/video
source references.

## Behavior

- The canvas toolbar offers a multimedia file picker. Supported extensions
  are PNG, JPG, JPEG, GIF, WEBP, MP3, WAV, OGG, M4A, AAC, FLAC, MP4, WEBM,
  MOV, and M4V.
- Dropping a supported desktop file onto the canvas imports it and places a
  node at the drop point. Unsupported file types are ignored with an explicit
  user-visible error.
- The Rust import command validates a regular file and supported extension,
  copies it to a collision-resistant filename under
  `<app_data_dir>/assets/`, and returns its absolute destination path.
- Image nodes render the copied local asset using Tauri's asset protocol,
  preserve their aspect ratio, and can be resized. Audio and video nodes show
  browser media controls in styled, resizable nodes.
- Media node type and data are saved by the existing canvas snapshot path.
  Board export/import retains media data and node geometry.
- The existing media player remains path-based and is not changed by this
  canvas asset workflow.

## Persistence and local access

- `VoraMediaData` stores the media kind, copied absolute path, original
  filename, and optional dimensions in the ReactFlow node's `data`.
- Snapshot version content remains the durable source of canvas node data;
  no new database table is introduced.
- Tauri's asset protocol is enabled only for the app-data assets directory.
  The frontend uses `convertFileSrc` to generate WebView-readable URLs.
- Import errors are surfaced in the canvas UI; source files are never modified.

## Verification

- `pnpm tsc --noEmit`
- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Confirm media nodes autosave/restore and survive board export/import.
- Confirm supported toolbar import and native desktop drop create nodes at the
  requested canvas positions; unsupported extensions report an error.
