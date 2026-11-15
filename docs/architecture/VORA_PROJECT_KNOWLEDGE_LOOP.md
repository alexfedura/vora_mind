# VORA Project Knowledge Loop

## Status and authority

This document records the Stage 3.1 architecture contract from the checked-in
domain code and schemas. It does not create new entities, change ownership, or
promise a complete end-to-end workbench. Rust repositories and their persisted
schemas are authoritative; TypeScript services and panels are adapters. A
frontend filter is not an authorization boundary.

The Stage 3.2 Project Knowledge Workbench implementation and its validation
evidence are documented in
[VORA_STAGE_3_2_IMPLEMENTATION.md](./VORA_STAGE_3_2_IMPLEMENTATION.md).

## Workflow map

```text
Workspace-shared Source
  └── immutable SourceVersion (capture metadata)
        └── immutable SourceLocation
              └── immutable KnowledgeExtraction
                    └── reviewed SourceClaim
                          ├── explicit typed claim relationships
                          ├── user-created project Research references
                          └── user-curated FrameworkVersion membership
                                ├── explicit Research project adoption
                                └── explicit Knowledge Transfer, when eligible

Source ── explicit ResearchProjectSource association ──> project context
        (use_context and notes; not a version citation)
```

The diagram is a set of persisted references, not an automatic pipeline. Each
step requires an explicit operation. A project may associate a shared Source
without citing a particular version; a Research citation instead pins an exact
SourceVersion and may pin its SourceLocation and SourceClaim.

## Entity contracts

| Entity | Owner, identifier, and version rule | Sharing / project semantics and authoritative operations | Permitted links and relationship | Never infer |
|---|---|---|---|---|
| **Source** | Source domain; `source_id`; stable catalog identity. | Workspace-shared. Source acquisition and `research_project_sources` operations in `source/acquisition` and `research/repository.rs`. | Direct parent of SourceVersions; may have one explicit ResearchProjectSource row per project. | A project association, citation, permission, or ownership from a Source's presence in the catalog. |
| **SourceVersion** | Source acquisition; `source_version_id`; immutable row. The schema blocks update/delete. | Workspace-shared capture metadata: capture kind, edition/URI, capture time, optional hash, external asset reference, and metadata. Created/listed by the Source acquisition repository and IPC service. | Composite reference to one Source; parent of version-bound SourceLocations and extractions. Research citations may pin its exact ID. | That a hash, URI, or external reference means the captured bytes are stored locally or will remain retrievable. |
| **SourceLocation** | Source acquisition; `source_location_id` (DTO `id`; persisted as `source_version_segments.id`); immutable row. | Workspace-shared and tied to one exact SourceVersion. Created/listed by the acquisition repository. | Composite Source + SourceVersion reference; a parent location must share that context. Extractions reference its exact segment ID. | That a location identifies a stable excerpt in every edition, or that its text is independently verified. |
| **KnowledgeExtraction** | Source acquisition; `extraction_id`; immutable content. Review records are appended; correction creates a replacement extraction with a supersession record. | Workspace-shared and tied to Source, SourceVersion, and SourceLocation. Created, reviewed, corrected, and listed by the acquisition repository. | Direct composite reference to one exact location and version; a SourceClaim can be created only from an accepted `source_claim` extraction. | That accepted/rejected extraction review establishes scientific truth. It is an extraction-quality/review state. |
| **SourceClaim** | Source acquisition; `source_claim_id`; created from one accepted extraction; no project owner. | Workspace-shared attribution to a SourceVersion. Created/listed by the acquisition repository. | Direct references to Source, exact SourceVersion, and extraction. Hypothesis evidence and Research citations may refer to it. | That a SourceClaim is a Research project's conclusion, a validated finding, or a truth score. |
| **Claim relationships** | Source acquisition; persisted relation rows in typed relation tables; the list DTO carries target kind/ID, relation kind, and rationale. | Workspace-shared, explicitly authored; no project ownership is implied. Created/listed by the acquisition repository. | Typed SourceClaim-to-SourceClaim, Concept, Principle, or Procedure relations. | Any edge, polarity, or semantic relation not explicitly recorded by a user operation. |
| **ResearchProjectSource** | Research; composite identity `(project_id, source_id)`; stores `use_context`, `notes`, and timestamps. | Project-owned context over a shared Source. `associate_project_source`, `list_project_sources`, and `update_project_source_context` in `research/repository.rs`; writes require an active project, duplicates are rejected, and updates constrain both IDs. | Direct link to Source only; it is separate from exact-version citations. | A SourceVersion citation, claim adoption, or association in another project. |
| **ResearchProject records** | Research; `research_projects.id`; gaps, phenomena, observation plans, patterns, ideas, questions, hypotheses, and evidence are separate typed records. Hypothesis definitions and observation plans have exact version IDs and lifecycle states. | Research repository owns project-scoped records. Legacy Questions/Hypotheses may remain unassigned; the explicit assignment operations require an active destination and rationale and record provenance. | Explicit links between a project's Questions/Hypotheses, reviewed research evidence, and exact artifacts. | Assignment from a legacy/workspace read, or Source–Project association as a side effect of assignment. |
| **Research citations and evidence** | Research; LiteratureReviewSource and typed evidence records are owned by their Research project/research record. | `create_reviewed_literature_review` / `add_literature_source` and hypothesis-evidence operations in the Research repository. Exact source-version citations validate SourceClaim context; hypothesis evidence records an explicit polarity and rationale. | Direct IDs to exact SourceVersion, optional SourceLocation and SourceClaim; claim provenance reaches the extraction/location. Links are user-mediated. | A project association from a citation, or scientific acceptance from citation/evidence presence alone. |
| **Research design and dataset references** | Research; project-owned design IDs and version IDs; `DatasetVersionReference` has its own ID and immutable version label/reference. | Research repository owns research questions, hypotheses, design records, observations, and Research dataset references. `research_get_workspace` is a workspace-wide read; project records carry project IDs. | A Research design record/version stores project, description, protocol, and lifecycle. The Experiment contract adds a typed specification and preregistration extension keyed by that version ID; a Research draft alone is not a typed runnable Experiment design. Research dataset references remain separate project-owned IDs. | That a Research draft is a typed/preregistered Experiment design, or that a Research dataset reference is an Experiment `DatasetVersion`. No implicit conversion exists. |
| **Framework / FrameworkVersion** | Source acquisition; `framework_id` and exact `framework_version_id` / `version_no`. A published version and its items are immutable. | Workspace-shared; Framework DTOs have no project owner. Framework membership is curated from typed knowledge items and exact SourceClaims. | `research_project_knowledge_adoptions` explicitly pins a published FrameworkVersion and project context. Knowledge Transfer can separately reference a FrameworkVersion when an origin/eligibility record permits it. | Project ownership, adoption, or transfer eligibility from publication status. Research adoption and Knowledge Transfer are separate operations and records. |
| **Experiment design and DatasetVersion** | Experiment extends the Research design-version ID with a typed specification/hash and preregistration; dataset IDs and versions are Experiment-owned. | Experiment repository and `experimentService`; writes validate destination-project ownership. Dataset versions are staged, then sealed; sealed contents are immutable and carry schema, counts, hash, and managed-asset metadata. | A preregistered typed design pins exact sealed Experiment DatasetVersions; runs preserve those pins. Research questions/hypotheses may be explicitly referenced by the design specification. | That a Research design draft is executable, or that a Research dataset reference is an Experiment `DatasetVersion`. A Source hash or external reference is not an Experiment asset. |
| **Model identity / ModelVersion** | Model; identity has an optional `origin_project_id` (null denotes a workspace-reusable identity in the UI); each ModelVersion has its own ID/version number and required `created_in_project_id`, content hash, and lineage. | Model repository and `modelService`; project reads and writes constrain version/evidence ownership. Existing Knowledge Transfer rules permit only explicit supported artifact transfer and preserve receiving-project lineage. | Typed links to project-compatible hypothesis, finding, evidence, experiment, and dataset artifacts are repository-validated. | Source ownership, model ownership, or cross-project reuse from a Source/Framework link. Stage 3.1 does not alter Model or transfer semantics. |
| **Knowledge Transfer** | Knowledge Transfer domain; distinct origin, eligibility, transfer, adaptation, and usage record IDs. | Explicit cross-project workflow. Supported artifacts are FindingVersion, ConceptVersion, PrincipleVersion, FrameworkVersion, and ModelVersion. Repository operations verify origin ownership where applicable, eligibility/purpose/recipient, and transfer state. | Exact artifact-version references; receiving project and any adaptation are separately recorded. | Automatic transfer from publication, project adoption, shared Source access, or a SourceClaim. |
| **Learning Project** | Learning domain; `learning_project_id`, learner, and exact FrameworkVersion pin; separate Learning entities and lifecycle. | Not a ResearchProject and not a Research lifecycle alias. Learning repository owns its project, objectives, assessments, and learning records. | A LearningProject uses an exact FrameworkVersion; any transfer/reuse is explicit where Knowledge Transfer is involved. | Research project ownership, Research archive policy, or a Research adoption from Learning-project existence. |

## Boundary rules

1. A Source is shared; project-specific Source context belongs only to the
   ResearchProjectSource association. Context and notes must always be read or
   updated using both project and Source identity.
2. SourceVersion, SourceLocation, extraction, and SourceClaim IDs preserve
   provenance. They do not prove local asset preservation or truth.
3. Research design and dataset-reference records stay distinct from Experiment
   typed specification/preregistration state and DatasetVersions. Experiment
   enriches a Research design-version ID; it does not make an arbitrary Research
   draft runnable. Exact IDs may cross a boundary only through an explicit,
   repository-validated reference.
4. A published FrameworkVersion is a versioned workspace artifact, not a
   project-owned object. Project adoption and Knowledge Transfer eligibility
   are different persisted decisions.
5. Research Projects and Learning Projects have separate tables, owners, and
   lifecycle rules. No universal archive policy is implied.
6. IDs such as `created_by`, reviewer IDs, and actor IDs are audit metadata in
   this trusted single-user application; they are not authenticated principals
   or project-membership authorization.

## Research UI asynchronous state

The Research workspace read is intentionally workspace-global: it returns
records for multiple Research projects, which the page filters by project.
Source catalog data is also global. Refreshes use request identity so only the
latest active request may publish workspace, loading, or error state. Hook
cleanup invalidates outstanding refreshes.

Project-local selections, project Source associations, form resets, and
mutation errors are guarded by their owning context and the page lifecycle.
The main Research project and independently selected Question project retain
separate identities. Assignment operations use a record-specific context keyed
to the exact destination project submitted; changing that destination makes
the pending operation stale without conflating it with either selected
Research project.

Project creation remains a workspace-level operation. Its existing follow-up
selection/reset is guarded by the captured origin context, and its error
publication now receives that same token's currentness predicate, preventing a
late failure from updating hook state after the page lifecycle is invalidated.

The project-operation guard captures a lifecycle generation in each token.
The page effect activates the guard and invalidates it during cleanup, which
rejects every outstanding mutation token. Invalidation is idempotent, and
activation after cleanup creates a new generation so React Strict Mode's
setup/cleanup/setup cycle does not leave the guard inactive. Project selection
changes continue to use their separate context generations and do not
invalidate the page lifecycle. Workspace-global actions use a lifecycle-only
token so their late errors are also suppressed after unmount.

Late success, failure, and subsequent form/selection updates are published
only while both the relevant lifecycle and project/destination context remain
current. An already-started backend write is not canceled or rolled back; a
completed write may therefore persist even when its UI result is discarded.

Synthetic state tests exercise the production project-operation and
latest-refresh guards, including mutation invalidation, reactivation,
destination changes, and unchanged-destination failures. They do not mount
React components or verify browser-rendered form behavior or the effect
lifecycle itself.

These are presentation-state safeguards only. They do not cancel IPC calls,
roll back completed writes, or provide authenticated authorization or project
membership enforcement. The Research workspace transport remains global.

## Stage 3.2 prerequisites

- Write a feature specification before adding a Project Knowledge Workbench or
  any new persistence relationship.
- Keep the distinction between `source_segment_id` and the public
  `SourceLocationDto.id` explicit in any new service/type contract; the current
  schema uses the same persisted location row and validates the composite
  Source/SourceVersion context.
- Specify how a user selects and reviews a SourceVersion, location, extraction,
  claim, and project citation before wiring those operations into a workflow.
- Preserve project-scoped association context and exact version references;
  include two-project isolation tests using synthetic fixtures.
- Decide separately whether local asset preservation is a product requirement.
  The current metadata fields do not guarantee it.
- Do not add generic graph edges, automatic claim synthesis, truth scores,
  ownership conversion, or implicit Knowledge Transfer as a shortcut.
