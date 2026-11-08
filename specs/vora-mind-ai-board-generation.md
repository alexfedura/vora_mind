# Vora Mind AI Board Generation

## Goal

Generate a structured Board graph from a user prompt with DeepSeek, stream
generation progress to the UI, and insert a validated, automatically laid-out
graph into the active canvas.

## Backend

- Store the DeepSeek API key in the active workspace's existing settings file
  under `vora_deepseek_api_key`; do not expose a command for reading it back.
- Send prompts from Rust to DeepSeek's chat-completions API using a streaming
  JSON-object response.
- Parse SSE `data:` events, emit text deltas as `deepseek-chunk`, and emit the
  completed JSON string as `deepseek-done`.
- Surface request, HTTP, stream, settings, and event-emission failures to the
  invoking frontend command.
- The settings file is workspace-local JSON and is not encrypted by the
  existing settings persistence mechanism.

## Frontend

- Provide a canvas toolbar action and prompt modal with key setup, streamed
  response preview, loading state, and actionable errors.
- Validate the generated graph before insertion: supported node types,
  unique IDs, valid edge endpoints, non-empty bounded node count, and bounded
  edge/content sizes.
- Arrange generated cards and text nodes with Dagre, center the graph in the
  visible canvas, add arrowed edges, and frame the resulting graph.
- Add generated cards to the existing card store and canvas state so standard
  Board persistence and undo work as for other canvas content.

## Generated JSON Contract

```json
{
  "nodes": [
    { "id": "idea-1", "type": "voraCard", "title": "Idea", "content": "Details" }
  ],
  "edges": [
    { "source": "idea-1", "target": "idea-2", "label": "relates to" }
  ]
}
```

Supported node types are `voraCard` and `voraText`. Edge labels are optional.

## Verification

- Type-check and production-build the frontend.
- Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Manually verify key setup, streamed generation, graph insertion/layout, and
  Board persistence with a valid DeepSeek API key.
