# Inquiry Queue — Sub-phase A

## Scope

Extend the existing `questions` table into a durable Inquiry Queue. This phase is backend-only: additive schema migration, Rust lifecycle operations, Tauri IPC commands, and focused Rust tests. Existing `create_question` and `save_question` implementations remain unchanged. No frontend, route, new table, dependency, build-pipeline, or unrelated schema changes are included.

## Schema

Keep the `questions` table and its current columns/index. Add these columns through one `PRAGMA table_info(questions)` scan and static, guarded `ALTER TABLE ... ADD COLUMN` statements:

| Column | Definition |
|---|---|
| `status` | `TEXT NOT NULL DEFAULT 'parked' CHECK(status IN ('parked','triaged','archived','promoted'))` |
| `impact` | `INTEGER NULL` |
| `feasibility` | `INTEGER NULL` |
| `urgency` | `INTEGER NULL` |
| `priority_score` | `INTEGER NULL` |
| `triage_notes` | `TEXT NULL` |
| `triaged_at` | `INTEGER NULL` |
| `is_selected_for_promotion` | `INTEGER NOT NULL DEFAULT 0 CHECK(is_selected_for_promotion IN (0,1))` |
| `promoted_to_hypothesis_id` | `TEXT NULL` |

Add `idx_questions_status_score` on `(status, priority_score DESC, updated_at DESC)`. Use no `_vora_meta` marker. Every SQL operation must propagate errors.

## Rust contract and invariants

Extend `QuestionItem` with `status: String`, nullable `i64` impact/feasibility/urgency/priority score/triaged timestamp, nullable notes and future hypothesis ID, and a boolean selection field. Keep Serde camelCase.

Enforce in Rust:

- Scores are each null outside `triaged`; when triaged, each is 1–5, score is their product, and `triaged_at` is set.
- `is_selected_for_promotion` implies `status == "triaged"`; only one row can be selected at once.
- Allowed operations: parked→triaged, triaged→triaged, triaged→parked, parked/triaged→archived, and selecting/unselecting a triaged row.
- Archived is terminal. Commands in this phase never set `promoted`; it is reserved in the schema only.
- Untriage and archive clear all triage values and selection.
- Selection replacement is transactional.

## IPC

Add and register `triage_question`, `untriage_question`, `archive_question`, `select_for_promotion`, and `unselect_for_promotion`. Commands lock the existing DB state and map failures to strings, following existing question command conventions.

## Verification

Rust unit tests cover fresh schema, existing databases and defaults, repeated initialization, triage values and validation, transition restrictions, untriage/archive clearing, selection requirements and atomic replacement, and the absence of promotion transitions.

## Rollback

Dropping the added columns after users have written triage metadata would destroy that data; rollback is irreversible after metadata is written.
