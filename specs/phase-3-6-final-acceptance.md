---
Status: IN_PROGRESS
Author: Copilot
Target Scope: Phase 3.6 final acceptance and remaining gaps closure
---

# Phase 3.6 Final Acceptance

## Intent

Complete the remaining Phase 3.5 gaps without architecture changes.

## Requirements

1. Verify Windows DPAPI through isolated Windows build/tagged tests and
   disclose whether real user-context execution was available.
2. Persist explicit project workspace roots and recover them after startup,
   restore, and reopen. Project deletion must not delete files.
3. Expose backup validation and restore through the Wails runtime and UI,
   requiring explicit confirmation and preserving recovery copies.
4. Improve MCP subprocess coverage and document external-client verification.
5. Add UI component/integration evidence for operational acceptance.
6. Preserve shared-runtime parity and trust-boundary enforcement.

## Verification

- Go format/test/vet
- Wails frontend and Windows build
- MCP subprocess test
- Project filesystem isolation tests
- Restore application integration
- Credential lifecycle tests
- UI component/integration tests
- Diff, secret, artifact, and version checks
