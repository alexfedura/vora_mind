# VORA MIND Capability Contract

Each capability has a stable ID, semantic version, name, description,
JSON-compatible input/output schemas, validation handler, required permissions,
execution handler, structured result, structured error, and audit behavior.

## Execution result

```json
{
  "executionId": "exec_<id>",
  "capabilityId": "project.list",
  "status": "succeeded",
  "data": {},
  "durationMs": 1
}
```

## Structured error

```json
{
  "code": "PERMISSION_DENIED",
  "message": "The execution origin does not have the required permissions.",
  "capability": "filesystem.create_text",
  "retryable": false
}
```

Discovery descriptors contain machine-readable names, schemas, and required
permissions. They are intended for future provider-neutral AI tool adapters,
external API documentation, and UI consumers.
