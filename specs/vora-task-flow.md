# VORA Task Flow

> **Status**: IMPLEMENTED
> **Author**: Copilot
> **Target Scope**: `layout | store | tauri | db | types`

## 1. Intent & System Goals

* **Objective**: Provide two durable, workspace-wide task lists—**Task Flow** and **AI Tasks**—in the left sidebar. AI Tasks uses the same task workflow but keeps its records separate from regular tasks.
* **Target Workflow**: Users create tasks, mark them complete or incomplete, delete them, and drag task rows to change their saved priority order. The ordered rows form a simple timeline.
* **Task Editing**: Arrange priority, drag handle, completion toggle, and title from right to left. Show only the title in the timeline. Clicking a title enables inline editing; double-clicking opens a dialog for editing its title and description.
* **Task Separation**: Every task has a type (`task` or `ai`). Existing records remain regular tasks. List loading and ordering operate within one type so the two views never mix tasks or priorities.
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

`TaskFlowItem` contains `id`, `title`, `description`, `taskType`, `completed`, `position`, `createdAt`, and `updatedAt`. `TaskFlowType` is `task | ai`.

### 3.2. Backend Data Model

Rust serializes `TaskFlowItem` with camelCase field names to match the TypeScript interface.

### 3.3. Database Schema

Create `task_flow_items` with a text primary key, bounded title, description text, task type, checked completion integer, integer position, and creation/update timestamps. Existing databases gain the description column and a `task_type` defaulting to `task` through idempotent migrations. The task module is responsible for all Task Flow SQL; workspace schema initialization creates or migrates the table idempotently.

## 4. Implementation Plan

1. Define matching TypeScript and Rust task contracts.
2. Add database-backed list, create, title/description update, completion, reorder, and delete operations scoped to task type.
3. Expose typed Tauri IPC wrappers and Zustand actions.
4. Add a right-to-left Task Flow timeline with inline title editing and a title/description dialog. Add a separate AI Tasks navigation entry using the same interface and isolated data.
5. Validate with TypeScript and Rust checks plus focused backend tests.

## 5. Verification & Acceptance Criteria

* [x] `pnpm tsc --noEmit` exits successfully.
* [x] `cargo check --manifest-path src-tauri/Cargo.toml` exits successfully.
* [x] Focused Task Flow backend tests cover task creation, completion, deletion, and persisted ordering.
* [ ] Task rows show only their title and arrange priority, drag handle, completion toggle, and title from right to left.
* [ ] Clicking a title allows inline editing; double-clicking opens a dialog that saves both title and description.
* [ ] AI Tasks and Task Flow show and reorder only their own task records.
* [ ] Manual verification: create tasks, drag to reorder, toggle completion, reopen Task Flow, and confirm task state/order remain intact.
