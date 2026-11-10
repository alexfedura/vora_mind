# Vora Mind Research Pipeline

## Goal

Connect the existing Lab hypothesis workflow to the Vora Mind canvas. Users can
create a hypothesis from a source card or media node, see it as a dedicated
canvas node, connect it to source and matrix nodes, and open its Lab detail.

## Persistence

- Add nullable `source_card_id` and `source_highlight_id` columns to
  `hypotheses` through idempotent migrations in `hypothesis.rs`.
- Keep hypothesis-to-source lineage on the hypothesis record. The canvas node
  uses the existing Board node/snapshot persistence and regular canvas edges;
  no separate pipeline graph or matrix table is introduced.
- Canvas-origin hypotheses retain the existing required inquiry relationship:
  creation atomically records a promoted inquiry and its hypothesis.

## Interaction

- Right-clicking a card or media node offers “Create Hypothesis from this
  Source”. The form captures H1, H0, variables, metric, and success threshold,
  and includes the source card/node and available highlight lineage.
- Successful creation places a `voraHypothesis` node beside its source with
  the hypothesis ID, statement, and outcome/status presentation, and adds a
  labeled source-to-hypothesis canvas edge.
- Hypothesis nodes have source/target handles on each side so they can be
  connected to cards and decision matrices using standard canvas edges.
- Double-clicking a hypothesis node opens the Lab tab at that hypothesis.
- Lab detail offers a return-to-source action that returns to the Board and
  focuses/highlights the originating node when it is present.

## Verification

- Test the lineage migration against a pre-existing hypotheses schema and
  verify repeated initialization is safe.
- Run `cargo check` and `pnpm tsc --noEmit`.
- Manually verify source context-menu creation, persisted node insertion,
  hypothesis-to-source and hypothesis-to-matrix connections, Lab navigation,
  and return-to-source focus.
