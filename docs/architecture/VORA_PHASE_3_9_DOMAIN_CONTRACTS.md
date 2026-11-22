# VORA Phase 3.9 Domain Contracts

## Contract requirements

Every migrated domain must define:

- typed domain entities and stable IDs
- repository responsibilities
- application-service responsibilities
- capability IDs and schemas
- validation
- permissions
- project scope
- approval requirements
- audit events
- provenance
- persistence and migrations
- test evidence

## Core relationship contracts

### Project ownership

Every project-owned entity must carry an explicit project ID or prove that it is intentionally workspace-global. Cross-project reads and writes must fail before side effects.

### Sources and revisions

A source and source version must remain distinct. Versions must be immutable or revisioned. Project usage must be an explicit association, not a copied or flattened source description.

### Research graph

Questions, hypotheses, observations, evidence, claims, experiments, results, and findings must retain meaningful entity boundaries and typed relationships.

### Provenance

Evidence must identify source, source version, acquisition/derivation context, actor, project, timestamp, and relevant lineage. UI display must not become the provenance authority.

### Vora Trace

Trace decisions and experiment nodes must use canonical domain entities or explicit adapters. A file-backed graph must not become a second authoritative database.

### AI execution

AI execution must use the capability registry, not provider-specific direct calls. Approval must bind to exact capability ID and canonical input fingerprint.
