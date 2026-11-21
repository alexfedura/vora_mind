# E2.3 Electron Runtime Integration

## Scope
- Detect Electron through a typed Preload API.
- Expose app version, platform, architecture and package mode.
- Render a small runtime status indicator in the existing UI.
- Preserve Tauri runtime behavior when Electron is not present.
- Disable Electron-incompatible Tauri updater and drag-drop listeners.

## Exclusions
- No Tauri command migration.
- No database access.
- No worker, MPV or native domain migration.
- No UI redesign.
