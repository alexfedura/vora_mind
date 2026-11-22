# VORA Electron Retirement Matrix

## Relevant components

- `electron/main.ts`: window lifecycle, navigation security, IPC registration, updater, and sidecar startup.
- `electron/preload.ts`: context-isolated bridge.
- `electron/security.ts`: sender and URL authorization.
- `electron/sidecar-manager.ts`: Rust sidecar process protocol.
- `electron/sidecar-ipc.ts`: IPC commands for health, engine, and smoke.
- `electron/updater.ts` and `electron/github-updater.ts`: update handling.
- `electron/logger.ts`: Electron logging.
- `electron/dev-start.cjs`: development startup.
- `electron/sidecar/`: Rust sidecar source/build configuration.
- `package.json`: Electron scripts/dependencies and NSIS packaging.
- `.github/workflows/electron-release.yml`: Electron release pipeline.
- `dist-electron/`, `artifacts/electron/`: generated build and packaging output.

## Classification

| Component | Classification | Replacement |
|---|---|---|
| BrowserWindow lifecycle | active legacy runtime | Wails v3 window and app lifecycle |
| contextBridge/preload IPC | active legacy runtime | Wails generated bindings |
| Rust sidecar protocol | active legacy integration | Go shared runtime or intentional removal |
| Electron updater | active release/runtime dependency | Wails/native Go updater integration |
| Navigation/CSP security | active legacy runtime | Wails WebView policy plus Go trust boundaries |
| Electron tests | test-only evidence | Wails/Electron parity or removal after retirement |
| Electron release workflow | packaging dependency | Wails packaging/release workflow |
| Electron Rust sidecar | legacy implementation | not to be reused until behavior is mapped to Go |

## Removal prerequisites

1. Main React product functionality has a Go/Wails implementation or explicit removal decision.
2. Wails has equivalent native dialogs, lifecycle, updater, and window behavior.
3. Rust sidecar capabilities are represented in Go or retired.
4. Release signing, updater rollback, and installer validation are established for Wails.
5. A final acceptance matrix proves parity.

Removal is not safe merely because Electron source is currently isolated.
