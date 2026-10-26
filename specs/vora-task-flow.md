# VORA Task Flow

> **Status**: IMPLEMENTED
> **Author**: Copilot
> **Target Scope**: `layout | store | tauri | db | types`

## 1. Intent & System Goals

* **Objective**: Add a durable, workspace-wide task list named **Task Flow**, accessible in the left sidebar between Projects and Journal.
* **Target Workflow**: Users create tasks, mark them complete or incomplete, delete them, and drag task rows to change their saved priority order. The ordered rows form a simple timeline. In each task row, priority, drag handle, completion toggle, and title are arranged from right to left in that order. Only the task title is shown in the timeline. Clicking the title enables inline title editing; double-clicking it opens a dialog for editing the full title and description.
* **Non-Goals**: Calendar dates, times, reminders, project/board assignment, recurring tasks, and adding a new drag-and-drop dependency.

## 2. Affected Architectural Boundaries

* [x] `src/types/index.ts` (Task Flow data contract)
* [x] `src/shared/layout/LeftSidebar.tsx` (Navigation entry)
* [x] `src/shared/layout/TaskFlowPage.tsx` (Task Flow view)
* [x] `src/shared/store/useAppStore.ts` (Reactive task cache and actions)
* [x] `src/mind/services/dbService.ts` (Typed IPC methods)
* [x] `src-tauri/src/mind/task_flow.rs` (SQLite queries and model)
* [x] `src-tauri/src/mind/portable_db.rs` (Schema initialization)
* [x] `src-tauri/src/lib.rs` (IPC command handlers and registration)

## 3. Type Contracts & Data Schema Modifications

### 3.1. Frontend Interface

`TaskFlowItem` contains `id`, `title`, `description`, `completed`, `position`, `createdAt`, and `updatedAt`.

### 3.2. Backend Data Model

Rust serializes `TaskFlowItem` with camelCase field names to match the TypeScript interface.

### 3.3. Database Schema

Create `task_flow_items` with a text primary key, bounded title, description text, checked completion integer, integer position, and creation/update timestamps. Existing databases gain the description column through an idempotent migration. The task module is responsible for all Task Flow SQL; workspace schema initialization creates or migrates the table idempotently.

## 4. Implementation Plan

1. Define matching TypeScript and Rust task contracts.
2. Add database-backed list, create, title/description update, completion, reorder, and delete operations.
3. Expose typed Tauri IPC wrappers and Zustand actions.
4. Add a right-to-left Task Flow timeline with inline title editing and a title/description dialog, and place its active navigation item between Projects and Journal.
5. Validate with TypeScript and Rust checks plus focused backend tests.

## 5. Verification & Acceptance Criteria

* [x] `pnpm tsc --noEmit` exits successfully.
* [x] `cargo check --manifest-path src-tauri/Cargo.toml` exits successfully.
* [x] Focused Task Flow backend tests cover task creation, completion, deletion, and persisted ordering.
* [ ] Task rows show only their title and arrange priority, drag handle, completion toggle, and title from right to left.
* [ ] Clicking a title allows inline editing; double-clicking opens a dialog that saves both title and description.
* [ ] Manual verification: create tasks, drag to reorder, toggle completion, reopen Task Flow, and confirm task state/order remain intact.
