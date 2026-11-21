# VORA MIND — Phase 0 Full-System Discovery and Go/Wails v3 Migration Architecture

**Status:** Phase 0 discovery complete; migration not started  
**Verified at:** 2026-11-22  
**Repository:** `alexfedura/vora_mind`

## Executive conclusion

VORA MIND is not a framework-only application in need of a simple runtime swap.
It is a mixed local-first desktop system with persistent Rust/Tauri domain
behavior, an Electron shell experiment, a Rust sidecar experiment, React
interfaces, SQLite data, filesystem-backed assets, market/Parquet processing,
and substantial scientific-domain contracts.

The safest target remains a Go/Wails v3 application with React retained as the
presentation layer, but the migration should be capability-driven rather than
file-driven. Rust and Tauri domain behavior must be translated and revalidated
before retirement. Electron and the Rust sidecar must be removed only after the
Wails shell and Go application services reproduce their required behavior.

No Phase 1 implementation is approved by this report.

## 1. Repository and Git baseline

### 1.1 Verified Git state

- Branch: `feat/e2-electron-foundation`
- HEAD: `0b93fb8ef61c5947b728c22fb43f55d37dc4eec6`
- HEAD subject: `feat(layout): add Electron runtime integration`
- HEAD date: 2026-11-21 23:24:29 +0330
- Remote: `origin https://github.com/alexfedura/vora_mind.git`
- Latest observed tags: `v0.24.0`, `v0.23.4`, `v0.23.1`, `v0.23.0`

The working tree was not clean at the start of discovery. Existing changes
included Electron runtime integration and sidecar files plus package and Tauri
configuration edits.

### 1.2 Working tree status

Modified:

- `electron/main.ts`
- `electron/preload.ts`
- `package.json`
- `pnpm-lock.yaml`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src/shared/components/ElectronRuntimeBadge.tsx`
- `src/vite-env.d.ts`
- `tests/electron-security.test.cjs`

Untracked:

- `docs/architecture/VORA_E2_4_SIDECAR_INTEGRATION.md`
- `electron/sidecar-ipc.ts`
- `electron/sidecar-manager.ts`
- `electron/sidecar-protocol.ts`
- `electron/sidecar/`
- `specs/e2-electron-rust-sidecar.md`
- `tests/electron-sidecar.test.cjs`

No existing change was reverted or rewritten. The report itself is a new
documentation-only modification.

### 1.3 Toolchain baseline

- Go: `go1.27.1 windows/amd64`
- Node: `v24.19.0`
- pnpm: `12.6.0`
- Rust: `rustc 1.97.1`
- Cargo: `1.97.1`
- Package version: `0.26.3`

## 2. Electron removal inventory

### Active runtime dependencies

- `electron` and `electron-builder` are direct dev/build dependencies.
- `electron-updater` is a runtime dependency.
- `electron/main.ts` owns window lifecycle, navigation policy, CSP, single
  instance behavior, updater startup, shutdown, and sidecar startup.
- `electron/preload.ts` exposes a narrow `window.voraElectron` bridge.
- `electron/security.ts` authorizes IPC calls and validates payload shape.
- `electron/sidecar-manager.ts` spawns and owns a Rust executable protocol.
- `electron/sidecar-protocol.ts` provides Zod schemas for sidecar requests.
- `electron/sidecar-ipc.ts` registers sidecar IPC handlers.
- `electron/sidecar/src/main.rs` is a proof sidecar implementing health,
  synthetic SQLite, approved SQL, Parquet fixture, and shutdown operations.
- `.github/workflows/electron-release.yml` packages and signs Electron NSIS
  installers.
- `package.json` retains Electron scripts and NSIS/electron-builder metadata.

### Removal/translation targets

| Current Electron responsibility | Target |
|---|---|
| BrowserWindow/WebView lifecycle | Wails application/window lifecycle |
| contextBridge/preload | Wails bindings plus explicitly typed bridge contracts |
| ipcMain handlers | Go services exposed only through capability adapters |
| CSP/navigation authorization | Wails/frontend origin and capability authorization; backend permission checks |
| Electron updater | Retained product updater implemented through Wails/native lifecycle or a dedicated Go service |
| Single-instance behavior | Wails/application-lifecycle implementation or Go process lock |
| Rust sidecar process manager | Keep as domain worker only if proven reusable; otherwise replace with Go application services/jobs |
| Electron NSIS packaging | Wails build/sign/package pipeline |

Electron is a real active implementation on this branch, not only historical
documentation.

## 3. Rust/Tauri retirement inventory

### Runtime dependencies

- Tauri v2 dependencies are in `package.json` and `src-tauri/Cargo.toml`.
- `@tauri-apps/api`, dialog, fs, opener, process, and updater are frontend
  dependencies.
- Rust/Tauri commands are registered in `src-tauri/src/lib.rs`.
- Source, market, research, experiment, analysis, model, learning, knowledge
  transfer, and acquisition expose additional domain IPC modules.
- AppData database initialization and migration live in
  `src-tauri/src/mind/`.
- `rusqlite`, DuckDB, Git, Tree-sitter, DuckDB/Parquet, HTTP, and worker
  utilities are Rust implementation dependencies.

### Responsibility map

| Rust/Tauri responsibility | Go/Wails target |
|---|---|
| AppState and SQLite connection ownership | Go database/transaction manager |
| Tauri command handlers | Capability handlers, Go application services, and Wails bindings |
| Domain repositories | Go repositories |
| Scientific schemas/migrations | Central migration ledger and clean initial schema |
| Canvas and snapshot persistence | Go canvas/document/visual object service |
| Source acquisition/extraction | Go source and provenance service |
| Market data and DuckDB/Parquet | Go market/dataset adapter with explicit dataset versions |
| Forge index and AST tools | Go code-index/tool subsystem or reusable executable adapter |
| Worker queue | Go background-job and workflow runner |
| Trace file store | Go project/decision graph repository or deliberate redesign |
| Updater and dialogs | Wails/native lifecycle plus Go update service |
| DeepSeek HTTP integration | Provider adapter in AI integration module |

The Rust proof sidecar is distinct from the main Tauri Rust backend and should
not be confused with a reusable backend; it is an early Electron integration
prototype.

## 4. Existing product capability inventory

The following inventory classifies repository evidence rather than inferred
product intent.

| Capability | Purpose | Evidence/status | Database/files | Reuse decision |
|---|---|---|---|---|
| Boards and spatial canvas | Create objects, links, cards, shapes, snapshots | Implemented, not fully runtime-certified | Main SQLite: boards, objects, edges, versions | Reuse domain concepts; redesign identity and persistence around visual entities |
| Library and item groups | Organize reusable knowledge | Implemented, partially integrated | SQLite library/group/version tables | Reuse UI and repository concepts |
| Document viewers/editors | Markdown, PDF, DOCX, text viewing/editing | Implemented in React | Filesystem and document references | Reuse frontend; route mutations through services |
| Media | Reference/play audio/video | Implemented path exists | Media tables and filesystem references | Reuse with explicit asset manifests |
| Card versioning | Commit and restore card/board versions | Implemented; focused tests exist | SQLite | Redesign into general entity revisions |
| Project lifecycle | Create/select/archive projects | Implemented with project context in scientific slices | Research/project tables | Reuse and extend universally |
| Source catalog | Catalog sources, segments, extracts, revisions | Implemented/partially verified | SQLite Source schema | Reuse with immutable source editions |
| Source acquisition | Acquire content and register provenance | Implemented in Rust/TS | Source acquisition tables/files | Reuse behind application service |
| Research | Gaps, questions, observations, findings | Implemented domain slices | Scientific SQLite modules | Reuse contracts, add complete service boundaries |
| Inquiry triage/promotion | Triage questions and promote hypotheses | Implemented/transactional | Questions/hypothesis tables | Reuse as capability, add project rules |
| Hypothesis/Lab | Protocol, checklist, execution, verdict | Partial scientific experiment substitute | JSON fields plus hypothesis tables | Redesign JSON protocol into typed experiment/design/run |
| Experiment | Design/run/dataset concepts | Implemented domain slices | Experiment schema/repositories | Reuse as target contracts, add end-to-end workflow acceptance |
| Analysis | Runs, evidence, temporal leakage rules | Implemented domain slices | Analysis schema/repositories | Reuse; add execution/job integration |
| Model | Model identity, versions, validations, replication | Implemented domain slices | Model schema/repositories | Reuse; define project/global visibility consistently |
| Learning/mastery | Assessment and competency concepts | Implemented domain slices | Learning schema/repositories | Reuse; attach execution/audit |
| Knowledge transfer | Transfer/adapt across projects | Implemented domain slices | Transfer schema/repositories | Reuse; authenticate actor/recipient later |
| Trace | Decision graph and experiment-like nodes | Implemented file-backed UI | AppData files/YAML/Markdown | Redesign or translate as a view adapter over canonical entities |
| Market/chart | Candles, lines, rectangles, zones, measurements | Implemented | SQLite and Parquet/DuckDB | Reuse visual geometry and coordinate utilities; replace domain-specific assumptions |
| Replay and snapshots | Capture and replay canvas/chart state | Implemented | SQLite/file-backed snapshot structures | Reuse as generic snapshot service |
| Search/retrieval | SQL and UI filtering, limited text search | Partial; no unified retrieval API | SQLite/filesystem | Required redesign |
| AI board generation | Prompt DeepSeek and stream generated board JSON | Implemented | Settings plus board API | Reuse only as provider adapter/tool workflow |
| Forge/code intelligence | Symbol indexing, graph, patching, impact | Implemented Rust domain code | SQLite and Git/filesystem | Reuse service logic, provider-independent |
| Worker | Queue, heartbeat, logs and worker process | Implemented external process | Separate SQLite queue | Redesign as background workflow runner |
| Import/export/backup | SQLite backup, settings, board backup | Implemented partially | SQLite/filesystem | Add asset manifest and full recovery verification |
| Updater/settings/lifecycle | Tauri and Electron updater paths | Implemented, dual shell | JSON/settings/updater metadata | Choose one Wails implementation |

## 5. Current architecture assessment

Current runtime layers are:

1. React 19/TypeScript UI, Zustand stores, typed frontend services.
2. Tauri v2 IPC or Electron preload IPC.
3. Rust command/domain/repository logic and SQLite.
4. Separate market, Trace, worker, source, scientific, and sidecar persistence.
5. Filesystem assets and Parquet/DuckDB data.

Strengths:

- Strong local-first persistence and domain-model inventory.
- Typed Rust DTO/service patterns exist.
- Scientific domain modules have focused repository tests and migrations.
- Project, exact SourceVersion, and provenance concepts exist in recent slices.
- React components and existing geometry utilities can be retained.

Primary weaknesses:

- Two desktop shells and two updater/packaging systems are active.
- Rust IPC, repositories, and domain modules are not a single shared
  application-service architecture.
- Scientific schemas are split across module-owned initialization and migration
  code with no single migration ledger.
- Database, workflow, AI, filesystem, and external-network permissions are not
  modeled as one authorization system.
- No raw end-to-end desktop or full Rust library result was available at the
  Phase 0 start.
- `0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND` prevents the full Rust library
  executable from starting in this environment, despite prior repository
  reports of green focused suites.
- External assets are not included in a database-only backup manifest.

## 6. Target Go/Wails v3 architecture

### 6.1 Module layout

Recommended future layout:

```text
cmd/vora/
internal/
  app/
  capability/
  auth/
  audit/
  project/
  source/
  research/
  experiment/
  analysis/
  model/
  learning/
  transfer/
  canvas/
  snapshot/
  media/
  market/
  forge/
  search/
  ai/
  workflow/
  external/
  filesystem/
  database/
  event/
pkg/contracts/
web/
```

Dependency direction:

```text
UI / Wails bindings / external API
             |
    Capability adapters
             |
       Application services
             |
 Domain rules + repository interfaces
             |
 SQLite / filesystem / provider / network adapters
```

No Wails binding, HTTP handler, AI tool, Electron IPC handler, or CLI command
may bypass application services and domain validation.

### 6.2 Persistence

- Modernize to Go SQLite with explicit migrations and ordered, transactional
  schema initialization.
- Use a single active database connection pool/transaction boundary.
- Enforce project and entity ownership in repositories and services.
- Store immutable source/visual/entity revisions with audit provenance.
- Keep external assets in a manifest with hashes and recoverable paths.
- Provide backup/restore that tests database integrity and asset completeness.
- Use a separate workflow/job database only if it cannot remain in the primary
  SQLite database without lock contention.

## 7. Shared capability contract

Every capability requires:

- Stable identifier: for example `canvas.object.create.v1`
- Semantic versioned JSON input/output schemas
- Validation rules
- Authorization scope and approval class
- Execution mode: synchronous, streamed, cancellable, resumable
- Idempotency key where applicable
- Typed success and error codes
- Retryability and compensation/recovery behavior
- Audit event on start, completion, denial, and failure
- Documentation and discoverable examples
- Unit, repository, contract, and end-to-end tests

Recommended interfaces:

```text
Human UI -> Capability Dispatcher -> Application Service -> Repository/Infrastructure
AI Agent -> Tool Adapter -> Capability Dispatcher -> Application Service
External API -> API Adapter -> Capability Dispatcher -> Application Service
CLI/Worker -> Job Adapter -> Capability Dispatcher -> Application Service
```

Capabilities should be generated or registered from one metadata registry.
AI tools and HTTP routes consume the same registered contracts rather than
duplicating implementation.

## 8. AI agent and tool architecture

Required components:

- Provider-neutral chat/completion/streaming interface
- Provider adapters for DeepSeek and future providers
- Model registry and task-specific model selection
- Capability/tool registry with discovery and schemas
- Context assembly and scoped resource handles
- Planner/orchestrator for multi-step workflows
- Typed tool executor
- Execution history and event stream
- Cancellation, resumability, checkpoints, and retry policy
- Approval gates for destructive or external operations
- Audit log and secret isolation

The AI must not receive raw SQL or unrestricted filesystem access. Tool calls
must be bounded by project, path, data, network, and action scopes.

## 9. Visual and chart architecture

Visual objects should become typed persistent entities, not ad hoc canvas JSON:

- Shared visual document/entity identity
- Nodes, shapes, groups, regions, annotations
- Chart datasets and series
- Domain-neutral coordinate geometry
- Domain-specific adapters for price/time and generic numeric/time charts
- Lines, rectangles, zones, paths, swing points, and measurements
- Selection, editing, undo/redo, snapshots, and history
- Import/export and visual-to-domain relation records
- Human editing of AI-generated objects
- Capability operations for AI create/update/delete/list/inspect

Chart financial behavior can remain a market adapter, while the geometry and
visual-persistence engine stay domain-neutral.

## 10. Security and permissions

Define permission scopes for:

- Project read/write/archive
- Database read/write/export/import
- Filesystem root/read/write/delete
- Internet and external-source access
- API/network operations
- Provider API and secret access
- Workflow execution/cancellation
- Destructive actions
- Cross-project transfer

Security invariants:

- UI and AI receive explicit authorization context.
- High-impact operations require approval.
- Path scopes are canonicalized and checked before I/O.
- SQL is parameterized and schema operations are allowlisted.
- Secrets are never returned in tool/audit payloads.
- Audit logs record actor, capability, scope, before/after identity, result,
  retry, and approval.
- Every background job has cancellation and resource limits.

## 11. Database strategy

Phase 0 decision:

- Do not migrate the legacy database as user data.
- Preserve domain knowledge and contract migration intent.
- Create a clean initial schema.
- Build a numbered, transactional migration system.
- Enforce foreign keys on every connection.
- Use project ownership and provenance as primary schema concepts.
- Store immutable source and visual/model/experiment revisions.
- Add schema, integrity, foreign-key, transaction, and backup tests.
- Define asset manifests for media, documents, Trace, and Parquet files.

## 12. Verified Wails v3 technical assessment

### Official release metadata

- Latest GitHub release observed: Wails `v3.1.0`
- Published: 2026-11-01
- Official tag and module: `github.com/wailsapp/wails/v3`
- Official module requirement: Go `1.26.0`
- Official module toolchain: `go1.26.10`
- Official install docs require Go 1.26 or later.
- Official docs for Windows require WebView2 and state Windows AMD64/ARM64
  support.
- Official Wails README describes native rendering engines and React-compatible
  frontend integration through bundled templates and generated TypeScript bindings.

The release response is for the stable GitHub latest release; the repository
README and installation guide still display CLI examples with `v3.0.0`, so
new projects should pin and verify the exact desired CLI/module pair rather
than copy an assumed tag.

### Platform

- Windows: WebView2.
- macOS: Xcode command line tools.
- Linux: GTK4/WebKitGTK 6.0 in the default stack; legacy GTK3 may be temporary.

### Integration

- Wails provides Go methods bound for JavaScript/TypeScript.
- Wails provides generated TypeScript definitions.
- Wails provides application/custom events and native dialogs/windows.
- React is supported as a frontend template; exact generated binding output
  must be tested in a proof project.
- Hot reload/build behavior must be accepted in a Windows proof project.

No migration implementation should be started until a small Wails v3 Windows
proof project verifies WebView2 startup, React binding generation, events,
dialog/file APIs, and packaging.

## 13. Migration dependency graph

```text
Phase 0 report
  -> Wails v3 Windows proof
  -> Capability registry and contracts
  -> Clean database/migration foundation
  -> Project/authorization/audit services
  -> Shared source/provenance service
  -> Canvas/visual object service
  -> Scientific domain services
  -> AI provider/tool adapter
  -> Workflow/job runner
  -> Market/chart adapters
  -> Forge/code-intelligence adapter
  -> External API
  -> Electron removal
  -> Rust/Tauri retirement
```

Parallel proof work can proceed for Wails and the capability registry, but
Electron removal should wait until the Wails shell passes desktop acceptance.

## 14. Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| Wails v3 maturity/API drift | High | Pin v3.1.0, retain official module/tag evidence, run Windows proof |
| Dual desktop shells released simultaneously | High | One shell owner; explicit cutover gates |
| Legacy Rust behavior lost in translation | High | Capability-by-capability contract and parity tests |
| Database migration ambiguity | High | Fresh schema, numbered migrations, integrity fixtures |
| Project/permission leakage | High | Repository and service authorization checks |
| AI unrestricted I/O | Critical | Scope-aware tools, approval gates, audit |
| External asset loss on restore | High | Asset manifest, hashes, backup/restore tests |
| SQLite lock and worker contention | Medium | Transaction boundaries, bounded jobs, lock measurements |
| Large React bundle and split architecture | Medium | Preserve React 19; code split and profile |
| Test harness cannot start Rust test binary | High | Reproduce Windows manifest/dependency fix before Phase 1 acceptance |
| Existing user database diversity | High | Until migration, inventory deployments; do not assume one schema |

## 15. Recommended implementation sequence

1. Approve a pinned Wails v3.1.0/Go 1.26 proof baseline.
2. Create one documentation location and capability registry.
3. Implement a clean SQLite schema and migration runner.
4. Introduce project, authorization, audit, and actor context.
5. Translate file/project/source/provenance capabilities first.
6. Translate canvas and visual entities.
7. Translate scientific domains and existing project context.
8. Add provider-neutral AI tools over registered capabilities.
9. Translate market/chart, forge, worker, Trace, and assets.
10. Add external API and desktop package parity.
11. Run Wails desktop acceptance and backup/recovery tests.
12. Remove Electron after a documented cutover.
13. Remove Rust/Tauri only after domain parity and no runtime references remain.

## 16. Test and acceptance strategy

Required suites for the target:

- Go unit tests for domain rules and pure geometry.
- SQLite migration/rollback/foreign-key/integrity tests.
- Repository transaction and project-isolation tests.
- Capability contract/schema/authorization tests.
- Wails binding mock/integration tests.
- React state/controller and mounted UI tests.
- AI tool discovery and end-to-end orchestrator tests.
- Approval, cancellation, retry, and audit tests.
- External API authentication/authorization tests.
- Desktop startup, file dialog, hot reload, update, and packaging tests.
- Real backup/restore including external assets.
- Electron/Tauri parity matrix and migration soak tests.

No successful frontend build or TypeScript check should be treated as
application functionality.

## 17. Exact Phase 1 plan pending approval

Phase 1 should contain only proof and foundation work:

1. Create a minimal Wails v3.1.0 Windows application using React/TypeScript.
2. Verify Go 1.26.10, WebView2, native window, file dialog, events, bindings,
   hot reload, and packaging.
3. Define capability contracts and a Go capability registry.
4. Create the clean SQLite migration and repository foundation.
5. Implement project and audit contexts.
6. Implement one low-risk capability end to end across UI, service, Go
   repository, and external/tool/API adapter.
7. Publish test evidence and a cutover/retirement gate.

Do not begin Electron deletion or Rust/Tauri retirement in Phase 1.

## 18. Commands executed and exit codes

| Command | Exit code | Result |
|---|---:|---|
| `git branch --show-current; git rev-parse HEAD; git status --short --branch; git remote -v; git tag --sort=-creatordate` | 0 | Baseline captured |
| `go version; go env GOTOOLCHAIN; node --version; pnpm --version; rustc --version; cargo --version` | 0 | Toolchains captured |
| `pnpm tsc --noEmit` | 0 | Passed |
| `cargo check --manifest-path src-tauri/Cargo.toml -j 1` | 0 | Passed with existing migration warnings |
| `pnpm test:frontend` | 0 | 49 passed, 0 failed |
| `pnpm test:electron` | 0 | 5 passed, 0 failed |
| `pnpm test:sidecar` | 0 | 2 passed, 0 failed |
| `pnpm build` | 0 | Passed; existing mixed-import and chunk-size warnings |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib -j 1` | 0xc0000139 | Full Rust executable did not start |

Official Wails release/module and installation information was verified from
the official Wails GitHub repository and release metadata. The Wails
documentation website itself returned HTTP 403 to the document fetch client,
so source-tagged official GitHub files were used.

## 19. Capability-by-capability migration roadmap

| Capability | Migration priority | Exit criterion |
|---|---|---|
| Project lifecycle | P0 | Project isolation and archive rules pass tests |
| Authorization/audit | P0 | Every write is authorized and audited |
| Database/migrations | P0 | Clean install, integrity, rollback, and backup pass |
| Filesystem/service | P0 | Canonical scope, read/write/delete, and cancellation pass |
| Source/provenance | P1 | Exact source/version/location/claim lineage preserved |
| Canvas/visual entities | P1 | Human and AI can create/edit/delete typed objects |
| Board/snapshot/history | P1 | Snapshot and undo/redo parity proven |
| Research | P1 | Project-scoped and exact-reference workflows pass |
| Hypothesis/experiment | P1 | Typed design/run/data/analysis/finding chain exists |
| Analysis/evidence | P1 | Temporal and provenance rules enforced |
| Model/validation/replication | P1 | Versions/evaluations/replications are project-safe |
| Learning/mastery | P2 | Assessments and competency evidence are typed |
| Knowledge transfer | P2 | Attribution, rights, lineage, and archive rules pass |
| AI/provider/tool system | P1 | Provider-independent discovery and execution pass |
| Workflow/job system | P1 | Queue, cancel, resume, retry, audit, and history pass |
| Market/chart | P2 | Domain-neutral geometry plus market adapter verified |
| Forge/code intelligence | P2 | Index/search/impact/patch parity and safety pass |
| Trace | P2 | Canonical entity view or explicit redesign accepted |
| Media/documents | P2 | Asset manifest and referential integrity pass |
| External API | P1 | Authenticated capability parity and audit pass |
| Import/export/backup | P1 | Database plus asset recovery pass |
| Updater | P2 | Signed update lifecycle and rollback pass |
| Electron removal | P3 | Wails parity and package acceptance complete |
| Rust/Tauri retirement | P4 | No runtime contract remains; legacy archive retained |

## 20. Critical architectural decisions

1. Wails v3 is the target desktop shell; Electron is transitional, not
   architectural ground truth.
2. Go is the target backend; Rust domain behavior is migration evidence and
   reusable logic, not permanent shell infrastructure.
3. React/TypeScript remains the likely UI implementation during the cutover.
4. SQLite remains local-first ground truth; a clean schema and migration
   ledger replace the split legacy initializers.
5. AI tools, external APIs, CLI/jobs, and Wails UI bindings converge on shared
   registered application capabilities.
6. AI receives scope-aware capability tools, never unrestricted SQL or
   filesystem access.
7. Visual/chart objects become persistent typed entities.
8. Electron and Rust/Tauri removal occur only after executable acceptance
   gates.

## 21. Unresolved questions and blockers

- Is Wails v3.1.0 the required pinned runtime, or should `v3.0.0` remain the
  compatibility baseline?
- Does generated Wails binding behavior satisfy all existing React service
  contracts and streaming events?
- Which Electron-only behavior is required for product release?
- Which Rust domain modules are truly reusable versus coupled to Tauri IPC?
- What are deployed schema/user-database variants and asset layouts?
- What authentication/actor model should external API and AI audit events use?
- Which AI providers are mandatory for the initial Go integration?
- Are cross-project transfer, project archive, and destructive workflow rules
  already legally/product-defined?
- Which Rust test harness manifest/dependency issue causes `STATUS_ENTRYPOINT_NOT_FOUND`
  in this Windows environment?

## 22. Final Phase 0 conclusion

Phase 0 is complete and found enough implementation evidence to define a safe
migration architecture. It does not establish that the current application is
runtime-clean or migration-ready. Phase 1 must begin with a pinned Wails proof
and capability/SQLite foundation, and must not delete legacy code or proceed
without explicit approval of this report.
