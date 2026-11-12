# Vora Target Architecture

This file records the target architecture for the scientific and project-aware Vora Mind system. It is intentionally grounded in the repository’s current implementation boundaries rather than an imagined rewrite.

## 1. Architectural intent

The target system keeps the current Tauri + React + Rust + SQLite architecture but adds an explicit scientific domain layer with project ownership, immutable lineage, and auditable evidence.

## 2. Core layers

### 2.1 Workspace layer

- Stores local application state and user-level resources
- Remains workspace-global for boards, library groups, Trace navigation, market state, worker queue, and other non-project-scoped utilities
- Requires explicit boundaries for any data that is project-owned

### 2.2 Research project layer

- A `ResearchProject` is the primary scientific context
- Each project has lifecycle states including `proposed`, `active`, `paused`, `completed`, and `archived`
- Archived projects remain readable but reject ordinary writes and experiment execution

### 2.3 Scientific entity layer

The scientific entity graph should be represented by typed entities and strict foreign-key boundaries:

- research observations
- research patterns
- ideas and research gaps
- questions and hypotheses
- experiment designs and runs
- datasets and dataset versions
- analysis runs and evidence
- findings and evaluations
- model identities, versions, validations, and replications
- explicit knowledge transfer records

These should not be merged with board snapshots or trace graphs unless the architecture explicitly defines an integration layer.

### 2.4 Shared source and provenance layer

- Shared sources remain globally cataloged
- Source versions are immutable and content-hash protected where supported
- Each project usage is an explicit association
- Citation and location references remain exact and auditable

## 3. Hard requirements

- No project-scoped read or write may accept an entity ID from another project
- Global query results must not leak private project details
- Project ownership must be enforced in backend repositories, not just the UI
- Legacy or unassigned records must be retained with provenance and an assignment workflow
- AI-generated suggestions must remain distinguishable from human-approved scientific decisions

## 4. Known implementation gap

The current repository already contains the necessary building blocks for source, research, experiment, analysis, model, and transfer domains, but not yet a fully consistent, verified cross-domain implementation with universal ownership enforcement.

The architecture target therefore is not a rewrite: it is a disciplined integration and enforcement effort that preserves valid legacy data while closing the ownership, provenance, and workflow gaps identified in the current-state audit.
