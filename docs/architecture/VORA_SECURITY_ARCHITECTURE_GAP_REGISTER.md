# VORA Security Architecture Gap Register

## Critical

- No critical unrestricted-SQL or unrestricted-filesystem bypass is known in the Go foundation.
- The legacy frontend/Rust code is not yet governed by the Go runtime. Retirement cannot be considered safe until active Rust capabilities are mapped.

## High

- Independent MCP client invocation remains human acceptance pending.
- HTTP filesystem routes are partial; every exposed route must be audited against project scopes.
- Live restore, GUI approval/denial, and two-project switching remain unverified.

## Medium

- MCP grants a broad fixed permission set instead of credential/project-derived MCP permissions. Verify this is intentional and not caller-controlled.
- UI-origin approval context must remain exact; project deletion is fixed, but every approval-required operation should be audited.
- Windows symlink privilege cannot be verified in the current environment.
- Legacy Tauri commands may have caller identities and filesystem policies independent of the Go runtime.

## Low

- Wails official documentation retrieval returned 403 and therefore requires native repository evidence and later manual validation.

## Existing controls

- PermissionPolicy applies origin and permission checks.
- HTTP loopback and bearer credential authentication are enforced.
- MCP result errors are structured.
- Credential secret ciphertext and revocation/expiration are tested.
- Project roots and cross-project isolation are tested.
- Approval fingerprints reject workflow parameter substitution.
- Restore validation and rollback tests pass.
