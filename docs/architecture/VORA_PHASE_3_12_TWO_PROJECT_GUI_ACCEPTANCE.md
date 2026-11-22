# VORA Phase 3.12 Two-Project GUI Acceptance

## Live Wails evidence

- Project A created: `Phase312-Project-A`
- Root A: `C:\Users\HELIX\AppData\Roaming\Vora Mind\phase312-acceptance\Root-A`
- Project B created: `Phase312-Project-B`
- Root B: `C:\Users\HELIX\AppData\Roaming\Vora Mind\phase312-acceptance\Root-B`

Observed:

- A opened and showed active context A.
- A root displayed.
- B opened and showed active context B.
- B root displayed after switching.
- Switching A→B→A→B changed active project details correctly.
- Both roots remained listed after switching.
- Restart restored project list and root list.
- Persisted active project was Project B.
- Reopening B after restart showed Project B context.

## UI fix

The workspace-root field is now synchronized from the active project and returned root list in `go-app/frontend/src/App.tsx`.
