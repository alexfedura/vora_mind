# VORA MIND Security and Permissions

## Permission scopes

- `project.read`
- `project.write`
- `database.inspect`
- `filesystem.read`
- `filesystem.write`

The capability runtime checks these permissions before invoking handlers.

## Execution origins

- `human_ui`
- `internal`
- `ai_agent`
- `external_api`

Execution origin is transport metadata, not an authentication identity.

## Filesystem

Filesystem operations are restricted to explicitly configured roots. Relative
paths resolve from the default application data root. Absolute paths are
accepted only when they remain under an allowed root.

## Audit

Audit events contain execution identity, capability, origin, project scope,
status, duration, errors, and required permissions.

## Future policy

AI permissions and approval gates must remain configurable and must not grant
raw SQL or unrestricted filesystem access.
