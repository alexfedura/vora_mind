# Phase 0 — Migration Compatibility and Transaction Contract

**Status:** Migration architecture decision; no migration is implemented or executed here.  
**Evidence baseline:** [Current-state audit](../../docs/audit/VORA_CURRENT_STATE_AUDIT.md), [existing data inventory](../../docs/audit/VORA_EXISTING_DATA_INVENTORY.md), and [dependency/migration risks](../../docs/audit/VORA_DEPENDENCY_AND_MIGRATION_RISKS.md).  
**Domain and integrity rules:** [Domain contracts](./09-phase-0-domain-contracts.md), [relationship/lifecycle contracts](./11-phase-0-relationship-and-lifecycle-contracts.md).

## 1. Current compatibility facts and limits

The inspected application resolves a main AppData `vora_data.vora`, copies a legacy `vora_workspace.db` when its destination is absent, handles a legacy `database.json` selector, composes module-specific schema initializers, and imports SQLite packages with staged validation. The main database has no unified migration ledger or observed `PRAGMA user_version` policy. A second worker queue database, Trace files, settings sidecar, and external media/Parquet assets exist separately.

The repository audit did not inspect installed user workspaces, classify actual table/column variants, or prove which prior module migrations ran. The 58-table initializer inventory is not a deployed-DB version matrix. Therefore **there are currently no certified automatic legacy upgrade profiles in this Phase 0 deliverable**. Legacy support must be discovered and explicitly tested before it is claimed.

## 2. Compatibility matrix

| Input/workspace state | Evidence now | Phase 0 disposition | Future support gate |
|---|---|---|---|
| Empty workspace DB | Schema can be composed by current module initializers; no unified versioning today. | Fresh target DB is `user_version = 0` before migration and converges through the ordered target migration chain. | Fresh-schema tests prove same final schema as every supported upgrade. |
| Existing default `vora_data.vora` | Active path is known from source; deployed schema/rows are not. | Treat as `legacy-unclassified` until its schema fingerprint matches an owner-approved fixture/profile. Preserve original; no speculative scientific backfill. | Profile fingerprint, fixture, row/ID preservation, integrity checks, app reopen, and migration rollback test. |
| Legacy `vora_workspace.db` | Code copies it when fixed DB path is absent; schema distribution unknown. | Preserve source file and hash; after copy, classify contents as above. Copying is not proof of upgrade compatibility. | Test each recognized legacy profile; keep original/backup until post-migration acceptance. |
| Database selected via legacy `database.json` | Selector migration exists; actual custom paths/content are unknown. | Path selection is not evidence of valid workspace schema. Do not normalize/overwrite an unknown source. | Read-only preflight, explicit confirmation, verified backup, supported profile match, separate staging destination. |
| Imported portable SQLite package | Import stages a DB, performs current limited checks, then initializes schema; package schema generations are not fully inventoried. | Preserve input; reject scientific migration if schema/profile is unrecognized. Settings remain a separate input. | Versioned package manifest, exact schema profile, asset contract, data-preservation and full restore fixture. |
| Partially initialized module schema | Current startup can run independent initializers; interrupted legacy states are not catalogued. | Unknown/partial state is not auto-repaired by Phase 0. Inspect a copy and report unresolved; never silently “complete” using guessed columns. | Sanitized partial-init fixtures; deterministic repair migration or explicit refusal; interruption/restart tests. |
| AppData settings sidecar / legacy settings table | Sidecar is current; compatibility code can handle legacy table state. | Keep separate from scientific entity migration; preserve both and their association. | Paired DB/settings backup and import tests; explicit conflict policy. |
| Worker queue SQLite | Separate relative-path operational DB; deployment path unknown. | Not a scientific workspace migration input; preserve independently. Do not translate tasks into runs. | Separate worker schema/version policy only if deployment requires it. |
| Vora Trace files | File-backed graph/Markdown with its own schema/version fields and legacy-copy behavior. | Preserve as Trace data. Do not convert nodes to research projects, experiment runs or evidence. | Explicit versioned adapter and user-reviewed links if integration is later required. |
| Source/media and Parquet assets | Paths/hashes are partly stored; full backup coverage unknown. | Do not move, rewrite, infer, or delete files during schema migration. | Workspace asset manifest, verified checksums, staged copy/restore and missing-asset reporting. |

`user_version = 0` is not a blanket legacy profile. The migrator must first match the entire inspected schema shape to a tested fingerprint/profile. A `user_version` higher than the application supports, a mismatched migration checksum, partial ledger, unknown table/column signature, or failed integrity check causes an explicit stop with the source preserved.

## 3. Target migration architecture

### 3.1 Version authority and ownership

- Use SQLite `PRAGMA user_version` as the fast monotonic schema version for the main workspace file.
- Add `schema_migrations(version INTEGER PRIMARY KEY, migration_id TEXT UNIQUE, checksum TEXT NOT NULL, applied_at_us INTEGER NOT NULL, app_version TEXT NOT NULL)` as the detailed ledger. A migration's version, ID and source checksum must match the shipped manifest; mismatch is an error, never a successful skip.
- App version and schema version are distinct. A binary may support a bounded schema interval, declared and tested in release metadata.
- One Rust migration coordinator owns ordering, preflight, backup/staging, transaction boundaries and postflight. Domain modules may own their migration SQL/typed operations, but the coordinator applies one deterministic global sequence. Tauri handlers/UI do not run migrations or SQL.
- The legacy `src-tauri/src/schema.sql`, current inline Rust initializers, and included SQL files are not simultaneously authoritative schemas. A later implementation must establish one generated/owned fresh-schema path and one migration sequence; this phase does not rewrite existing schema files.
- The worker DB and Trace file formats remain separate versioning concerns; `user_version` for the main workspace does not certify them.

### 3.2 Fresh initialization

1. Create a new temporary DB on the same volume as the destination.
2. Configure and verify the DB connection before any transaction; create the schema-migration ledger and workspace metadata.
3. Apply the numbered target migration chain from the first schema version to the binary's supported current version. Fresh install and supported legacy upgrades must converge to identical schema and seeded predicate rules.
4. Run schema postconditions, `PRAGMA integrity_check`, `PRAGMA foreign_key_check`, relation/subtype consistency checks, JSON-schema checks for seeded data, and asset-reference checks.
5. Commit each numbered migration and `user_version` together. Publish the temporary DB at the destination only after all initialization and postconditions succeed.

No separate, hand-maintained “fresh schema” may drift from the migration chain. The preferred source is migration-owned DDL with a fresh initialization that runs the same versioned sequence.

### 3.3 Existing workspace upgrade

1. Close/stop all workspace connections and prevent worker/other processes from writing the file.
2. Read-only preflight: resolve path, hash the original, record SQLite/schema/ledger/app evidence, inspect exact tables/columns/indexes/FKs, check disk space, run `quick_check`, inspect `foreign_key_check`, and match a supported fixture fingerprint. No migration write occurs during preflight.
3. Produce a verified SQLite online backup. Preserve original, settings sidecar, and all in-scope referenced assets with a manifest/checksum. A DB backup alone is not a full-workspace backup.
4. Migrate a staged copy, not the only original. Enable FKs on the staging connection before transactions. Apply each ordered migration and ledger/version update atomically.
5. Run postconditions and compare protected table/row/ID inventories plus relation and asset manifests. Ambiguous values remain in legacy form with explicit unknown/unassigned status; conversion counts and unresolved items are reported.
6. Close staging handles, flush file data, then replace the active DB on the same volume with a recoverable prior copy. On Windows, ensure handles are closed before replacement. Never delete the previous DB/backup as part of the migration.
7. Reopen through the standard connection factory, reassert FK enforcement, run a representative read/write smoke test, and keep recovery artifacts for the documented acceptance window.

If a migration profile is unknown, stop before step 4 and ask for an approved manual recovery/mapping workflow. Do not attempt “best effort” conversion.

## 4. Idempotency, transactions, and interruption

- A numbered migration is applied once. DDL/DML, migration checksum row, and `user_version` update are in one `BEGIN IMMEDIATE` transaction where SQLite supports transactional DDL.
- Idempotency comes from a committed migration ledger and exact pre/postconditions, not from suppressing SQL errors or wrapping every statement in `IF NOT EXISTS`. Re-running an already committed version verifies its checksum and skips it; an interrupted transaction rolls back and remains unapplied.
- A migration that cannot be safely transactional (e.g. filesystem move, `VACUUM`, or platform-level file replacement) operates on a staged copy with a durable recovery manifest; the original is untouched until validation.
- `journal_mode` is configured outside an active transaction and is not changed as a side effect of a scientific schema migration. Busy timeout and connection policy are centralized.
- Never catch-and-ignore DDL, FK, checksum, asset, or row conversion errors. Failure aborts migration, reports migration ID/step/error, retains source and staging diagnostics, and does not advance schema version.
- Each migration has preconditions, exact supported input profiles, postconditions, data-loss classification, backup requirement, FK checks, and failure-injection tests.
- No migration drops/renames/deletes scientific or user-owned data unless a reviewed, separately approved destructive plan defines a backup, reversible staging, row-level report, user confirmation, and recovery test.

### 4.1 Multi-file and asset crash recovery

SQLite cannot atomically commit with filesystem assets or JSON settings. During migration, preserve asset bytes and settings in place unless a separate package operation is explicitly requested. For a future asset rewrite, write/verify immutable content-addressed assets first, commit DB references second, and record a recoverable migration manifest. A crash may leave an unreferenced asset; it must not leave a committed reference to missing bytes. Do not garbage-collect or auto-delete orphans in this phase.

## 5. Connection and foreign-key contract

Every connection constructor used by the app, import, CLI, tests, and future worker integration must pass through a shared persistence factory. Before preparing or executing application statements it must:

1. Set `PRAGMA foreign_keys = ON` outside a transaction.
2. Query `PRAGMA foreign_keys` and fail connection admission unless it returns `1`.
3. Set bounded busy timeout and documented safety pragmas.
4. Record database path/profile and connection purpose without leaking private data.

Migration runner, import/restore path, and normal app path each verify the pragma. Integration tests attempt a deliberately invalid FK on every constructor and require a constraint error. `PRAGMA foreign_key_check` runs after migrations/import and is a release gate. It is not sufficient to enable FKs only on the main shared connection.

The current audit identified `open_default_db_connection()` as a path that does not set `foreign_keys`; this contract requires fixing all such paths in a future implementation, not merely documenting the desired behavior.

## 6. Preservation and compatibility rules

- Preserve all legacy IDs and raw legacy values. New registry IDs for imported legacy entities, if approved, map through a durable alias record; do not rewrite identifiers referenced by snapshots, exports, or UI state.
- Preserve `sources`, `segments`, `extracts`, `links`, `source_revisions`, questions, hypotheses, Library/canvas snapshots, Market, Forge, settings and external files through a tested compatibility window.
- Do not infer SourceVersion from `source_revisions`; do not infer project ownership from title/path/board/Trace; do not convert hypothesis JSON into observations; do not promote extracts to claims/findings; do not convert market candles, Trace experiments, worker tasks, or visual edges into scientific runs/relations.
- Known `project` owner is assigned only by evidence-backed reviewed mapping. Otherwise set `unassigned` only when assignment is known to be outstanding; set `unknown` when the available evidence cannot determine ownership. Record reason and migration provenance.
- Preserve unknown JSON as original bytes/schema marker; unsupported schema versions are visible to users/operators and not decoded to success-shaped defaults.
- Existing feature writes/reads must continue across a migration release. New tables are additive first; destructive legacy retirement requires a later phase and separate acceptance.

## 7. Transaction boundary matrix

| Unit | Required transaction behavior |
|---|---|
| Single canonical entity + subtype + provenance | One transaction; registry identity, exactly one concrete subtype, and creation event either all persist or none. |
| Draft update | Optimistic revision check plus content/revision/provenance writes atomically; conflict leaves prior data untouched. |
| Version creation or release | Identity/version/subtype, content hash, parent links, provenance, and current-version event atomically; released version immutable. |
| Relation add/retract | Endpoint/predicate/version/cardinality validation and relation/event/provenance writes atomically; duplicate request is idempotent. |
| Project assignment | Validate destination project, append assignment event, update owner projection, preserve prior unknown/unassigned state atomically. |
| Observation/attempt/evidence capture | Entity, version/sample/input refs, validated asset manifest, and provenance commit atomically after external payload staging/checksum. |
| Run/analysis start | Persist run and frozen input manifest as queued/running, commit, then release DB lock for computation. |
| Run/analysis finalization | Validate outputs and hashes, append results/deviation/status event and final state atomically. Failure logs terminal failed outcome without deleting the run. |
| Knowledge transfer | Validate exact reusable version, source/recipient, license/review and acceptance; transfer, adoption relation and lineage/provenance commit atomically. |
| Numbered migration | One migration's DDL/DML and schema ledger/version bump are atomic; complex migrations use staged DB replacement. |
| Backup/restore | Backup creation and verification complete before replacement. DB/settings/assets are staged separately with a manifest; publish only after cross-file validation. |

Transaction details and user-operation semantics are normative in [relationship/lifecycle contracts](./11-phase-0-relationship-and-lifecycle-contracts.md).

## 8. Required migration test matrix before any migration release

1. Fresh install and every supported fixture converge to identical schema fingerprint/version.
2. Exact old workspace profiles: populated and empty; `vora_data.vora`; copied `vora_workspace.db`; legacy selected-path; supported package imports; each known partial initializer state.
3. Unknown schema, newer schema, broken ledger/checksum, malformed JSON, dangling FK, insufficient disk space, missing/changed external asset: fail closed with originals intact and actionable diagnostics.
4. Migration applied twice; process killed before/during/after each migration; restart; `user_version` never advances on uncommitted changes.
5. Injected failure at each DDL/backfill/validation/replace boundary; staging/original/backup remain recoverable.
6. Protected row counts, IDs, table data, raw blob hashes, external asset hashes, settings association, old UI/API read/write behavior, and relation history preserved.
7. Foreign-key pragma verified on every connection factory and worker/CLI paths that may access scientific DB.
8. Post-migration `integrity_check`, `foreign_key_check`, entity/subtype, relation/version, ownership, JSON, asset and required-lineage checks.
9. Full backup/restore to a different clean path, with missing/relocated assets reported and no false success.
10. Conversion report enumerates converted, preserved, unassigned, unknown, rejected and failed rows with stable legacy references. No ambiguous mapping is silently applied.

## 9. Phase 0 exit and non-claims

Phase 0 establishes the migration architecture but **does not**:

- Establish supported legacy profiles or a safe actual upgrade path; that requires authorized deployed DB copies and tests.
- Add a migration ledger, set `user_version`, change startup, repair FK connection setup, or modify any application schema.
- Execute or test migrations, create fixtures from user data, convert entities, or alter user workspaces.
- Claim the current backup process includes all external files, settings, Trace or worker state.
- Claim the target domain contracts have stakeholder ratification.

The next step, if approved, is a separate Phase 1 spec and work item: obtain sanitized compatibility fixtures, define exact support profiles, then implement the connection/migration foundation and tests before scientific feature tables.
