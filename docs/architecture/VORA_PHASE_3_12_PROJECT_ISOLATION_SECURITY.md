# VORA Phase 3.12 Project Isolation Security

## Automated evidence

Passed:

- Project filesystem isolation and reopen
- Credential project scope cannot access another project
- Credential discovery scope rejection
- MCP project context and outside-root rejection
- HTTP project open/context/delete approval
- Windows junction escape rejection
- Symlink/broken-symlink rejection

## Live evidence

- Project A/B root switching was observed.
- Active project changed to A, B, A, B.
- Restart preserved Project B as active.
- Root A and Root B remained distinct.
- Disposable projects and roots were removed after approval.
