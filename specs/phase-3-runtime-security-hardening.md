---
Status: APPROVED
Author: Copilot
Target Scope: go-app runtime security and operational controls
---

# Phase 3 Runtime Security and Reliability

## Intent

Complete the Phase 2 foundation without bypassing the shared capability runtime.
The implementation preserves loopback-only HTTP, Wails binding, official MCP SDK,
SQLite durability, and the existing application architectures.

## Shared Requirements

- API credentials are generated with `crypto/rand`, persist only through an
  OS-backed credential provider, and are presented once.
- External and MCP identities receive explicit least-privilege permissions.
- Filesystem operations use canonical paths, allowed roots, and capability
  permissions.
- Workflow interruption is recovered deterministically. Naturally idempotent
  steps may retry; uncertain or non-idempotent steps require operator resolution.
- Database restore is blocked until a backup passes integrity, foreign-key,
  schema, ownership, and path checks, then uses staged replacement and rollback.
- MCP remains a stdio protocol adapter over the capability runtime.

## Verification

- `go test ./...`
- `go vet ./...`
- Wails frontend build
- Wails Windows build
- MCP protocol tests and subprocess build
- Root frontend build
