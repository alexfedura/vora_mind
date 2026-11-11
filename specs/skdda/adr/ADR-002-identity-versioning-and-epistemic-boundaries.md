# ADR-002: Opaque Identity, Immutable Versions, and Epistemic Boundaries

- **Status:** Accepted for Phase 0 architecture baseline; product/scientific-owner ratification pending.
- **Date:** 2026-11-11
- **Decision owners:** Architecture proposal; named domain owners not recorded.
- **Scope:** Entity IDs, versions, scientific meaning, ownership, and visual boundaries.

## Context

The current application has UUID-like IDs, semantic board/library version fields, append-only source operation records, mutable source metadata, free-text hypothesis fields, and visualization edges. These existing mechanisms are not equivalent to immutable SourceVersion, ExperimentRun, DatasetVersion, ResearchFinding, ModelVersion, or a typed scientific relationship.

## Decision

1. Generate new identity IDs as opaque lowercase UUIDv7 values. The repository already has the UUID v7 dependency. IDs do not encode title, path, status, domain, owner, or creation order. Preserve existing legacy IDs unchanged.
2. Each versioned family has a stable entity ID plus a distinct version ID and monotonic display sequence. Version ID—not a label such as `1.2.0` or “latest”—is the durable pin.
3. Versions are immutable snapshots from creation. Draft edits create a replacement version/revision; preregistration, publication, submission or release seals the exact version for downstream use.
4. Observations, submitted attempts, run inputs/results, evidence, findings, evaluations, validations, replications, relation assertions and provenance events retain prior meaning. Corrections append a superseding record/event; physical deletion is restrictive.
5. SourceVersion is an immutable edition/capture of Source, not a row synthesized from `source_revisions`. SourceClaim is an attributed source proposition; ResearchFinding is a scoped research synthesis. Neither substitutes for the other.
6. Concept, Principle and Idea are distinct: semantic abstraction; scoped rule/generalization; unvalidated proposal. Pattern is exploratory regularity; Observation is measured/recorded event; Hypothesis is falsifiable proposition; Evidence is attributable artifact/assessment; ResearchFinding is synthesis.
7. HypothesisAssessment, ModelEvaluation, Validation, Replication and CompetencyRecord are separate records and outcomes. No “supported/validated/replicated/mastered/established” universal status.
8. A ResearchProject owns project-specific research records. Reusable objects have explicit workspace/project ownership and retain origin. Unknown/unassigned ownership is explicit and is never inferred from titles or directories.
9. Canvas remains visualization. Moving/deleting a node or edge cannot change the scientific entity, canonical relation, lifecycle, evidence, or version; a canvas edge is a projection referencing a relation ID.

## Alternatives considered

- Treat each edit as an UPDATE and rely on timestamps: rejected because previous protocol, data and scientific meaning would be unrecoverable.
- Use semantic version strings or paths as IDs: rejected because display labels and paths can change/collide.
- Convert existing names/JSON/Trace nodes to scientific entities based on label: rejected because semantic type, version, provenance, ownership and validity cannot be established from text alone.
- Store scientific truth as canvas snapshots/edges: rejected because visual deletion/fork/layout would corrupt domain meaning.

## Consequences

More records and joins are required, but exact citations, reproducible inputs, lineage and historical results remain addressable. Local user-generated legacy IDs remain valid through adapters; no mass re-key is authorized. Stakeholders still need to decide policy details such as access restrictions, Theory semantics, and learner identity beyond a local workspace profile.
