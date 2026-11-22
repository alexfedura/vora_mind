---
Status: IN_PROGRESS
Author: Copilot
Target Scope: VORA Phase 3.5 operational acceptance
---

# Phase 3.5 Operational Acceptance

## Intent

Close the remaining security and operational gaps in the existing Go/Wails
runtime without introducing a second adapter implementation.

## Requirements

1. Credential material is protected with an OS-backed provider on Windows,
   with an explicit test/development provider that fails closed unless enabled.
2. API credential metadata, lifecycle, rotation, revocation, and project scopes
   are exposed without listing plaintext secrets.
3. Filesystem operations remain canonical and project-aware, with no default
   write access for HTTP or MCP adapters.
4. MCP is verified by an external official-SDK subprocess test.
5. Database restore remains staged, authorized, validated, reversible, and
   rebound across runtime services.
6. Wails, HTTP, MCP, and workflows continue to delegate to the shared runtime.

## Verification

- Go unit and integration tests
- Go vet
- TypeScript and frontend builds
- Wails build
- MCP executable subprocess test
- Restore and credential lifecycle tests
- Diff and secret scanning
