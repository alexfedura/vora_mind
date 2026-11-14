# Project Ownership Spec

## Policy

The scientific project boundary is enforced in backend repositories, not only in the UI.

### Ownership model

- Workspace: local container for the application and general resources
- ResearchProject: independent scientific research context
- ResearchDomain: conceptual domain membership for a project
- Shared Workspace Resources: objects not tied to a single project
- Project-owned scientific records: all project-scoped research artifacts
- Cross-project knowledge transfer: explicit and auditable, never implicit or silent

## Required backend enforcement

- project-scoped reads must validate the requested project
- project-scoped writes must validate the owning project on every relevant entity record
- archived projects must reject ordinary writes, experiment execution, and model change operations
- mismatched project/entity IDs must return a backend error, not a filtered UI result

This is an operational policy for a trusted single-user local application, not
authenticated authorization or multi-user membership enforcement. Actor IDs are
audit metadata and do not establish an authenticated principal.

## Ownership examples

Project-owned records include:

- research observations
- patterns and ideas
- gaps and questions
- hypotheses and experiment designs
- experiment runs and datasets
- analysis runs, evidence, and findings
- model versions, validations, and replications

## Legacy and unassigned records

Legacy rows without explicit ownership must remain readable and auditable. The application must allow assignment to a project without overwriting the original provenance.

## Cross-project references

Cross-project access is allowed only through an explicit association or transfer record that records:

- origin project
- receiving project
- source entity
- source version
- transfer reason
- recorded domain approval/rationale (not authenticated principal authorization)
- timestamp
- adaptation status

The transfer must not bypass project access controls.
