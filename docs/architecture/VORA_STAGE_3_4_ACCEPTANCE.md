# Stage 3.4 — Synthetic End-to-End Acceptance and Integration Validation

## Status

Stage 3.4 validates the combined Project Knowledge lifecycle with isolated synthetic fixtures and the existing production model and guard contracts. It does not create new persistence, database records, schema changes, or release operations. The validation is intentionally read-only in the repository and uses synthetic IDs to confirm project isolation and exact lineage guarantees.

## Scope and acceptance criteria

The validation covers the supported lifecycle path:

```text
Shared Source
  -> exact SourceVersion
  -> exact SourceLocation / Extraction context
  -> reviewed SourceClaim and explicit relationships
  -> project-scoped Research usage
  -> FrameworkVersion adoption
  -> Experiment design and dataset pin
  -> Analysis evidence and finding
  -> ModelVersion and model evidence
  -> explicit Knowledge Transfer
  -> distinct recipient project identity and exact lineage
```

The accepted behavior is:

- project-scoped records remain filtered to the selected project;
- cross-project references remain explicit and visible as outside-project boundaries;
- exact SourceVersion, SourceLocation, SourceClaim, FrameworkVersion, DatasetVersion, ModelVersion, and transfer artifact IDs are preserved;
- unavailable exact references remain explicitly unavailable and are never silently replaced by another version;
- SourceClaims remain attributed evidence, not scientific conclusions;
- Knowledge Transfer keeps origin and recipient adaptations distinct;
- stale async publication remains prevented by the existing guard contracts;
- no backend write cancellation or rollback is claimed unless it is implemented and proven.

## Synthetic fixture design

The validation uses the existing model-level test architecture in [projectLifecycleModel.test.mjs](../tests/projectLifecycleModel.test.mjs) and the production guards in [researchAsyncGuards.test.mjs](../tests/researchAsyncGuards.test.mjs). Synthetic data is built entirely in memory with test-only IDs and no access to the workspace database.

The Stage 3.4 test suite validates:

- Project A and Project B separation.
- Exact dataset and model-version pins for the selected project.
- Source-version and SourceLocation provenance retention.
- FrameworkVersion adoption without ownership transfer.
- Knowledge Transfer origin adaptation distinctness across project boundaries.
- Missing exact ModelVersion non-substitution.
- Review of unsupported or unavailable operations as explicit limitations.

## Validation outcomes

### Project isolation and lifecycle integrity

- Project A and Project B stay separated in the synthetic fixture graph.
- The selected project does not absorb unrelated project records.
- Cross-project transfer origin artifacts remain marked as outside-project when referenced from another project.
- The recipient adaptation is preserved as a distinct artifact ID rather than replacing the original origin version.

### Exact version and provenance integrity

- The exact SourceVersion ID remains the target of citations and evidence links.
- The exact SourceLocation ID remains explicit in the source provenance chain.
- The exact SourceClaim ID remains visible as attributed evidence.
- An unavailable requested ModelVersion remains unavailable and is not silently replaced by another local version.
- FrameworkVersion adoption remains a shared workspace version reference and does not imply ownership transfer.

### Async and lifecycle integrity

The existing guard tests continue to protect stale UI publication, including:

- A → B → late A invalidation;
- late success/failure publication after the relevant project or lifecycle becomes stale;
- stale loading/finally updates;
- lifecycle cleanup and Strict Mode reactivation behavior.

These are UI stale-response protections only. They do not cancel backend IPC requests or roll back already-completed writes.

## UI verification level

This Stage 3.4 validation runs service/model-level tests and the existing guard contracts. The repository does not currently mount React components for the workflow in this audit. Browser-mounted lifecycle behavior, scroll/event integration, and full UI automation remain unverified here. That limitation is documented and accepted for this stage.

## Commands and results

The repository's actual validation commands were run in the current working tree and reported the following:

- `node --test tests/researchAsyncGuards.test.mjs tests/projectLifecycleModel.test.mjs tests/projectKnowledgeStage3_4.test.mjs` — passed.
- `pnpm tsc --noEmit` — passed.
- `pnpm build` — passed.
- `git -c core.whitespace=cr-at-eol diff --check` — passed.

## Blocking findings and known limitations

No blocking functional defect was found in the supported Project Knowledge lifecycle contracts covered by the synthetic validation. The remaining limitations are intentionally scoped:

- no mounted React lifecycle verification;
- no backend cancellation or rollback claims;
- no multi-user authorization or project-membership enforcement claim;
- no production database access or migration authority.

## Final decision

`ACCEPT WITH DOCUMENTED LIMITATIONS — STAGE 3.4 VALIDATED`

This acceptance applies only to the validated project lifecycle slice and remains subject to the documented limitations above.
