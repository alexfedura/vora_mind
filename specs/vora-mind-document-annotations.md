# Vora Mind Document Annotations

> **Status**: Approved for implementation
> **Scope**: `canvas | db | store | layout`

## Goal

Import PDF, DOCX, and Markdown documents into application-managed storage,
capture durable text highlights and notes, and promote highlights into
traceable Library cards.

## Persistence

- Copy imported documents into the application data directory and store
  document metadata in SQLite.
- Store highlights with document ID, selected text, optional page, JSON
  coordinates or text offsets, color, note, optional linked card, and timestamp.
- Validate document types, selection coordinates, status of referenced rows,
  and card ownership at the Rust command boundary.
- Promoting a highlight creates a normal Library card with source document and
  highlight IDs in its metadata. Repeated promotion returns the existing card.

## Frontend

- A Document Workspace lists imported documents and renders PDF, DOCX, and
  Markdown with text selection and persisted highlights.
- PDF files are loaded as bytes through Tauri IPC and rendered from a data
  buffer, never from a browser URL.
- Text documents store character offsets; PDF highlights store normalized
  page-relative rectangles.
- Promoted cards appear in the Library immediately and can navigate back to
  their source document and highlight.

## Validation

- Test import type/path validation, highlight persistence, document lookup,
  promotion, repeated promotion, and source metadata.
- Run `pnpm tsc --noEmit` and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
