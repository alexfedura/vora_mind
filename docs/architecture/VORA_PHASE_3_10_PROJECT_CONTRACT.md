# VORA Phase 3.10 Project Contract

## Supported operations

| Operation | Implementation | Wails | HTTP | MCP | Workflow |
|---|---|---|---|---|---|
| Create | `project.create` via project service | yes | yes | yes | capability |
| List | `project.list` via project service | yes | yes | yes | capability |
| Get | `project.get` via project service | yes | yes | yes | capability |
| Update | `project.update` via project service | yes | yes | yes | capability |
| Delete | `project.delete`, approval required | yes | approval denied unless approved | approval denied | approval |
| Open/switch | `project.open` | yes | yes | yes | capability |
| Get context | `project.get_context` | yes | yes | yes | capability |

No separate `project.switch` capability was added. Wails and external adapters use `project.open` for switching.

## Authoritative implementation

The shared project responsibilities live in `go-app/backend/projects/service.go` and execute repositories under `go-app/backend/persistence/project_repository.go`.

Active project context is persisted in `application_identity.active_project_id`.
