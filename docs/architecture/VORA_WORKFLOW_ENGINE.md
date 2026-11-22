# VORA MIND Workflow Engine

## States

- Draft
- Pending
- Running
- AwaitingApproval
- Succeeded
- Failed
- Cancelled

## Persistence

The engine stores workflows and ordered steps in SQLite. Step input is stored
as JSON; execution results, errors, retry counts, timestamps, and approval
states are persisted.

## Execution

Every step invokes the capability runtime. It does not reimplement capability
logic. Workflow execution creates a controlled execution context with workflow
ID and correlation ID.

## Recovery

The engine restarts at the persisted `current_step`. Cancellation is persisted.
Approval-required steps remain blocked until an approval decision is recorded.
Idempotency and explicit per-step recovery decisions remain required for
non-idempotent operations in later phases.
