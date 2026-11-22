# Vora Mind API Credential Security

Credentials use 32 bytes of `crypto/rand` entropy and are stored only by a
SHA-256 fingerprint in SQLite. The complete secret is returned only at creation
and is never logged. Authentication uses constant-time handling for the supplied
token and credential lookup; credential state, permissions, creation, last-use,
and revocation metadata are persisted.

The default external credential receives project read/write, database inspect,
and capability discovery. It does not receive filesystem write or workflow
write/approval permissions.

The API binds to `127.0.0.1`, rejects non-loopback Host names, rejects browser
Origin headers, limits request and header sizes, applies timeouts and rate
limits, and returns structured authentication errors.

Known limitation: Windows Credential Manager/DPAPI protection is not yet wired
to the credential manager. Plaintext credential persistence is prohibited; the
current database stores only token fingerprints.
