# Vora Mind SKDDA — Phase 0 Executive Report

**Decision requested:** Approve the Phase 0 architecture and bounded Phase 1 preparation scope below before implementation begins.  
**Implementation status:** No SKDDA feature or migration was implemented as part of this package.

## 1. Executive summary

Vora Mind has a real local-first application foundation and several useful knowledge-work primitives, but it is not yet a unified scientific research system. Source provenance, spatial boards/versioning, question triage, a hypothesis/Lab workflow, chart/market persistence, Trace, Forge and worker operations all exist as distinct domains. Some are reusable; none should be promoted to complete scientific infrastructure just because its name resembles a target entity.

Current maturity:

- **Strong:** desktop/frontend/backend integration pattern; SQLite-backed workspace; source→segment→extract provenance; append-only source revision triggers; board and library snapshot/version systems; inquiry triage; hypothesis protocol/verdict/data-entry workflow; backup/import/export support; focused Rust unit tests.
- **Partial:** source attribution; evidence/support/contradiction links; question-to-hypothesis lifecycle; hypothesis lineage; research visualization and historical snapshots.
- **Missing as first-class scientific capability:** research projects/domains/gaps; learning objectives/assessments/mastery; typed observations; experiment design/run; dataset/version/sample; reproducible analysis; evidence/finding synthesis; independent model/version lifecycle; validation/replication; universal typed relationships; cross-project transfer.

Overall readiness for SKDDA implementation: **architecturally promising but data-contract readiness is low**. The highest-risk work is migration/provenance design, not screen implementation. Building experiments or model versions before resolving exact IDs, relation integrity, source/data version pinning, connection configuration, and migration rollback would risk disconnected or historically ambiguous research records.

## 2. Critical structural risks

1. **No single migration ledger.** Runtime schemas are assembled from multiple Rust modules and SQL files using idempotent table creation/column checks and module-specific markers; partial startup mutation is possible.
2. **Foreign-key integrity is connection-dependent.** Main setup enables FKs, Forge and worker configure them, but `open_default_db_connection()` does not set them; SQLite pragma is per connection.
3. **Soft and polymorphic refs.** Source `links.target_id`, question promotion, hypothesis inquiry/source/parent/snapshot fields, and canvas edge endpoints are not all DB-enforced references.
4. **Schema source ambiguity.** `src-tauri/src/schema.sql` is not the actual complete runtime schema; module initialization and `schema_source.sql`/`schema_market.sql` define deployed behavior.
5. **History semantics vary.** Source revision rows have update/delete triggers; hypothesis child data and library versions have cascade behavior; the whole workspace does not yet have a uniform scientific immutability policy.
6. **External asset recovery.** SQLite online backup exists, but inclusion of every document/media/Parquet/Trace asset is not established.
7. **Scientific evidence gaps.** Hypothesis notes/JSON data do not provide a dataset, experiment run, statistical analysis provenance, or reusable finding.

Evidence and details are in [01-repository-audit.md](01-repository-audit.md), [02-database-persistence-audit.md](02-database-persistence-audit.md), and [07-migration-strategy.md](07-migration-strategy.md).

## 3. Existing reusable infrastructure

- Main app SQLite `AppState` and typed Tauri command path.
- Rust source DB operations, parent/supersession links, review flags, and append-only source revision events.
- Question triage and tested transactional promotion to hypothesis.
- Hypothesis protocol lock/execution/verdict and parent/refinement workflows.
- Board/canvas snapshots, checkpoint/delta reconstruction, board/library version snapshots.
- Main-database online backup/export and staged import/restore logic.
- Existing Rust schema tests and domain-level transaction patterns.
- Market tick/Parquet access and chart data as potential explicit adapters, not prevalidated datasets.

Reuse means preserving behavior and data while adding new typed contracts; it does not mean converting existing board, Trace, Market or Library objects wholesale into scientific records.

## 4. Major missing capabilities

One shared scientific domain model and typed lineage; explicit research project ownership; source editions/versioned citations; claims/concepts/principles/framework versions; formal learning/mastery; phenomena and observations; experiment design/run; datasets and immutable dataset versions; AnalysisRun; typed Evidence and Finding; Model/ModelVersion/ModelParameter/Evaluation; Validation and Replication; required bias/reproducibility disclosures; and knowledge transfer with exact version attribution.

See the complete evidence matrix in [03-domain-gap-analysis.md](03-domain-gap-analysis.md).

## 5. Decisions requiring approval in Phase 0

1. **Scientific relation integrity:** typed relation tables, central entity registry, or a hybrid. Must enforce endpoints in SQLite and Rust; no bare unconstrained polymorphic IDs.
2. **Canonical project/ownership policy:** which records are project-owned vs reusable; represent unknown legacy ownership without auto-assignment.
3. **Source version identity:** what constitutes an edition/capture/version for books, URLs, media and local files; how exact citations bind.
4. **Dataset storage:** row-level SQLite vs external immutable files/manifests for larger data; checksum, split assignment and backup bundle contract.
5. **Migration framework:** migration ledger format, supported old workspace versions, transaction/staging/recovery policy, handling of partial initialization.
6. **Deletion and retention:** which existing cascades are acceptable for current UI-only content and which scientific data must be retained/retracted rather than deleted.
7. **Theory semantics:** whether Theory is a distinct typed entity or a role/composition of framework/model/claims. Do not create a table before this is decided.
8. **Learner identity/privacy:** single-user local profile assumptions, data retention and export model.
9. **Statistical execution:** what runs inside app vs worker/external tool; capture environments/results without holding DB locks or implying validation.

## 6. Recommended Phase 1 scope

**Phase 1 should be a narrow “shared identity, provenance, and relation integrity foundation,” not a broad feature phase.**

Deliver:

- Approved ERD/logical registry and typed relation pattern with endpoint matrix.
- Canonical version/citation semantics for existing sources and explicit unknown state.
- One DB connection factory/configuration contract; assert foreign keys for every durable DB path.
- Ordered migration runner/ledger and compatibility fixture matrix, including backup/restore and failure behavior.
- Non-destructive schema inspection tool/test fixture against sanitized copies only.
- A minimal lineage query service that proves source → segment → extract → claim/citation without automatic conversion of legacy extracts to claims.
- Legacy reference inventory/report for unresolved links and soft refs; no data remapping absent explicit user/review action.
- Type/DTO serialization rules (enum, nullability, timestamp units, numeric precision, errors).

**Out of scope for Phase 1:** full research project UI, experiment execution, statistical library, data import automation, model lifecycle UI, learning assessment, cross-project adoption UX, or conversion of existing content into scientific types.

## 7. Phase 1 dependencies

- Architecture owner approves decisions in §5.
- Identify supported deployed workspace DB variants using user-approved copies, not production database mutation.
- Migration fixture set includes empty/current/legacy and partially initialized states.
- Define file/asset manifest and backup assumptions for the main `.vora` database.
- Resolve table ownership and how source/relation migrations coexist with existing module-owned schema initialization.
- Tests run on temp copies/in-memory data and prove rollback without touching user data.

## 8. Phase 1 acceptance criteria

1. Written ADR/ERD selects endpoint integrity strategy and lifecycle rules.
2. Fresh and supported legacy fixtures have a deterministic migration version and converge without data loss.
3. Connection factory returns connections with foreign keys enabled; tests assert this for app, CLI, market and worker-adjacent paths as applicable.
4. Migration failure leaves original DB recoverable, reports actionable error, and does not mark schema version complete.
5. Backup/restore preserves main workspace database records, settings association, FK integrity, and explicitly reports external assets not included.
6. Existing question/hypothesis/source/library/canvas row counts and stable IDs are preserved through migration fixtures.
7. Legacy source link/hypothesis/canvas refs without proven endpoints are surfaced as unresolved; no inferred mapping from text/title.
8. Typed relation validation rejects invalid endpoint/predicate pairs in backend and enforces concrete FK integrity.
9. Source→segment→extract→claim/citation lineage returns exact known records; unknown source version remains unknown.
10. Rust/TypeScript contract tests agree on serialization, casing, nullability, time units and errors.
11. No feature is called “scientific validation,” “dataset versioning,” or “experiment execution” solely because legacy text or a chart/Trace node exists.
12. Focused migration, repository, frontend type/build and Rust checks pass; any user-data-backed test is explicitly excluded unless a disposable copy is used.

## 9. What was inspected and what remains unknown

Inspected source/schema and existing audit evidence for:

- Main app setup and DB initialization (`src-tauri/src/lib.rs`, `mind/portable_db.rs`).
- Source tables/operations/revisions (`schema_source.sql`, `source/model.rs`, `source/db.rs`, `source/revision.rs`, `source/ipc.rs`).
- Mind board/library/hypothesis/question tables and workflows (`mind/canvas_db.rs`, `mind/resolver.rs`, `mind/library_groups.rs`, `mind/questions.rs`, `mind/hypothesis.rs`).
- Market/Parquet and Forge table ownership (`schema_market.sql`, `market/db.rs`, `market/local_db.rs`, `forge/index.rs`).
- Worker queue database (`bin/vora_worker.rs`).
- Frontend services, stores and representative UI; Trace file workflow.
- Existing repository audit/spec documents and master SKDDA spec.

Not verified by runtime execution: user DB count, DB integrity, installed-app UI persistence, every open-connection pragma, external asset backup completeness, all schema variants deployed to users, and actual execution of Rust tests. No DB was opened or modified and no migrations/tests were run for this documentation task.

## 10. Package index

1. [Repository architecture audit](01-repository-audit.md)
2. [Database and persistence audit](02-database-persistence-audit.md)
3. [Domain gap analysis](03-domain-gap-analysis.md)
4. [Canonical domain contracts](04-canonical-domain-contracts.md)
5. [Scientific relationship architecture](05-scientific-relationship-architecture.md)
6. [Lifecycle and versioning specification](06-lifecycle-versioning-specification.md)
7. [Migration strategy](07-migration-strategy.md)
8. [Phase 0 executive report](08-phase-0-executive-report.md)
