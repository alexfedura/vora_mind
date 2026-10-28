# Lab Hypothesis Promotion

## Scope

Add a backend-only Hypothesis domain to the Lab workflow. A selected, triaged
Inquiry may be promoted exactly once into a Hypothesis. No frontend changes or
new dependencies are included.

## Persistence

Create a `hypotheses` table with an ID, the source `inquiry_id`, H1/H0
statements, independent and dependent variables, measurement metric, success
threshold, status, and creation/update timestamps. Status is constrained to
`draft`, `preregistered`, `testing`, `supported`, or `rejected`. Index
hypotheses by inquiry and by status/update time.

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

## Verification

Unit tests cover successful promotion and inquiry state changes, rejection of
unselected and missing inquiries, and descending hypothesis retrieval order.
The focused command is:

`cargo test --manifest-path src-tauri/Cargo.toml --lib mind::hypothesis -- --nocapture`
