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
Add nullable `promoted_to_library_id` and `parent_hypothesis_id` columns
idempotently to track Library graduation and hypothesis iterations.
Create a `hypothesis_definitions` junction table that records each linked
Library group and exact version ID, definition name, and version string for a
hypothesis. Fetch these links with each hypothesis and store them in the same
transaction as hypothesis promotion or child creation. Older callers may omit
the list, which is treated as empty.

Initialize this schema immediately after the existing questions schema when
initializing a workspace database.

## Promotion

The `promote_to_hypothesis` IPC command runs a single SQLite transaction. It
requires the inquiry to exist, have `triaged` status, and be selected for
promotion. It inserts a draft hypothesis, marks the inquiry `promoted`, stores
the hypothesis ID on the inquiry, clears its promotion selection, and updates
its timestamp. Optional linked Library definitions preserve the exact selected
group and version; any failed validation or write rolls back all changes.
Available selections include Library definitions and their available versions.

`get_hypotheses` returns all hypotheses in descending creation-time order.
Both commands use the existing workspace database state and expose
camel-cased serialized model fields.

## Test and Verdict

The Lab view lists hypotheses and opens a selected hypothesis detail view with
its H1, H0, variables, metric, and success threshold. A hypothesis without a
verdict accepts evidence notes and one of three verdicts: supported, rejected,
or inconclusive. The `record_verdict` IPC command validates the verdict,
persists the notes and verdict, and records the evaluation timestamp. A
recorded verdict and its evidence are shown in the detail view. Users can edit
a recorded verdict after confirming; saving the edit replaces the previous
verdict and evidence notes.

## Post-verdict actions

A supported hypothesis can be graduated to the Library once. Graduation saves
its H1/H0 statements, variables, metric, success threshold, and evidence notes
as a Library item-group version and records that group's ID on the hypothesis
in the same transaction. An optional existing Library group receives an
appended version; otherwise graduation creates a new group.

A rejected hypothesis can create a draft child hypothesis. The child inherits
the source inquiry, stores the parent hypothesis ID, and starts with its
parent's H1/H0, variables, metric, and success threshold so those values can be
iterated. It also inherits the parent's exact linked definition versions so
the changed hypothesis variable is not confounded by a changed definition.
Hypothesis detail shows linked definitions with their recorded version.

## Verification

Unit tests cover successful promotion and inquiry state changes, rejection of
unselected and missing inquiries, and descending hypothesis retrieval order.
They also cover verdict recording, overwriting an existing verdict and its
evidence, validation, and schema migration idempotency. The focused command is:

`cargo test --manifest-path src-tauri/Cargo.toml --lib mind::hypothesis -- --nocapture`
