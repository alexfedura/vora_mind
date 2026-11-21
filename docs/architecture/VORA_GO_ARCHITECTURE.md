# VORA MIND Go Architecture

The Go foundation follows a modular, dependency-directed structure.

```text
Wails bindings / future API adapters
              |
       application runtime
              |
   capability registry + handlers
              |
       domain interfaces
              |
 SQLite / filesystem / audit adapters
```

## Package ownership

- `backend/domain`: domain types and domain rules.
- `backend/application`: orchestration and capability registration.
- `backend/capabilities`: capability contracts, registry, validation, results,
  and errors.
- `backend/infrastructure`: filesystem and external infrastructure adapters.
- `backend/persistence`: SQLite lifecycle, migrations, and repositories.
- `backend/security`: permission scopes.
- `backend/audit`: execution event contracts and persistence.
- `runtime_service.go`: Wails delivery adapter.
- `main.go`: process composition, lifecycle, configuration, and window startup.

## Execution flow

```text
React -> RuntimeService -> Capability Runtime -> Handler
                                              -> Application Service
                                              -> Repository/Filesystem
                                              -> Structured Result
```

Future AI and external API adapters must call the same capability runtime and
must not duplicate handlers or business rules.
