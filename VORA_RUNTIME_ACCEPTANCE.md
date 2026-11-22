# Vora Mind Runtime Acceptance

## Procedure

1. Set `VORA_GO_DATA_DIR` to an isolated writable directory.
2. Build the Wails frontend:
   `cd go-app/frontend && pnpm build`
3. Build the Windows application:
   `cd go-app && wails3 build`
4. Launch the generated executable and keep it running for at least 30 seconds.
5. Verify the React interface renders and project/capability/workflow sections load.
6. Create, list, and update a project.
7. Enable and stop the loopback API.
8. Close the main window and confirm the process exits.
9. Confirm no port outside `127.0.0.1` is listening.

## Phase 3 evidence

- Wails frontend production build passed.
- Wails binding generation passed.
- Wails Windows executable build passed.
- The executable remained running for a 30-second smoke window.
- An isolated SQLite database was created and migrations completed.
- The embedded frontend asset was present.
- Process shutdown exited cleanly.

Blocked/limited: visual interface observation and GUI interaction were not
performed. No project creation, capability explorer, workflow controls, API
status, or closing behavior was visually verified. This report claims only
process-level runtime acceptance.
