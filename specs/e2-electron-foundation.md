# E2 Electron Foundation

## Objective
Create a secure Windows x64 Electron shell around the existing React build without migrating Tauri commands, Rust, databases, MPV or worker behavior.

## Approved Scope
- Electron Main foundation and BrowserWindow lifecycle.
- Secure typed Preload API.
- Sender, frame and payload validation.
- Single-instance lock.
- Existing React build loading in development and production.
- Structured application logging.
- Windows x64 NSIS packaging.
- electron-updater generic/latest configuration hooks.
- Signing configuration placeholders without credentials.

## Exclusions
- Tauri removal or command migration.
- Tauri compatibility shims.
- Database access or migration.
- Rust or worker changes.
- MPV changes.
- Deep links or tray.
- Cross-platform packaging.
- UI or scientific workflow redesign.

## Security
- `contextIsolation: true`
- `nodeIntegration: false`
- `sandbox: true`
- Explicit navigation and popup restrictions.
- Allowlisted named Preload methods only.
- No raw IPC exposure.

## Verification
- TypeScript typecheck.
- Unit/integration tests for Preload and IPC.
- Development startup smoke test.
- Local packaged startup smoke test.
- No orphan Electron process.
