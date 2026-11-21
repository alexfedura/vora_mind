# E2 Known Limitations

- The existing Tauri-dependent UI calls remain disabled and no compatibility shim was created.
- Worker remains external/manual and is not packaged or automatically started.
- Azure Trusted Signing is not configured.
- No real generic update endpoint or `latest.yml` publication exists.
- Auto-update round-trip is not tested.
- The local NSIS installer is unsigned and is not for public release.
- Deep links and tray are not implemented.
- MPV, database and Rust command migration are not implemented.
