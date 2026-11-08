# AI Board Generation

## Goal

Generate a structured graph from a user prompt using DeepSeek, stream progress
from the Rust backend, and insert a laid-out graph into the active canvas.

## Backend contract

- Persist the DeepSeek API key through the existing settings-file commands.
- Send a streaming chat-completions request from Rust; do not expose the key to
  the frontend after it is saved.
- Forward content deltas as `deepseek-chunk` events and the complete generated
  JSON as `deepseek-done`.
- Return transport, HTTP, stream, and parsing errors to the invoking frontend.

## Canvas contract

- Accept only card and text nodes and edges whose endpoints resolve to generated
  nodes.
- Assign application-owned IDs, lay out the graph with Dagre, then append nodes
  and edges to the active board using the existing flow state setters.
- Create cards through the existing app store so generated board content uses
  the regular persistence path.

## User experience

- Provide a canvas toolbar entry point and prompt modal with optional API-key
  setup, streaming progress, and surfaced errors.
- Keep the generation UI separate from the Forge coding-agent workflow.

## Validation

- Run `pnpm tsc --noEmit` and `cargo check --manifest-path
  src-tauri/Cargo.toml`.
- Verify request/response parsing and graph validation with focused tests where
  practical.
