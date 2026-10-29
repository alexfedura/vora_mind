# Lab Hypothesis Promotion and Verdicts

## Scope

The Lab workflow promotes a selected, triaged Inquiry into a Hypothesis and
supports recording test evidence and a final verdict. A Hypothesis can be
reviewed from the Lab tab in Planning.

## Persistence

Create a `hypotheses` table with an ID, the source `inquiry_id`, H1/H0
statements, independent and dependent variables, measurement metric, success
threshold, status, and creation/update timestamps. Status is constrained to
`draft`, `preregistered`, `testing`, `supported`, or `rejected`. Add nullable
`evidence_notes`, `verdict`, and `evaluated_at` columns idempotently for
existing databases. `verdict` is null until recorded, then must be one of
`supported`, `rejected`, or `inconclusive`. Index hypotheses by inquiry and by
status/update time.

Initialize this schema immediately after the existing questions schema when
initializing a workspace database.

## Promotion

The `promote_to_hypothesis` IPC command runs a single SQLite transaction. It
requires the inquiry to exist, have `triaged` status, and be selected for
promotion. It inserts a draft hypothesis, marks the inquiry `promoted`, stores
the hypothesis ID on the inquiry, clears its promotion selection, and updates
its timestamp. Any failed validation or write rolls back all changes.

`get_hypotheses` returns all hypotheses in descending creation-time order.
Both commands use the existing workspace database state and expose
camel-cased serialized model fields.

## Test and Verdict

The Lab view lists hypotheses and opens a selected hypothesis detail view with
its H1, H0, variables, metric, and success threshold. A hypothesis without a
verdict accepts evidence notes and one of three verdicts: supported, rejected,
or inconclusive. The `record_verdict` IPC command validates the verdict,
persists the notes and verdict, and records the evaluation timestamp. A
recorded verdict and its evidence are shown in the detail view.

## Verification

Unit tests cover successful promotion and inquiry state changes, rejection of
unselected and missing inquiries, and descending hypothesis retrieval order.
They also cover verdict recording, validation, and schema migration
idempotency. The focused command is:

`cargo test --manifest-path src-tauri/Cargo.toml --lib mind::hypothesis -- --nocapture`
