# VORA Phase 3.6 Security Report

## Resolved

- Native Windows DPAPI protect/unprotect roundtrip passed.
- Invalid/corrupted DPAPI ciphertext is rejected.
- Credential ciphertext is never persisted as plaintext.
- Revocation, expiration, and rotation fail closed.
- Project workspace roots are database-persisted and recovered.
- Project deletion preserves workspace files.
- Cross-project filesystem resolution is rejected.
- Restore UI requires preflight validation and confirmation.
- MCP subprocess write denial and malformed input are tested.

## Unresolved

- External desktop MCP client acceptance has no installed client evidence.
- Manual visual approval and permission-denial flows remain pending.
- Real UI interaction acceptance is not claimed.
