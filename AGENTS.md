# AGENTS.md — Autonomous Engineering Operating Standard for VORA

> **Role & Mental Model**: You are a deterministic, senior systems and UI engineer working on VORA (a local-first visual canvas & spatial knowledge workspace powered by Tauri v2, React 19, ReactFlow / @xyflow, and SQLite).
> **Golden Directive**: You MUST NOT guess architecture, invent unauthorized dependencies, bypass module boundaries, or write untyped mutations.

---

## 1. System Core Directives (Non-Negotiable Invariants)

1. **Spec-Driven First**: No new feature, database table, or major refactoring should occur without a verified plan or markdown spec in `specs/`. If none exists, draft the specification first.
2. **Local-First & Durable Ground Truth**:
* The SQLite database on disk (`vora_workspace.db`) managed via `src-tauri/src/db.rs` is the durable source of truth.
* Zustand stores (`src/store/`) manage reactive UI/canvas states and memory caches.
* Any persisted state change MUST follow a strict linear flow: UI Action -> Store/Hook -> `src/services/dbService.ts` -> Tauri IPC Invoke -> `resolver.rs` -> `db.rs` -> SQLite.


3. **Strict Type Safety**:
* Zero `any`, zero `// @ts-ignore`, zero unchecked runtime type-casting.
* All domain interfaces MUST reside in `src/types/index.ts`.
* Backend Rust models (`struct` with `Serialize, Deserialize`) and Frontend TypeScript interfaces MUST mirror each other 1:1.


4. **Automated Verification Loop**: Every code generation or modification turn MUST be verified with:
* Frontend check: `pnpm tsc --noEmit`
* Backend check: `cargo check --manifest-path src-tauri/Cargo.toml`



---

## 2. Directory Layout & Access Control Boundaries

Every directory has a strict single responsibility. Cross-layer violations are forbidden.

```text
vora/
├── specs/                   # Technical specifications, schemas, and task plans
├── src/
│   ├── components/
│   │   ├── canvas/          # ReactFlow nodes, canvas toolbars, replay controllers
│   │   ├── layout/          # Shell, Sidebars (LeftSidebar, RightDrawer), Panels
│   │   └── modals/          # Dialogs (CommitModal, SettingsModal, FullscreenDocModal)
│   ├── hooks/               # Custom adapters & workflow logic (useVoraFlow, etc.)
│   ├── store/               # Central Zustand state management (useAppStore)
│   ├── services/            # Tauri IPC bridges & SQLite query callers (dbService)
│   ├── types/               # Pure single-source-of-truth TypeScript definitions
│   ├── utils/               # Pure mathematical, text processing, and layout utilities
│   ├── App.tsx              # Root layout & view controller
│   └── main.tsx             # Application bootstrap & providers
└── src-tauri/
    ├── src/
    │   ├── db.rs            # SQLite connection pool, migrations, and low-level queries
    │   ├── resolver.rs      # Tauri command handlers (IPC entrypoints)
    │   ├── schema.sql       # Initial database schemas and table definitions
    │   ├── lib.rs           # Tauri builder registration & plugins
    │   └── main.rs          # Native desktop entrypoint
    ├── Cargo.toml           # Rust crates and dependencies
    └── tauri.conf.json      # Window settings, permissions, and security capabilities

```

### Boundary Access Rules:

* **`components/`** may only call `hooks/`, `store/`, and read `types/`. Never call Tauri `invoke()` directly inside React visual components.
* **`services/`** is the ONLY frontend directory permitted to import `@tauri-apps/api/core` (`invoke`).
* **`src-tauri/src/resolver.rs`** receives IPC commands and delegates all query execution to **`db.rs`**. Never execute inline SQL inside `resolver.rs` or `lib.rs`.

---

## 3. Standard Step-by-Step Feature Workflow

When tasked with implementing a new feature or resolving a bug, execute in this exact sequence:

1. **Context Discovery**: Read related type definitions in `src/types/` and existing hooks in `src/hooks/`.
2. **Contract Update**: If new state or database fields are needed:
* Update `src-tauri/src/schema.sql` (and migrations in `db.rs`).
* Update `src/types/index.ts`.


3. **Backend Implementation**: Expose commands via `db.rs` -> register in `resolver.rs` -> register command in `lib.rs`.
4. **Frontend Service & Store**: Add the typed invoke wrapper in `src/services/` and integrate into `src/store/useAppStore.ts`.
5. **UI & Canvas Integration**: Connect state to `src/hooks/useVoraFlow.ts` and render inside appropriate `components/`.
6. **Self-Healing Verification**: Run `pnpm tsc --noEmit` and `cargo check`. Fix any compilation or type mismatches immediately.

---

## 4. Conventional Commit Standard

All commits generated must strictly follow the Conventional Commits format:
`<type>(<scope>): <concise action-oriented message>`

* **Allowed Types**: `feat`, `fix`, `refactor`, `perf`, `docs`, `chore`, `style`, `test`
* **Allowed Scopes**: `canvas`, `layout`, `store`, `tauri`, `db`, `config`, `types`, `spec`, `deps`

**Examples**:

* `feat(canvas): add smart auto-resizing text node with markdown support`
* `fix(layout): prevent floating toolbox from overlapping left sidebar`
* `chore(tauri): update Rusqlite migration query for node positions`
