# Inquiry Queue / Planning Focused Recon

> **Scope:** Read-only audit for an additive Inquiry Queue and Planning Triage feature.  
> **Evidence labels:** **FACT** = directly visible in repository source; **INFERENCE** = recommendation or consequence derived from those facts. `UNKNOWN` means repository evidence does not settle the point.

## 1. QUESTIONS BOARD — FULL STACK TRACE

### Schema and Rust model

**FACT —** `questions` is initialized by `mind::questions::init_schema`; it has no declared foreign key or trigger in the table definition. It has one secondary index. (`src-tauri/src/mind/questions.rs:16-28`)

| Column | SQLite definition | Constraints/default |
|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` |
| `title` | `TEXT` | `NOT NULL`; `CHECK(length(trim(title)) BETWEEN 1 AND 200)` |
| `description` | `TEXT` | `NOT NULL`; no default |
| `created_at` | `INTEGER` | `NOT NULL`; no default |
| `updated_at` | `INTEGER` | `NOT NULL`; no default |

Index: `idx_questions_updated_at(updated_at DESC, created_at DESC, id)`. No other questions index is declared in the repository’s schema initializer. (`src-tauri/src/mind/questions.rs:16-28`)

**FACT — Rust model (`QuestionItem`), complete source:** (`src-tauri/src/mind/questions.rs:4-12`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}
```

`QUESTION_COLUMNS` selects the same five fields; `map_question` maps them from SQLite in that order. (`src-tauri/src/mind/questions.rs:14, 89-101`)

**FACT — Complete questions schema initializer:** (`src-tauri/src/mind/questions.rs:16-28`)

```rust
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS questions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
            description TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_questions_updated_at
            ON questions(updated_at DESC, created_at DESC, id);",
    )
}
```

### Rust IPC commands

**FACT —** Command bodies lock `AppState.db`, delegate to `mind::questions`, and convert Rust/SQLite errors to `String`. `create_question` creates a UUIDv7-based `question_…` ID; `save_question` updates title/description and timestamp; delete returns `()` or an error. (`src-tauri/src/lib.rs:279-314`)

```rust
fn get_questions(state: State<AppState>) -> Result<Vec<mind::questions::QuestionItem>, String>
fn create_question(
    state: State<AppState>, title: String, description: String,
) -> Result<mind::questions::QuestionItem, String>
fn save_question(
    state: State<AppState>, id: String, title: String, description: String,
) -> Result<mind::questions::QuestionItem, String>
fn delete_question(state: State<AppState>, id: String) -> Result<(), String>
```

These are the complete questions IPC signatures; their command implementations are at `src-tauri/src/lib.rs:279-314`.

**FACT — Registration:** `get_questions`, `create_question`, `save_question`, and `delete_question` are in `tauri::generate_handler!` at `src-tauri/src/lib.rs:1333-1336`.

### TypeScript service and Zustand

**FACT —** The typed invoke wrappers live in the shared `dbService` object. (`src/mind/services/dbService.ts:155-178`)

```ts
async getQuestions(): Promise<QuestionItem[]> {
  return await invoke<QuestionItem[]>("get_questions");
},
async createQuestion(title: string, description: string): Promise<QuestionItem> {
  return await invoke<QuestionItem>("create_question", { title, description });
},
async saveQuestion(id: string, title: string, description: string): Promise<QuestionItem> {
  return await invoke<QuestionItem>("save_question", { id, title, description });
},
async deleteQuestion(id: string): Promise<void> {
  await invoke("delete_question", { id });
},
```

`QuestionItem` in `src/types/index.ts` mirrors the Rust fields as camelCase (`id`, `title`, `description`, `createdAt`, `updatedAt`). (`src/types/index.ts:82-88`)

**FACT —** `useAppStore` holds `questions`, `questionSaveStatus`, and `questionSaveError`. Loading reconciles unsaved in-memory edits; create/load/save results are sorted by descending `updatedAt`, descending `createdAt`, then ascending ID. Title/description edits update memory immediately and debounce persistence by 500ms; per-ID save queues serialize writes. (`src/shared/store/useAppStore.ts:36-42, 91-102, 385-496`)

The complete questions store state/actions are at `src/shared/store/useAppStore.ts:91-102, 385-500`; this source range includes DB load reconciliation, optimistic updates, the 500ms debounce, per-ID queued saves, visible save errors, and deletion. The state declaration is `questions: QuestionItem[]`, save status/error maps, and `loadQuestions/createQuestion/updateQuestion/flushQuestion/deleteQuestion` (`src/shared/store/useAppStore.ts:91-102`).

**FACT — Persistence distinction:** Zustand `partialize` persists only board cards and `executedTasks`; `questions` are not serialized into Zustand storage. SQLite is the durable source for question rows, while the store array/statuses are a runtime cache. (`src/shared/store/useAppStore.ts:553-562`; DB CRUD at `src-tauri/src/mind/questions.rs:30-87`)

### UI, filtering, and tests

**FACT —** `QuestionsBoardPage` is a React component with one prop, `onGoHome: () => void`. It loads on mount, tracks selected question/create/loading/error states locally, and invokes store actions for CRUD. (`src/shared/layout/QuestionsBoardPage.tsx:6-49`)

Relevant source excerpts, with layout-only markup omitted:

```tsx
interface QuestionsBoardPageProps {
  onGoHome: () => void;
}

export const QuestionsBoardPage: React.FC<QuestionsBoardPageProps> = ({ onGoHome }) => {
  const questions = useAppStore((state) => state.questions);
  const saveStatus = useAppStore((state) => state.questionSaveStatus);
  const saveError = useAppStore((state) => state.questionSaveError);
  const loadQuestions = useAppStore((state) => state.loadQuestions);
  const createQuestion = useAppStore((state) => state.createQuestion);
  const updateQuestion = useAppStore((state) => state.updateQuestion);
  const flushQuestion = useAppStore((state) => state.flushQuestion);
  const deleteQuestion = useAppStore((state) => state.deleteQuestion);
  const [selectedQuestionId, setSelectedQuestionId] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  // ...
};
```

Source: `src/shared/layout/QuestionsBoardPage.tsx:6-49`. The omitted handlers create rows, flush edits before returning home/list, ask `window.confirm` before deletion, and render retryable load/save errors (`:51-148`). The detail view edits title and Markdown description; the list view provides a create form, loading and empty states, and a card grid (`:219-365`). Each card sets `selectedQuestionId`; the detail view is conditional on that selected row (`:219-220, 330-365`).

**FACT — Filtering/sorting:** The Questions Board has no visible search/filter control. It renders all loaded `questions`; ordering comes from the store’s `sortQuestions` helper and the backend’s matching `ORDER BY`. (`src/shared/layout/QuestionsBoardPage.tsx:311-365`; `src/shared/store/useAppStore.ts:36-42`; `src-tauri/src/mind/questions.rs:30-36`)

**FACT — Tests:** `questions.rs` has Rust unit tests for CRUD, list order, empty/long descriptions, invalid titles, and missing updates/deletes (`src-tauri/src/mind/questions.rs:104-165`). No Questions Board frontend test file was found; `package.json` has build scripts but no test script (`package.json:6-17`).

## 2. PLANNING — FULL STACK TRACE

### Schema, Rust model, and commands

**FACT —** `task_flow_items` is declared in `mind::task_flow::init_schema`. (`src-tauri/src/mind/task_flow.rs:37-77`)

| Column | SQLite definition | Constraints/default |
|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` |
| `title` | `TEXT` | `NOT NULL`; length check 1–200 |
| `description` | `TEXT` | `NOT NULL DEFAULT ''` |
| `task_type` | `TEXT` | `NOT NULL DEFAULT 'task'`; check in `('task','ai')` |
| `completed` | `INTEGER` | `NOT NULL DEFAULT 0`; check in `(0,1)` |
| `position` | `INTEGER` | `NOT NULL` |
| `created_at` | `INTEGER` | `NOT NULL` |
| `updated_at` | `INTEGER` | `NOT NULL` |

Indexes are `idx_task_flow_items_position(position, created_at, id)` and `idx_task_flow_items_type_position(task_type, position, created_at, id)`. The schema initializer also adds legacy `description` and `task_type` columns after checking `PRAGMA table_info`. (`src-tauri/src/mind/task_flow.rs:37-77`)

**FACT — Rust model and enum:** (`src-tauri/src/mind/task_flow.rs:5-33`)

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskFlowType { Task, Ai }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskFlowItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub task_type: TaskFlowType,
    pub completed: bool,
    pub position: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
```

**FACT — Complete Task Flow schema initializer:** (`src-tauri/src/mind/task_flow.rs:37-77`)

```rust
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_flow_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
            description TEXT NOT NULL DEFAULT '',
            task_type TEXT NOT NULL DEFAULT 'task' CHECK(task_type IN ('task', 'ai')),
            completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
            position INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_task_flow_items_position
            ON task_flow_items(position, created_at, id);",
    )?;
    let (has_description, has_task_type) = {
        let mut statement = conn.prepare("PRAGMA table_info(task_flow_items)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        let columns = columns.collect::<Result<Vec<_>>>()?;
        (
            columns.iter().any(|name| name == "description"),
            columns.iter().any(|name| name == "task_type"),
        )
    };
    if !has_description {
        conn.execute(
            "ALTER TABLE task_flow_items ADD COLUMN description TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !has_task_type {
        conn.execute(
            "ALTER TABLE task_flow_items ADD COLUMN task_type TEXT NOT NULL DEFAULT 'task'",
            [],
        )?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_task_flow_items_type_position
            ON task_flow_items(task_type, position, created_at, id);",
    )?;
    Ok(())
}
```

**FACT — IPC signatures and behavior:** (`src-tauri/src/lib.rs:189-253`)

| Command | Signature (excluding boilerplate state argument where noted) | Behavior |
|---|---|---|
| `get_task_flow_items` | `(state, task_type: TaskFlowType) -> Result<Vec<TaskFlowItem>, String>` | Lists one task type, ordered by position. |
| `create_task_flow_item` | `(state, title: String, task_type: TaskFlowType) -> Result<TaskFlowItem, String>` | Validates title; appends at `MAX(position)+1`. |
| `update_task_flow_item` | `(state, id: String, title: String, description: String) -> Result<TaskFlowItem, String>` | Saves title and description. |
| `set_task_flow_item_completed` | `(state, id: String, completed: bool) -> Result<TaskFlowItem, String>` | Persists completion toggle. |
| `reorder_task_flow_items` | `(state, ids: Vec<String>, task_type: TaskFlowType) -> Result<Vec<TaskFlowItem>, String>` | Validates and transactionally stores ordering. |
| `delete_task_flow_item` | `(state, id: String) -> Result<(), String>` | Deletes one task. |

All six names are registered in `generate_handler!` at `src-tauri/src/lib.rs:1313-1318`.

### TypeScript service and store

**FACT —** TypeScript type fields mirror the Rust DTO; service wrappers invoke the same command names. (`src/types/index.ts:1-11`; `src/mind/services/dbService.ts:180-202`)

```ts
async getTaskFlowItems(taskType: TaskFlowType): Promise<TaskFlowItem[]>
async createTaskFlowItem(title: string, taskType: TaskFlowType): Promise<TaskFlowItem>
async updateTaskFlowItem(id: string, title: string, description: string): Promise<TaskFlowItem>
async setTaskFlowItemCompleted(id: string, completed: boolean): Promise<TaskFlowItem>
async reorderTaskFlowItems(ids: string[], taskType: TaskFlowType): Promise<TaskFlowItem[]>
async deleteTaskFlowItem(id: string): Promise<void>
```

**FACT —** The store has one `taskFlowItems` array and actions that call `dbService`; it updates the array only after service calls succeed. This array is not included in Zustand’s `partialize` persisted subset. (`src/shared/store/useAppStore.ts:79-85, 265-299, 553-562`)

### UI, mount, and relationships

**FACT —** `TaskFlowPage` accepts `taskType: TaskFlowType`; the same component serves Task Flow and AI Tasks. It loads the selected list in an effect, has a local error/reload path, create form, empty/loading states, drag reorder, inline title editor, detail dialog, completion toggle, and delete action. (`src/shared/layout/TaskFlowPage.tsx:6-149, 150-435`)

Relevant excerpts (unrelated presentation markup omitted):

```tsx
interface TaskFlowPageProps {
  taskType: TaskFlowType;
}

const handleDrop = async (targetTask: TaskFlowItem): Promise<void> => {
  if (!draggedTaskId || draggedTaskId === targetTask.id || isBusy) return;
  const sourceIndex = tasks.findIndex((task) => task.id === draggedTaskId);
  const targetIndex = tasks.findIndex((task) => task.id === targetTask.id);
  if (sourceIndex < 0 || targetIndex < 0) return;
  const nextTasks = [...tasks];
  const [movedTask] = nextTasks.splice(sourceIndex, 1);
  if (!movedTask) return;
  nextTasks.splice(targetIndex, 0, movedTask);
  await runAction(() => reorderTasks(nextTasks.map((task) => task.id), taskType));
};
```

Source: `src/shared/layout/TaskFlowPage.tsx:6-14, 136-148`. Rows show their one-based order and use native drag events; the checkbox-like button calls `setTaskCompleted` and exposes `aria-pressed`. (`src/shared/layout/TaskFlowPage.tsx:232-295`). Empty/loading/error handling is at `:199-230`; editing dialog at `:363-434`.

**FACT — Mounting:** `main.tsx` has routes for `/home`, `/sources`, `/chart`, `/scale-lab`, and `/snapshots`; there is no Planning or Questions Board route (`src/main.tsx:50-61`). `App.tsx` chooses `TaskFlowPage` or `QuestionsBoardPage` in its in-canvas workspace render (`src/App.tsx:2505-2521`). Sidebar callbacks close peer panels when opening Task Flow, AI Tasks, or Questions Board (`src/App.tsx:2416-2458`); sidebar labels/active state are in `src/shared/layout/LeftSidebar.tsx:104-164`.

**FACT — Relationships:** `questions` and `task_flow_items` are distinct tables, with separate models, IPC commands, store actions, and views. Neither schema declares an FK or link column to the other table; no shared tag or cross-reference is in the inspected question/task APIs. (`src-tauri/src/mind/questions.rs:16-28`; `src-tauri/src/mind/task_flow.rs:37-77`; `src-tauri/src/lib.rs:189-314`; `src/shared/store/useAppStore.ts:79-102`)

**FACT — Task Flow ordering:** `position` is the persisted order field. Creation chooses `COALESCE(MAX(position) + 1, 0)` scoped to task type; reorder assigns zero-based positions in a transaction after checking that the submitted IDs exactly match that type’s rows. (`src-tauri/src/mind/task_flow.rs:90-114, 156-189`)

**FACT — Tests:** Task Flow has Rust unit coverage for CRUD/completion/order, invalid reorder IDs, legacy schema migration, and separate AI/regular lists (`src-tauri/src/mind/task_flow.rs:236-365`). No frontend Task Flow test file was found; root scripts do not define a test runner (`package.json:6-17`).

## 3. THE QUESTIONS ↔ PLANNING INTERSECTION

| Question | Finding |
|---|---|
| Existing relation? | **FACT:** No FK or cross-link is present in either schema or their model/API surface (`src-tauri/src/mind/questions.rs:16-28`; `src-tauri/src/mind/task_flow.rs:37-77`; `src-tauri/src/lib.rs:189-314`). |
| Natural Triage host? | **INFERENCE:** Add an inner view switch in `TaskFlowPage`, rendering an `InquiryTriagePanel` inside its existing Planning workspace. Its mount point already distinguishes Task Flow/AI Tasks from other workspace panels (`src/App.tsx:2505-2507`). Keep Task Flow as the default view to preserve current behavior (`src/shared/layout/TaskFlowPage.tsx:150-203`). |
| Reusable priority? | **FACT:** `position` is an explicit zero-based saved order and the UI presents its one-based index as a numbered circle; drag/drop changes it (`src-tauri/src/mind/task_flow.rs:90-114, 156-189`; `src/shared/layout/TaskFlowPage.tsx:232-279`). It is ordering, not a stored impact/feasibility/urgency score. |
| Shared UI primitive? | **FACT:** No dedicated tab, radio, or prioritization component was found in the inspected Planning/Questions components. `RightDrawer` has a local filter dropdown that switches among All/Cards/Item Groups/Trash and is the closest in-page switching example (`src/shared/layout/RightDrawer.tsx:970-1023`). Task Flow’s index badge and drag controls are the closest priority affordances (`src/shared/layout/TaskFlowPage.tsx:260-282`). |

The Questions Board currently uses a single selected-card state to switch list/detail content. That is a local selection pattern, not a persistent relationship with Planning. (`src/shared/layout/QuestionsBoardPage.tsx:20-30, 219-220, 330-365`)

## 4. ADDITIVE MIGRATION PATTERN — EXACT RECIPE

### Existing PRAGMA / ALTER pattern

**FACT —** Task Flow uses the most direct safe precedent: collect current column names from `PRAGMA table_info`, conditionally execute static `ALTER TABLE` statements, and propagate every SQLite error with `?`. (`src-tauri/src/mind/task_flow.rs:52-73`)

```rust
let (has_description, has_task_type) = {
    let mut statement = conn.prepare("PRAGMA table_info(task_flow_items)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let columns = columns.collect::<Result<Vec<_>>>()?;
    (
        columns.iter().any(|name| name == "description"),
        columns.iter().any(|name| name == "task_type"),
    )
};
if !has_description {
    conn.execute(
        "ALTER TABLE task_flow_items ADD COLUMN description TEXT NOT NULL DEFAULT ''",
        [],
    )?;
}
if !has_task_type {
    conn.execute(
        "ALTER TABLE task_flow_items ADD COLUMN task_type TEXT NOT NULL DEFAULT 'task'",
        [],
    )?;
}
```

`canvas_db::ensure_board_column` is a generic helper with the same PRAGMA-check/ALTER/error-return pattern, but it is domain-specific to board columns and constructs SQL with a table-name argument; it is not the appropriate owner for questions migrations. (`src-tauri/src/mind/canvas_db.rs:759-773`)

### Owner, constraints, and metadata

**FACT —** The `questions` table’s only check is the title length/trim check; no trigger is created alongside it. (`src-tauri/src/mind/questions.rs:16-28`). The migration should be owned by `src-tauri/src/mind/questions.rs`, next to its schema, CRUD queries, and tests. Workspace schema initialization already calls `questions::init_schema`, so `portable_db.rs` need not change for an in-place migration (`src-tauri/src/mind/portable_db.rs:883-892`).

**FACT — `_vora_meta`:** Market initialization creates `_vora_meta(key TEXT PRIMARY KEY, value TEXT)` and uses keyed markers; some historical migrations use ignored errors/`.ok()` and are not a safe template for this change (`src-tauri/src/market/db.rs:113-118, 127-174`). `initialize_workspace_schema` calls `questions::init_schema` before `market::db::init_schema`, so `_vora_meta` is not guaranteed to have been created earlier on a fresh DB (`src-tauri/src/mind/portable_db.rs:883-892`).

**INFERENCE —** A version marker can gate this additive migration, but the migration must first ensure `_vora_meta` exists (without adding a separate metadata table), and only write its marker after every column is checked/added successfully. Alternatively, the six PRAGMA column checks are already idempotent and avoid coupling the question migration to the market metadata owner. The repository has no questions-specific schema version key today (`src-tauri/src/mind/questions.rs:16-28`; metadata keys are market-owned at `src-tauri/src/market/db.rs:113-174`).

### Error-propagating idempotent recipe (recommended shape)

**INFERENCE —** Apply this static-name pattern inside `questions::init_schema`, preferably in a transaction; do not copy `_`/`.ok()` error suppression used by legacy market migrations. If using a marker, create/verify `_vora_meta` before reading it and commit the marker only after all alterations succeed.

```rust
let columns = {
    let mut statement = conn.prepare("PRAGMA table_info(questions)")?;
    statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>>>()?
};
if !columns.iter().any(|name| name == "kind") {
    conn.execute(
        "ALTER TABLE questions ADD COLUMN kind TEXT NOT NULL DEFAULT 'question'",
        [],
    )?;
}
// Repeat a static check and `conn.execute(...)?` for each remaining column.
```

This follows the checked migration shape in `src-tauri/src/mind/task_flow.rs:52-73`; static SQL avoids accepting an untrusted table/column identifier.

**INFERENCE — Reversibility:** The operation is additive and preserves legacy rows with defaults, but once users save kind/status/score values, rollback by dropping columns loses that metadata. Mark rollback **irreversible after metadata is written** for that one-line reason; do not describe the migration as data-preserving reversible.

## 5. UI PATTERNS FOR PRIORITIZATION

### Sortable/reorderable list

**FACT —** Task Flow is the best existing example. `handleDrop` computes a new array order and calls the store with all IDs; backend reorder verifies the submitted ID set and updates `position` transactionally. (`src/shared/layout/TaskFlowPage.tsx:136-148`; `src-tauri/src/mind/task_flow.rs:156-189`)

```tsx
const nextTasks = [...tasks];
const [movedTask] = nextTasks.splice(sourceIndex, 1);
if (!movedTask) return;
nextTasks.splice(targetIndex, 0, movedTask);
await runAction(() => reorderTasks(nextTasks.map((task) => task.id), taskType));
```

### Badge/chip

**FACT —** There is no existing semantic priority/status chip for questions. Closest visual example is the Task Flow index circle, which shows `index + 1` and changes color for completed tasks (`src/shared/layout/TaskFlowPage.tsx:260-272`). The Questions Board renders a static “Question” kind label (`src/shared/layout/QuestionsBoardPage.tsx:330-347`); it is not data-driven yet.

### Selection

**FACT —** Questions Board’s card button selects a single question by ID, and the same page conditionally displays that question’s editor (`src/shared/layout/QuestionsBoardPage.tsx:20-30, 219-220, 330-365`). This is the closest single-select pattern. No radio input was found in these two feature components.

### Tabs/panel switch

**FACT —** `TaskFlowPage` itself has no internal tabs. `RightDrawer` offers a local dropdown that updates `currentFilter`, closes peer menus, and highlights the active filter (`src/shared/layout/RightDrawer.tsx:970-1023`). The root `App` switches top-level workspace panels with booleans rather than a Planning tab system (`src/App.tsx:367-374, 2505-2521`).

**INFERENCE —** Reuse the active-button styling and conditional panel-render pattern, but keep the switch local to Planning; do not route Triage through `App`’s root panel state.

## 6. BUILD PERFORMANCE RISK

### Files in the smallest additive implementation

**INFERENCE —** The following set supports the requested lifecycle without a new route, new dependency, or change to `portable_db.rs` (which already invokes the questions initializer, `src-tauri/src/mind/portable_db.rs:883-892`):

| File | Change | Recompile/build scope |
|---|---|---|
| `specs/inquiry-queue.md` | Write/approve feature contract first, per spec-first rule. | No build effect. |
| `src-tauri/src/mind/questions.rs` | Add columns/defaults/migration, extend model/select/insert/update/map logic, add migration/CRUD tests. | Touching a Rust module recompiles its containing crate target; Cargo dependencies need not be rebuilt unless manifests/features change. `mind` declares this as a module (`src-tauri/src/mind/mod.rs:1-8`). |
| `src-tauri/src/lib.rs` | Add/update command implementation and handler registration (the current handlers/registry are centralized here, `:279-314, 1308-1336`). | Central Tauri crate root and command macro are rebuilt; downstream app target is relinked. |
| `src/types/index.ts` | Extend `QuestionItem` and add inquiry enums if represented as unions. | TypeScript/Vite build rechecks the frontend project. |
| `src/mind/services/dbService.ts` | Add typed wrapper for triage updates; existing list wrapper can return expanded DTO. | Frontend build processes this existing service module. |
| `src/shared/store/useAppStore.ts` | Store typed triage updates or selection and call service; preserve SQLite durability flow. | Frontend build processes the central store module. |
| `src/shared/layout/QuestionsBoardPage.tsx` | Preserve create/edit/delete behavior; show kind/status and retain defaults for existing rows. | Frontend build processes the page module. |
| `src/shared/layout/TaskFlowPage.tsx` | Add an inner Planning view switch; leave Task Flow selected by default. | Frontend build processes this existing page. |
| `src/shared/layout/InquiryTriagePanel.tsx` | New focused view for inquiry list/selection/score UI (recommended separation). | New TSX module is included in the existing frontend build. |
| `src/shared/layout/LeftSidebar.tsx` | Rename visible Questions Board label to Inquiry Queue while keeping its current callback/route-less mount. | Frontend build processes the sidebar module. |

**FACT — Build commands:** `pnpm build` runs `tsc && vite build`; `pnpm tauri:build` runs `tauri build` (`package.json:6-17`). There is no reason to introduce a dependency or alter the frontend build pipeline.

**INFERENCE —** Avoid editing `src/App.tsx` or `src/main.tsx`: Triage can live inside `TaskFlowPage`, which is already the mounted Planning component; no new route is needed (`src/App.tsx:2505-2521`; `src/main.tsx:50-61`). `lib.rs` is the unavoidable Rust hot spot because this codebase defines and registers these command handlers there (`src-tauri/src/lib.rs:279-314, 1308-1336`). Exact wall-clock rebuild time is **UNKNOWN**; the repository contains no benchmark of per-file build cost.

**FACT — Startup migration:** The questions initializer is invoked during workspace schema initialization (`src-tauri/src/mind/portable_db.rs:883-892`). Therefore adding columns requires a startup migration for existing DBs. A `_vora_meta` gate is feasible but not required for idempotence; PRAGMA checks can skip already-present columns (`src-tauri/src/mind/task_flow.rs:52-73`). If gated, initialize `_vora_meta` first because of the current call order (`src-tauri/src/mind/portable_db.rs:883-892`; `src-tauri/src/market/db.rs:113-118`).

## 7. EXISTING SPECS & CONVENTIONS

**FACT — `AGENTS.md` rules:** “No new feature, database table, or major refactoring should occur without a verified plan or markdown spec in `specs/`.” (`AGENTS.md:10`). It requires Rust and TypeScript serialized models to mirror and says IPC should pass through service/store (`AGENTS.md:19-23, 76-84`); visual components should not call `invoke` directly and the service layer owns `invoke` (`AGENTS.md:64-65`).

**FACT — Actual codebase precedent:** The Questions Board spec says SQL/persistence belong in `questions.rs`, commands register in `lib.rs`, and typed calls flow through `dbService` and Zustand; database errors must be visible in the UI (`specs/vora-questions-board.md:24-31`). Task Flow’s spec lists the current affected files and calls for module-owned SQL and idempotent workspace schema migration (`specs/vora-task-flow.md:15-38`).

**FACT — Mismatch to note:** `AGENTS.md` diagrams an older `src/store`, `src/services`, `resolver.rs`, and `db.rs` flow (`AGENTS.md:37-58, 76-84`). The inspected Questions/Task Flow implementation instead uses `src/shared/store/useAppStore.ts`, `src/mind/services/dbService.ts`, command handlers in `src-tauri/src/lib.rs`, and domain SQLite modules (`src/shared/store/useAppStore.ts:1-8`; `src/mind/services/dbService.ts:155-202`; `src-tauri/src/lib.rs:189-314`). Follow the implementation precedent while honoring the type-safety and spec-first rules.

**FACT — Naming:** Existing IPC command names are verb/entity snake_case (`get_questions`, `create_question`, `save_question`, `delete_question`; and `get_task_flow_items`, `reorder_task_flow_items`); no `q_` prefix convention is documented or used (`src-tauri/src/lib.rs:189-253, 279-314`). Service methods use camelCase in the existing shared `dbService` object (`src/mind/services/dbService.ts:164-202`). State is part of the central `useAppStore`, not a per-domain Zustand file (`src/shared/store/useAppStore.ts:1-8, 48-102`).

## 8. RECOMMENDED INTEGRATION PLAN (INFERENCE)

### Contract proposal

**INFERENCE —** Keep the existing table name and all existing CRUD semantics. Proposed additive fields:

| Column | Proposed SQLite type/default | Reason/qualification |
|---|---|---|
| `kind` | `TEXT NOT NULL DEFAULT 'question' CHECK(kind IN ('question','idea','insight'))` | Existing rows remain questions; new rows can represent all three kinds. |
| `status` | `TEXT NOT NULL DEFAULT 'parked' CHECK(status IN ('parked','triaged','promoted','hypothesis'))` | `hypothesis` reserves the named later lifecycle state; no hypothesis table/FK is introduced. |
| `priority` | `INTEGER NOT NULL DEFAULT 0` | Use as an explicit user-maintained ordering/rank, consistent with Task Flow’s integer positions. Define whether smaller or larger wins in the spec. |
| `impact` | `INTEGER NULL DEFAULT NULL` | Keeps legacy scores unknown rather than inventing a rating. Proposed scale 1–5 must be confirmed. |
| `feasibility` | `INTEGER NULL DEFAULT NULL` | Same as impact; proposed scale 1–5 must be confirmed. |
| `promoted_to_hypothesis_id` | `TEXT NULL DEFAULT NULL` | No FK can be declared until a hypothesis table exists; use this as a future soft reference only. |

**BLOCKING SEMANTIC GAP:** The requested score is impact × feasibility × urgency, but the requested columns do not include an urgency value and the repo has no urgency field on questions or planning items. Do not silently substitute `priority` for urgency: current Planning `position` is order, not an urgency score (`src-tauri/src/mind/task_flow.rs:90-114, 156-189`). Keep Phase 1 read-only until the urgency definition is settled.

### IPC and UI placement

**INFERENCE —** Phase 1 can reuse `get_questions` and extend its DTO; no new list command is required. Add one Phase 2 command:

```text
update_inquiry_triage(
  id: String,
  kind: InquiryKind,
  status: InquiryStatus,
  priority: i64,
  impact: Option<i64>,
  feasibility: Option<i64>
) -> Result<QuestionItem, String>
```

`InquiryKind` / `InquiryStatus` should be typed Rust enums serialized in the existing camelCase model contract, with matching TypeScript unions (`src-tauri/src/mind/questions.rs:4-12`; `src/types/index.ts`; `AGENTS.md:19-23`). Phase 3 can add `promote_inquiry_to_hypothesis(id: String) -> Result<QuestionItem, String>` as a stub-only promotion hook; until a hypothesis model exists, it should set only `status = 'promoted'` and leave `promoted_to_hypothesis_id` null. This does not create a hypothesis row.

Put `InquiryTriagePanel` under the inner view switch of `TaskFlowPage` for `taskType === "task"`; keep Task Flow as the default and leave the AI Tasks view isolated. The existing top-level mount already does this type selection (`src/App.tsx:2505-2507`).

### Exact proposed files

1. `specs/inquiry-queue.md` — write and review first.
2. `src-tauri/src/mind/questions.rs` — migration, model, queries, update method, focused Rust tests.
3. `src-tauri/src/lib.rs` — Phase 2/3 command wrappers and `generate_handler!` entries.
4. `src/types/index.ts` — expanded item and union types.
5. `src/mind/services/dbService.ts` — typed update/promote wrappers.
6. `src/shared/store/useAppStore.ts` — update action plus any view-local data state that must survive component rerenders.
7. `src/shared/layout/QuestionsBoardPage.tsx` — rename/extend visible board content while preserving create/edit/delete and defaults.
8. `src/shared/layout/TaskFlowPage.tsx` — internal view selector, defaulting to current Task Flow.
9. `src/shared/layout/InquiryTriagePanel.tsx` — new list/selection UI.
10. `src/shared/layout/LeftSidebar.tsx` — change visible label only; keep existing callback behavior.

No `main.tsx`, `App.tsx`, `portable_db.rs`, migration table, dependency, or new route is needed for this proposal. `portable_db.rs` already calls the initializer, and the current task/board switch can remain unchanged (`src-tauri/src/mind/portable_db.rs:883-892`; `src/App.tsx:2505-2521`).

### Three independently verifiable phases

| Phase | Files | Acceptance criteria | Tests |
|---|---|---|---|
| **1 — Schema + read-only Triage** | Spec; `questions.rs`; `types/index.ts`; `TaskFlowPage.tsx`; new `InquiryTriagePanel.tsx`; optionally `QuestionsBoardPage.tsx` and `LeftSidebar.tsx` for visible rename. No new IPC if expanded `get_questions` suffices. | Existing questions migrate with `kind=question`, `status=parked`, null scores/link; existing CRUD and Questions Board remain usable; Triage appears inside Planning and is read-only. | Rust tests for new and legacy schemas, defaults, DTO round-trip, idempotent second initialization; manual build and UI smoke checks. Existing backend test pattern is in `questions.rs:104-165`. |
| **2 — Interactive Triage** | `questions.rs`; `lib.rs`; `types/index.ts`; `dbService.ts`; `useAppStore.ts`; `InquiryTriagePanel.tsx`; possibly QuestionsBoard page. | Persist kind/status/priority/impact/feasibility through store → typed service → IPC → SQLite; selected inquiry is single-select; score order only after urgency semantics are specified. | Rust update/validation tests, invalid values and not-found tests; build/type-check; manual restart persistence check. Use Task Flow transactional reorder tests as a pattern (`task_flow.rs:156-189, 299-321`). |
| **3 — Promotion hook (stub only)** | `questions.rs`; `lib.rs`; `dbService.ts`; `useAppStore.ts`; `InquiryTriagePanel.tsx`; shared types. | Promote action records `promoted`; no hypothesis row or FK is created; link remains null until a later hypothesis feature. | Rust transition/invalid-status tests and one manual UI check that the promoted row leaves or updates the parked/triage list as specified. |

**Migration rollback note:** Mark the column migration irreversible once non-default values have been written; dropping columns would erase user-authored inquiry metadata.

## 9. OPEN QUESTIONS

1. What is the **urgency** input in `impact × feasibility × urgency`? Is it an additional durable field, an interpretation of `priority`, or a value derived elsewhere? The planned column list currently has no urgency field.
2. What scale and ordering should `impact` and `feasibility` use (for example 1–5), and what should `priority` mean (rank, score, or separate manual order)?
3. Should `hypothesis` be a valid `status` immediately, or should that value be introduced with the later hypothesis feature?
4. When a row becomes `promoted` before hypotheses exist, should it remain visible in Triage, move to a separate promoted filter, or become read-only?

## TL;DR for the Inquiry Queue implementation

- Keep `questions`; its schema, CRUD, model, UI, and Rust tests are already localized in `questions.rs`, `lib.rs`, `dbService`, Zustand, and `QuestionsBoardPage`.
- Put Triage inside `TaskFlowPage` behind an inner view switch; preserve Task Flow as the default and do not add a route or change root panel wiring.
- Migrate additively in `questions.rs` with checked PRAGMA/ALTER operations and propagated errors; legacy rows should default to `question` / `parked`.
- Reuse Task Flow’s integer ordering, reorder transaction, and numbered row affordance; no dedicated priority chip or Planning tab component currently exists.
- Resolve what “urgency” and `priority` mean before implementing the score; keep the migration reversible only until new metadata is written, then explicitly mark rollback as data-losing.
