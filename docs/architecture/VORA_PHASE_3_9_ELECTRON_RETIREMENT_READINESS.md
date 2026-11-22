# VORA Phase 3.9 Electron Retirement Readiness

## Readiness: NOT READY

Electron remains active for runtime, sidecar, updater, packaging, and release.

## Blockers

- No complete Wails replacement for the main React/Rust product feature surface.
- Electron sidecar behavior is implemented as a separate process protocol.
- Electron updater and signing pipeline have no equivalent Wails-only release pipeline evidenced.
- Native dialog and window behavior for the main frontend require contract mapping.
- Electron-specific tests do not prove product capability parity.

## Go/Wails owner map

- Window lifecycle: Wails application/window APIs.
- IPC bridge: generated Wails bindings.
- Sidecar: Go service or durable workflow, unless behavior is intentionally removed.
- Updater: Go or Wails/native update service plus signing/release pipeline.
- Logging/security: Go/Wails policy and application audit.
- Installer: Wails packaging.
