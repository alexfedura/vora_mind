# Phase 1: Wails v3 Go Capability Foundation

## Status

Implemented only after Phase 0 approval. Legacy Tauri, Electron, and Rust
sidecar code remain unchanged.

## Objective

Create an independently runnable Wails v3 desktop application backed by Go,
SQLite, a permission-aware capability runtime, and the existing React
technology direction.

## Application location

- `go-app/` is the complete Wails v3 application root.
- `go-app/frontend/` retains and progressively replaces the legacy React UI.
- `go-app/backend/domain/` contains pure domain types and rules.
- `go-app/backend/application/` contains use cases and capability orchestration.
- `go-app/backend/capabilities/` contains capability definitions and handlers.
- `go-app/backend/infrastructure/` contains SQLite and filesystem adapters.
- `go-app/backend/security/` contains permissions and policy types.
- `go-app/backend/audit/` contains execution audit persistence and services.
- `go-app/backend/persistence/` contains migrations and repositories.

## Dependency direction

Wails bindings -> application services -> domain interfaces -> infrastructure
implementations.

Domain code must not import Wails, React, or infrastructure packages.

## Vertical slice

The first runtime capabilities are:

- `system.get_info`
- `project.create`
- `project.list`
- `project.get`
- `project.update`
- `database.get_status`
- `database.list_tables`
- `database.get_schema`
- `filesystem.list`
- `filesystem.stat`
- `filesystem.read_text`
- `filesystem.create_text`
- `capability.list`
- `capability.get`

## Execution contract

Every capability declares identity, version, name, description, JSON-compatible
input/output schemas, required permissions, handler, validation, audit behavior,
and structured error mapping.

## SQLite

The selected implementation will use a maintained CGO-free Go SQLite driver.
Fresh databases initialize automatically. Migrations are ordered and repeatable.
Foreign keys are enabled per connection and schema state is tracked.

## Security

Every capability requires at least one declared permission. The execution engine
checks permissions before handler execution. Filesystem operations are restricted
to the configured application data directory and explicit user-selected subtrees.

## Verification

Phase 1 is complete only with passing Go tests, frontend checks/build, Wails
build verification, and executable process startup evidence.
