---
Status: IN_PROGRESS
Author: Copilot
Target Scope: Phase 3.7 external MCP and real UI acceptance
---

# Phase 3.7 External MCP and Real UI Acceptance

## Intent

Complete real-world external MCP and UI acceptance for the existing Go/Wails
architecture without starting a new phase or redesigning it.

## Requirements

1. Verify or configure VS Code MCP for the Vora executable without claiming
   external acceptance until the independent client connects.
2. Bind workflow approvals to exact capability input fingerprints.
3. Require explicit project scope for filesystem operations.
4. Add project delete confirmation to Wails and keep metadata deletion separate
   from workspace file deletion.
5. Add restore and approval security regression coverage.
6. Produce source/UI, subprocess, Windows, and external-client evidence
   separately.

## Verification

- Go test/vet
- Native Windows DPAPI tests
- MCP subprocess test
- Wails frontend/build
- UI guard tests
- Restore and approval parameter tests
- Secret/diff/staged checks

## Acceptance Evidence

- The exact repository state, external MCP configuration, and actual Wails UI outcomes are
  recorded in `VORA_PHASE_3_7_IMPLEMENTATION_REPORT.md`.
- Automated, subprocess, source/UI, actual-GUI, and external-client categories remain
  explicitly separated in `VORA_PHASE_3_7_ACCEPTANCE_MATRIX.md`.
- The Wails delete approval gap found during GUI acceptance is fixed and covered by
  runtime/source regression tests.
