# VORA Phase 3.9 Tauri Command Inventory

## Counts

- Rust `#[tauri::command]` attributes: 333
- Unique Rust command names: 333
- Unique direct frontend invoke names: 49
- Direct invoke call strings: 49
- Commands with no direct frontend invoke: 284

No command was classified as unused solely from the absence of a direct import. Commands without direct calls are marked `NEEDS_DOMAIN_REVIEW`.

## Machine artifact

The complete inventory is stored in [VORA_PHASE_3_9_TAURI_COMMANDS.json](./VORA_PHASE_3_9_TAURI_COMMANDS.json).

Each entry records:

- command name
- implementation file
- inferred Rust module
- direct frontend callers
- migration status
- classification basis

## Classification

- `NOT_MIGRATED`: direct frontend caller exists and no equivalent Go capability was evidenced.
- `NEEDS_DOMAIN_REVIEW`: no direct frontend invoke string was found; indirect callers, events, plugins, binaries, or runtime registries may still exist.
- `MIGRATED`, `PARTIALLY_MIGRATED`, `DUPLICATE`, and `DEPRECATED` are not assigned until implementation and test evidence is available.

## Direct caller examples

The direct frontend inventory includes project-adjacent and legacy product operations such as board deletion, card versioning, snapshot operations, settings, media position/duration, research lineage, market alerts/templates, and Trace graph saving.

## Major implementation modules

The 333 commands are distributed across mind, research, source, experiment, analysis, model, learning, knowledge transfer, market, and Forge modules. See the JSON inventory for exact file mapping.
