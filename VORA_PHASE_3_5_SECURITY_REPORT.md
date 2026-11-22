# VORA Phase 3.5 Security Report

## Resolved

- Credential creation now requires an available secret protection provider.
- Windows uses a user-context DPAPI implementation behind an isolated interface.
- Non-Windows test/development mode is explicitly enabled and fail-closed by
  default outside isolated tests.
- Plaintext is not persisted; only protected ciphertext, hash, metadata, and
  provider name are retained.
- Authentication validates protected plaintext against its SHA-256 fingerprint,
  expiry, revocation, and provider availability.
- Rotation revokes the old credential and creates a replacement.
- API and MCP credentials receive least-privilege permissions by default.
- Filesystem resolution rejects canonical paths outside configured roots.
- HTTP identity and permissions are derived from authenticated credential data.

## Remaining

- Windows DPAPI behavior requires execution on Windows for provider-level
  acceptance; compile evidence alone is insufficient.
- UI/API credential permission allowlists should be completed for every
  explicitly permitted scope.
- Project-specific filesystem roots require full project lifecycle integration.
- External desktop MCP client verification remains pending.
