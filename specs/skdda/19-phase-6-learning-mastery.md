# Phase 6 — Learning, Competency, Mastery, and Independent Reproduction

## Authority and boundary

Implement the Learning domain on top of the published, immutable FrameworkVersion
foundation. Learning attainment is an independent learner assessment. It is not
scientific validation, research evidence, framework approval, or knowledge
transfer. Stop before Knowledge Transfer.

## Context and identity

- `LearningProject` is a learning workspace context with an explicit learner,
  exact published FrameworkVersion, and stable identity. It is not a
  ResearchProject and cannot own or mutate research findings.
- `Learner` is a stable local identity keyed by an opaque ID and display label.
- Framework ID and version ID are both pinned. A new framework publication
  never changes an existing LearningProject, objective, assessment, attempt,
  competency, or reproduction task.

## Objectives and prerequisites

- `LearningObjective` is a stable identity; every content revision creates an
  immutable `LearningObjectiveVersion`.
- An objective version includes its LearningProject, exact FrameworkVersion,
  observable outcome, criteria, level, and author/time.
- Prerequisites link exact objective versions within the same learning project
  and framework version. Reject self-links, duplicates, cross-context links,
  and cycles.
- Assessment definitions, competency criteria, and prerequisite edges pin
  objective versions, never a current/latest alias.

## Assessment lifecycle and attempts

- `Assessment` is stable identity. Every editable draft/revision is a new
  `AssessmentVersion`; versions pin one exact published FrameworkVersion and
  exact objective versions.
- `AssessmentVersion` lifecycle is draft → review → published → deprecated.
  Published content, questions, rubric reference, and objective links are
  immutable. Any edit creates a new version.
- Questions are ordered, version-owned records with prompt, response kind,
  maximum points, and exact objective version.
- Rubrics have stable identity and immutable `AssessmentRubricVersion`
  snapshots. Assessment versions pin one exact rubric version; rubric criteria
  have stable IDs, descriptions, and maximum points.
- An `AssessmentAttempt` pins learner, LearningProject, exact AssessmentVersion,
  exact FrameworkVersion, status, and timestamps. Responses may be edited only
  while in progress. Submission validates one response per required question,
  then seals the attempt and responses transactionally.
- Submitted responses cannot be changed or deleted. Every evaluation and
  correction is a new append-only evaluation revision with evaluator identity,
  exact rubric version, outcome, criterion scores, and rationale. Feedback is
  linked to that evaluation revision; it does not overwrite previous feedback.
- Delivery responses do not include evaluator-only scoring guidance. Errors
  remain explicit and an unsuccessful submission does not seal the attempt.

## Competency and explainable mastery

- `Competency` is a stable identity scoped to one LearningProject and exact
  FrameworkVersion. `CompetencyCriterion` is an immutable observable criterion
  linked to exact objective versions.
- `MasteryRule` is stable identity. `MasteryRuleVersion` is immutable and
  contains validated typed policy fields: minimum normalized score, minimum
  distinct evaluated attempts, whether all criteria must pass, whether
  independent reproduction is required, and the required reproduction outcome.
- `CompetencyRecord` is an immutable evaluation snapshot for one learner,
  competency, and exact MasteryRuleVersion. It stores the computed outcome and
  a human-readable explanation; child records pin the exact criteria,
  objective versions, attempt evaluations, and required reproduction
  evaluations used as inputs.
- Valid mastery inputs are only the exact assessment evaluations and
  independent reproduction evaluations declared for that competency. No
  research finding, framework scientific validation, content view, time spent,
  checklist, activity completion, or UI percentage can satisfy a rule.
- Re-evaluation creates a new CompetencyRecord. Historical outcomes and inputs
  remain immutable.
- `LearnerProgress` is a typed query projection computed from attempts,
  evaluations, competency records, and rule versions. It is not separately
  persisted mutable state. The projection lists qualifying and missing inputs,
  exact versions, and the reason for the current result.

## Independent reproduction

- `ReproductionTask` pins one exact FrameworkVersion and may reference a
  framework procedure membership. It defines task, conditions, expected result,
  exact rubric version, and learner/project context.
- Each `ReproductionSubmission` is immutable and records learner, timestamp,
  artifact URI/checksum and/or a structured result, and deviations. At least an
  artifact or a non-empty result is mandatory.
- `ReproductionEvaluation` is a separate append-only record pinning the
  submission and rubric version, evaluator, criterion scores, outcome,
  rationale, and evidence description. A submission cannot count toward
  mastery until it has a valid evaluation.
- Repeated submissions and opposing/inconclusive evaluations remain visible.

## Persistence, transactions, and IPC

- Add a checksummed, transactional Learning migration after the source,
  Research, Experiment, Analysis, and Model schemas. Preserve all earlier
  records without reinterpretation.
- Use typed FKs for learner/project/framework-version/objective-version,
  assessment/rubric, attempt, competency, mastery, and reproduction lineage.
  Use composite FKs or service validation to enforce same-context version
  consistency.
- Multi-row creation, attempt submission, evaluation, mastery evaluation, and
  reproduction evaluation are transactional.
- Rust domain operations own SQL and state-transition/scoring validation.
  Tauri commands delegate to Learning services. Frontend services own `invoke`;
  components do not call IPC directly.
- Append-only triggers protect submitted attempts/responses, evaluation
  revisions, published assessment/rubric/objective versions, competency
  records, submissions, and reproduction evaluations.

## Frontend workflows

The Learning workspace provides:

- Learning project/learner selection and immutable framework-version selection.
- Objective explorer with exact framework pin and prerequisite graph.
- Assessment authoring, review, publication, and version inspection.
- Learner-facing practice/delivery, attempt submission, and historical timeline.
- Evaluator workflow with criterion-level rubric scores, feedback, and explicit
  correction history.
- Competency criteria and versioned mastery rule management.
- Independent reproduction task, artifact/result submission, and evaluation.
- Explainable progress view listing exact attempts, evaluations, criteria,
  rule version, reproduction requirement, and missing requirements.

## Acceptance tests

1. Objectives, prerequisites, assessment versions, rubric versions,
   competencies, and reproduction tasks reject unpinned or cross-framework
   references.
2. Publishing a new FrameworkVersion leaves existing Learning records pinned
   to their original version.
3. Published assessment/rubric/objective versions and submitted attempts are
   immutable; corrections append history.
4. Attempts retain learner, assessment, and framework version identities.
5. Mastery rules are versioned; competency records reproduce from immutable
   input references and explain unmet requirements.
6. Content completion, arbitrary progress percentage, and research findings
   cannot create mastery.
7. Reproduction without an artifact/result or evaluator outcome cannot count.
8. Later attempts/evaluations do not remove historical records.
9. Restarting the application returns the same progress projection from
   durable rows.
10. Run focused Rust migration/domain tests, frontend type-check/build, and
    integration coverage for the project → objective → assessment → attempt →
    evaluation → mastery and reproduction workflows.
