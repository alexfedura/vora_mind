# Vora Mind Filesystem Capability Security

Filesystem operations execute only through the shared capability runtime.
Allowed roots are canonicalized and symlinks are resolved before boundary
checks. Relative paths are resolved inside the first configured scope.

Supported capabilities:

- `filesystem.list`
- `filesystem.stat`
- `filesystem.read_text`
- `filesystem.create_text`
- `filesystem.edit_text`
- `filesystem.rename`
- `filesystem.copy`
- `filesystem.move`
- `filesystem.delete`

Reads require `filesystem.read`; writes require `filesystem.write`. Delete
requires explicit runtime approval. API and MCP contexts do not receive
filesystem write by default.

Known limitations: project-specific roots are not yet mapped per project,
directory copy/delete are intentionally unsupported, and race-safe openat-style
file operations remain unavailable on the portable abstraction.
