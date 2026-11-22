# Vora Mind Workflow Recovery and Idempotency

Workflow steps may declare `naturally_idempotent`, `idempotency_key`,
`non_idempotent`, or `unknown`. Persisted fields track execution policy,
idempotency key, and recovery state.

On restart, Running and AwaitingApproval workflows are inspected. Naturally
idempotent steps and valid idempotency-key steps may be retried. Unknown and
non-idempotent steps are moved to AwaitingApproval recovery and are never
automatically repeated. Cancellation and existing approval decisions are
preserved.

Tests cover interruption, idempotent retry, uncertain-step approval recovery,
and persisted workflow state.
