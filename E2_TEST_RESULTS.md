# E2 Test Results

| Test | Command | Result |
|---|---|---|
| Electron security unit tests | `pnpm test:electron` | PASS, 3/3 |
| Frontend TypeScript | `pnpm tsc --noEmit` | PASS |
| Rust check | `cargo check --manifest-path src-tauri/Cargo.toml` | PASS with 2 existing warnings |
| Development startup | `pnpm electron:dev` with smoke | PASS |
| Unpacked packaged startup | C-drive Vora Mind.exe with smoke | PASS |
| Process cleanup | CIM process check | PASS |
| Single-instance lock | Packaged executable two-instance test | PASS |
| NSIS build | electron-builder NSIS x64 | PASS |
| NSIS installed startup | Silent installer + installed executable | NOT RUN / installer exit 1223 |
| Azure signing | No credentials/CI evidence | BLOCKED |
| electron-updater round trip | No update endpoint or signed old/new pair | BLOCKED |
