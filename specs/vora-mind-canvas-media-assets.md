# Vora Mind File References and Annotations

> **Status**: Approved for implementation
> **Scope**: `mind | canvas | tauri | db`

## Goal

Use immutable file UUIDs as the shared identity for canvas media, player
references, document sources, and timestamped annotations. Imported files stay
at their original paths; Vora Mind stores metadata and annotations only.

## Persistence

- `vora_files` stores a UUID, original name, cached current path, file type,
  SHA-256, byte size, modified time, and creation time. Playback duration,
  last position, and last-used time remain file metadata.
- `mind_media_slices.file_id` references `vora_files.id` and retains its title,
  start/end seconds, note, and timestamps.
- `documents.file_id` references `vora_files.id`. Existing document IDs and
  highlight relationships remain stable.
- Startup migrates old media sources and slices, and adds file references for
  existing document copies without deleting any existing files or annotations.
- New imports deduplicate the same canonical path and content. They do not
  copy files into application data.

## File identity and relocation

- `import_file_reference(path)` validates and fingerprints a regular supported
  file, stores its original path and returns its UUID.
- SHA-256 is calculated from file bytes. Recursive relocation checks candidate
  byte size before hashing and updates the cached path only on an exact hash
  match. Legacy entries without a readable source hash may use size plus
  modified time.
- `resolve_missing_file(file_id, search_dir)` returns a clear error if there
  is no match; it never silently chooses a same-name file.
- Current file types include image, audio, video, PDF, DOCX, and Markdown.

## Behavior

- Canvas media imports store `file_id` and the current original path in node
  data. They render the referenced file directly. A note indicator reflects
  saved time slices; double-click opens the player by UUID.
- The player saves and loads timestamped notes by `file_id`. Missing files can
  be located recursively from a user-selected directory.
- Canvas file access is scoped to the explicitly referenced path rather than
  granting the WebView unrestricted filesystem access.
- The Library's Media & Files view lists references and their slices, reports
  unavailable paths, and opens a slice at its saved start time.
- Canvas video previews use single-click playback and double-click to open the
  Vora Player. Saved slices can create Board cards carrying
  `source_file_id`, `start_time`, and `end_time` in card metadata.
- Card metadata is returned by card loading and retained in Board version
  snapshots and checkouts. A card's source badge and Library media note both
  reopen the source at the recorded timestamp.
- The Media Notes view is scoped to media file references used by the active
  Board, while preserving the original-file and missing-file workflows.
- PDF, DOCX, and Markdown imports reference the original file. Existing
  document/highlight IDs and viewing behavior are preserved.

## Verification

- `pnpm tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted Rust tests cover fingerprinting, relocation, and migration.
- Confirm imports retain original paths, annotations round-trip by file UUID,
  and relocating a renamed file updates its cached path.
