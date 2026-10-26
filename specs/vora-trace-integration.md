# Vora Trace Integration

> **Status**: IMPLEMENTING
> **Target Scope**: `layout | types | services | tauri | config`

## 1. Intent

Integrate the existing Vora Trace decision-graph application as a first-class Vora Mind sidebar section without losing its graph, decision, ecosystem, Git history, import, or export workflows.

## 2. Integration Contract

- Keep the Vora Trace React UI, internal project sidebar, graph, and dialogs intact inside a dedicated Vora Mind workspace page.
- Reuse the existing Vora Mind Tauri window, router, and application lifecycle; do not launch a second app or Vite server.
- Keep Trace types in the shared domain type module, its IPC calls in the permitted frontend services layer, and its independent Zustand store in its feature module.
- Namespace Trace Rust commands and store Trace files under the Vora Mind application data directory, separate from Vora Mind's SQLite database.
- On first launch, copy the existing Trace project registry and `.vora/trace` data from `D:\workspace\vora_trace` into Vora Mind AppData. Never modify or remove the source project or its data. Once migrated, use only the AppData copy.
- Keep Trace styling scoped to its page so its original app-level styles do not alter other Vora Mind pages.

## 3. Native Data and IPC

Trace retains its existing JSON graph and Markdown decision format. The migration copies `trace.yaml`, `graph.json`, and decision Markdown files if they exist and creates an idempotent migration marker. A missing source directory must not block Vora Mind startup; a missing or invalid Trace graph must produce a visible Trace-page error rather than a success-shaped empty graph.

Register Trace's graph load/save, decision Markdown, ecosystem scan, Git log/file history, recent decisions, and JSON import/export commands with the existing Vora Mind Tauri builder. Use the existing dialog plugin for file pickers; no second Tauri app or filesystem/shell plugin is required.

## 4. Verification

- TypeScript and production frontend build succeed.
- Rust check succeeds and Trace command tests cover data-path selection and missing-data behavior.
- Existing Trace data remains untouched while the migrated AppData copy is readable and writable.
- Vora Mind routes and the Trace page render without Trace global styles leaking to other pages.
- Bump the synchronized Vora Mind version, create a release tag, and push after checks pass.
