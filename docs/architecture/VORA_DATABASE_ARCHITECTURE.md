# VORA MIND Database Architecture

Phase 1 initializes a clean SQLite database. Legacy user data is not migrated.

## Driver

`modernc.org/sqlite` v1.44.3 is a maintained CGO-free Go SQLite implementation.

## Initialization

- Resolve the OS application configuration directory.
- Create the application data directory with restricted permissions.
- Open `vora-phase1.db`.
- Enable foreign keys and SQLite busy timeout.
- Run ordered migrations in transactions.
- Track applied migration versions.

## Initial tables

- `schema_migrations`: migration state.
- `projects`: stable project IDs and timestamps.
- `audit_events`: capability execution evidence.

## Integrity

Repeated initialization is safe. The migration runner skips applied migrations
and the database test suite verifies foreign-key configuration and migration
convergence.
