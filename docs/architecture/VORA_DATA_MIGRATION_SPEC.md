# Data Migration Spec

## Goal

Preserve existing data while making project ownership, source provenance, and scientific lineage explicit.

## Migration rules

- deterministic
- idempotent where safe
- transactional when practical
- preserve historical records
- maintain explicit schema versioning
- detect invalid references
- fail with explicit error reporting
- avoid silent data loss
- retain external asset references
- allow recovery from partial application

## Legacy record handling

Records with unknown ownership must:

- remain readable
- keep original provenance
- support inspection
- allow assignment without overwriting original ownership metadata
- record assignment history

## Migration coordination

The repository currently has domain-level initialization and migration logic distributed across modules rather than a single migration ledger. This document preserves the current architecture while requiring each domain to remain consistent and checkable independently.

## Recovery and backup

- use SQLite transactions for migration steps when feasible
- validate data integrity after each migration stage
- verify `PRAGMA foreign_key_check` after migration runs
- preserve external files and assets referenced by records

## Required migration tests

- successful migration of existing data
- failed migration rollback
- idempotent re-run of the same migration
- assignment of previously unassigned record
- project-scoping validation after migration
- source version preservation during source link changes
