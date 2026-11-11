CREATE TABLE learning_schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at_us INTEGER NOT NULL
);

CREATE TABLE learning_learners (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);

CREATE TABLE learning_projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    learner_id TEXT NOT NULL REFERENCES learning_learners(id) ON DELETE RESTRICT,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, framework_id, framework_version_id),
    FOREIGN KEY (framework_version_id, framework_id)
        REFERENCES knowledge_framework_versions(id, framework_id) ON DELETE RESTRICT
);

CREATE TABLE learning_objectives (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL
        REFERENCES learning_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, learning_project_id)
);

CREATE TABLE learning_objective_versions (
    id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL REFERENCES learning_objectives(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    parent_version_id TEXT,
    observable_outcome TEXT NOT NULL CHECK (length(trim(observable_outcome)) > 0),
    criteria_text TEXT NOT NULL CHECK (length(trim(criteria_text)) > 0),
    level TEXT NOT NULL CHECK (length(trim(level)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (objective_id, version_no),
    UNIQUE (id, objective_id),
    UNIQUE (id, learning_project_id, framework_version_id),
    UNIQUE (id, learning_project_id, framework_id, framework_version_id),
    FOREIGN KEY (objective_id, learning_project_id)
        REFERENCES learning_objectives(id, learning_project_id) ON DELETE RESTRICT,
    FOREIGN KEY (learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_projects(id, framework_id, framework_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (parent_version_id, objective_id)
        REFERENCES learning_objective_versions(id, objective_id) ON DELETE RESTRICT
);

CREATE TABLE learning_objective_prerequisites (
    objective_version_id TEXT NOT NULL,
    prerequisite_version_id TEXT NOT NULL,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    PRIMARY KEY (objective_version_id, prerequisite_version_id),
    CHECK (objective_version_id <> prerequisite_version_id),
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT,
    FOREIGN KEY (prerequisite_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT
);

CREATE TABLE assessment_rubrics (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, learning_project_id, framework_version_id),
    FOREIGN KEY (learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_projects(id, framework_id, framework_version_id) ON DELETE RESTRICT
);

CREATE TABLE assessment_rubric_versions (
    id TEXT PRIMARY KEY,
    rubric_id TEXT NOT NULL REFERENCES assessment_rubrics(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    learning_project_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    pass_threshold REAL NOT NULL CHECK (pass_threshold >= 0 AND pass_threshold <= 1),
    status TEXT NOT NULL CHECK (status IN ('draft', 'review', 'published', 'deprecated')),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    published_at_us INTEGER,
    UNIQUE (rubric_id, version_no),
    UNIQUE (id, learning_project_id, framework_version_id),
    FOREIGN KEY (rubric_id, learning_project_id, framework_version_id)
        REFERENCES assessment_rubrics(id, learning_project_id, framework_version_id)
        ON DELETE RESTRICT,
    CHECK ((status IN ('published', 'deprecated')) = (published_at_us IS NOT NULL))
);

CREATE TABLE assessment_rubric_criteria (
    id TEXT PRIMARY KEY,
    rubric_version_id TEXT NOT NULL
        REFERENCES assessment_rubric_versions(id) ON DELETE RESTRICT,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    objective_version_id TEXT NOT NULL,
    criterion_key TEXT NOT NULL CHECK (length(trim(criterion_key)) > 0),
    description TEXT NOT NULL CHECK (length(trim(description)) > 0),
    max_points REAL NOT NULL CHECK (max_points > 0),
    UNIQUE (rubric_version_id, criterion_key),
    UNIQUE (id, rubric_version_id),
    FOREIGN KEY (rubric_version_id, learning_project_id, framework_version_id)
        REFERENCES assessment_rubric_versions(id, learning_project_id, framework_version_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT
);

CREATE TABLE assessments (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL
        REFERENCES learning_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, learning_project_id)
);

CREATE TABLE assessment_versions (
    id TEXT PRIMARY KEY,
    assessment_id TEXT NOT NULL,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    rubric_version_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'review', 'published', 'deprecated')),
    change_rationale TEXT NOT NULL CHECK (length(trim(change_rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    published_at_us INTEGER,
    UNIQUE (assessment_id, version_no),
    UNIQUE (id, learning_project_id, framework_id, framework_version_id),
    FOREIGN KEY (assessment_id, learning_project_id)
        REFERENCES assessments(id, learning_project_id) ON DELETE RESTRICT,
    FOREIGN KEY (learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_projects(id, framework_id, framework_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (rubric_version_id, learning_project_id, framework_version_id)
        REFERENCES assessment_rubric_versions(id, learning_project_id, framework_version_id)
        ON DELETE RESTRICT,
    CHECK ((status IN ('published', 'deprecated')) = (published_at_us IS NOT NULL))
);

CREATE TABLE assessment_questions (
    id TEXT PRIMARY KEY,
    assessment_version_id TEXT NOT NULL,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    objective_version_id TEXT NOT NULL,
    question_order INTEGER NOT NULL CHECK (question_order >= 0),
    prompt TEXT NOT NULL CHECK (length(trim(prompt)) > 0),
    response_kind TEXT NOT NULL CHECK (response_kind IN ('text', 'number', 'choice', 'json')),
    max_points REAL NOT NULL CHECK (max_points > 0),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    UNIQUE (assessment_version_id, question_order),
    UNIQUE (id, assessment_version_id),
    FOREIGN KEY (assessment_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES assessment_versions(id, learning_project_id, framework_id, framework_version_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT
);

CREATE TABLE assessment_version_objectives (
    assessment_version_id TEXT NOT NULL,
    objective_version_id TEXT NOT NULL,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    PRIMARY KEY (assessment_version_id, objective_version_id),
    FOREIGN KEY (assessment_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES assessment_versions(id, learning_project_id, framework_id, framework_version_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT
);

CREATE TABLE assessment_reviews (
    id TEXT PRIMARY KEY,
    assessment_version_id TEXT NOT NULL
        REFERENCES assessment_versions(id) ON DELETE RESTRICT,
    review_status TEXT NOT NULL CHECK (review_status IN ('approved', 'changes_requested', 'rejected')),
    reviewer_id TEXT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    reviewed_at_us INTEGER NOT NULL
);

CREATE TABLE assessment_attempts (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learning_learners(id) ON DELETE RESTRICT,
    assessment_version_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('in_progress', 'submitted')),
    started_at_us INTEGER NOT NULL,
    submitted_at_us INTEGER,
    UNIQUE (id, learner_id, learning_project_id),
    FOREIGN KEY (assessment_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES assessment_versions(id, learning_project_id, framework_id, framework_version_id)
        ON DELETE RESTRICT,
    CHECK ((status = 'submitted') = (submitted_at_us IS NOT NULL))
);

CREATE TABLE attempt_responses (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES assessment_attempts(id) ON DELETE RESTRICT,
    assessment_question_id TEXT NOT NULL,
    response_json TEXT NOT NULL CHECK (json_valid(response_json)),
    created_at_us INTEGER NOT NULL,
    updated_at_us INTEGER NOT NULL,
    UNIQUE (attempt_id, assessment_question_id),
    FOREIGN KEY (assessment_question_id) REFERENCES assessment_questions(id) ON DELETE RESTRICT
);

CREATE TABLE attempt_evaluations (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES assessment_attempts(id) ON DELETE RESTRICT,
    rubric_version_id TEXT NOT NULL REFERENCES assessment_rubric_versions(id) ON DELETE RESTRICT,
    revision_no INTEGER NOT NULL CHECK (revision_no > 0),
    correction_of_id TEXT REFERENCES attempt_evaluations(id) ON DELETE RESTRICT,
    evaluator_id TEXT NOT NULL CHECK (length(trim(evaluator_id)) > 0),
    outcome TEXT NOT NULL CHECK (outcome IN ('pass', 'fail', 'inconclusive')),
    score REAL NOT NULL CHECK (score >= 0 AND score <= 1),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_at_us INTEGER NOT NULL,
    UNIQUE (attempt_id, revision_no),
    UNIQUE (id, attempt_id)
);

CREATE TABLE attempt_evaluation_scores (
    evaluation_id TEXT NOT NULL REFERENCES attempt_evaluations(id) ON DELETE RESTRICT,
    rubric_criterion_id TEXT NOT NULL REFERENCES assessment_rubric_criteria(id) ON DELETE RESTRICT,
    score REAL NOT NULL CHECK (score >= 0),
    PRIMARY KEY (evaluation_id, rubric_criterion_id)
);

CREATE TABLE evaluator_feedback (
    id TEXT PRIMARY KEY,
    evaluation_id TEXT NOT NULL REFERENCES attempt_evaluations(id) ON DELETE RESTRICT,
    feedback TEXT NOT NULL CHECK (length(trim(feedback)) > 0),
    created_by TEXT NOT NULL CHECK (length(trim(created_by)) > 0),
    created_at_us INTEGER NOT NULL
);

CREATE TABLE competencies (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, learning_project_id, framework_version_id),
    FOREIGN KEY (learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_projects(id, framework_id, framework_version_id) ON DELETE RESTRICT
);

CREATE TABLE competency_criteria (
    id TEXT PRIMARY KEY,
    competency_id TEXT NOT NULL,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    objective_version_id TEXT NOT NULL,
    criterion TEXT NOT NULL CHECK (length(trim(criterion)) > 0),
    minimum_score REAL NOT NULL CHECK (minimum_score >= 0 AND minimum_score <= 1),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    FOREIGN KEY (competency_id, learning_project_id, framework_version_id)
        REFERENCES competencies(id, learning_project_id, framework_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT
);

CREATE TABLE mastery_rules (
    id TEXT PRIMARY KEY,
    competency_id TEXT NOT NULL REFERENCES competencies(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, competency_id)
);

CREATE TABLE mastery_rule_versions (
    id TEXT PRIMARY KEY,
    mastery_rule_id TEXT NOT NULL,
    competency_id TEXT NOT NULL,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    minimum_average_score REAL NOT NULL CHECK (minimum_average_score >= 0 AND minimum_average_score <= 1),
    minimum_evaluated_attempts INTEGER NOT NULL CHECK (minimum_evaluated_attempts > 0),
    all_required_criteria INTEGER NOT NULL CHECK (all_required_criteria IN (0, 1)),
    reproduction_required INTEGER NOT NULL CHECK (reproduction_required IN (0, 1)),
    required_reproduction_outcome TEXT NOT NULL CHECK (required_reproduction_outcome IN ('pass', 'fail', 'inconclusive')),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (mastery_rule_id, version_no),
    UNIQUE (id, competency_id),
    FOREIGN KEY (mastery_rule_id, competency_id)
        REFERENCES mastery_rules(id, competency_id) ON DELETE RESTRICT
);

CREATE TABLE reproduction_tasks (
    id TEXT PRIMARY KEY,
    learning_project_id TEXT NOT NULL,
    framework_id TEXT NOT NULL,
    framework_version_id TEXT NOT NULL,
    objective_version_id TEXT NOT NULL,
    rubric_version_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    task_description TEXT NOT NULL CHECK (length(trim(task_description)) > 0),
    conditions TEXT NOT NULL CHECK (length(trim(conditions)) > 0),
    expected_result_json TEXT NOT NULL CHECK (json_valid(expected_result_json)),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_projects(id, framework_id, framework_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (objective_version_id, learning_project_id, framework_id, framework_version_id)
        REFERENCES learning_objective_versions(
            id, learning_project_id, framework_id, framework_version_id
        ) ON DELETE RESTRICT,
    FOREIGN KEY (rubric_version_id, learning_project_id, framework_version_id)
        REFERENCES assessment_rubric_versions(id, learning_project_id, framework_version_id)
        ON DELETE RESTRICT
);

CREATE TABLE reproduction_submissions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES reproduction_tasks(id) ON DELETE RESTRICT,
    learner_id TEXT NOT NULL REFERENCES learning_learners(id) ON DELETE RESTRICT,
    artifact_uri TEXT,
    artifact_sha256 TEXT,
    result_json TEXT CHECK (result_json IS NULL OR json_valid(result_json)),
    deviations TEXT NOT NULL,
    submitted_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_uri IS NOT NULL AND length(trim(artifact_uri)) > 0
         AND artifact_sha256 IS NOT NULL AND length(artifact_sha256) = 64
         AND artifact_sha256 NOT GLOB '*[^0-9a-fA-F]*')
        OR (result_json IS NOT NULL AND result_json <> '{}')
    ),
    CHECK ((artifact_uri IS NULL) = (artifact_sha256 IS NULL))
);

CREATE TABLE reproduction_evaluations (
    id TEXT PRIMARY KEY,
    submission_id TEXT NOT NULL REFERENCES reproduction_submissions(id) ON DELETE RESTRICT,
    rubric_version_id TEXT NOT NULL REFERENCES assessment_rubric_versions(id) ON DELETE RESTRICT,
    evaluator_id TEXT NOT NULL CHECK (length(trim(evaluator_id)) > 0),
    outcome TEXT NOT NULL CHECK (outcome IN ('pass', 'fail', 'inconclusive')),
    score REAL NOT NULL CHECK (score >= 0 AND score <= 1),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    evidence_description TEXT NOT NULL CHECK (length(trim(evidence_description)) > 0),
    created_at_us INTEGER NOT NULL
);

CREATE TABLE reproduction_evaluation_scores (
    evaluation_id TEXT NOT NULL REFERENCES reproduction_evaluations(id) ON DELETE RESTRICT,
    rubric_criterion_id TEXT NOT NULL REFERENCES assessment_rubric_criteria(id) ON DELETE RESTRICT,
    score REAL NOT NULL CHECK (score >= 0),
    PRIMARY KEY (evaluation_id, rubric_criterion_id)
);

CREATE TABLE competency_records (
    id TEXT PRIMARY KEY,
    competency_id TEXT NOT NULL,
    learning_project_id TEXT NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learning_learners(id) ON DELETE RESTRICT,
    framework_version_id TEXT NOT NULL,
    mastery_rule_version_id TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('mastered', 'not_mastered')),
    explanation TEXT NOT NULL CHECK (length(trim(explanation)) > 0),
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (competency_id, learning_project_id, framework_version_id)
        REFERENCES competencies(id, learning_project_id, framework_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (mastery_rule_version_id, competency_id)
        REFERENCES mastery_rule_versions(id, competency_id) ON DELETE RESTRICT
);

CREATE TABLE competency_record_criteria (
    competency_record_id TEXT NOT NULL REFERENCES competency_records(id) ON DELETE RESTRICT,
    competency_criterion_id TEXT NOT NULL REFERENCES competency_criteria(id) ON DELETE RESTRICT,
    satisfied INTEGER NOT NULL CHECK (satisfied IN (0, 1)),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    PRIMARY KEY (competency_record_id, competency_criterion_id)
);

CREATE TABLE competency_record_attempt_evaluations (
    competency_record_id TEXT NOT NULL REFERENCES competency_records(id) ON DELETE RESTRICT,
    evaluation_id TEXT NOT NULL REFERENCES attempt_evaluations(id) ON DELETE RESTRICT,
    PRIMARY KEY (competency_record_id, evaluation_id)
);

CREATE TABLE competency_record_reproduction_evaluations (
    competency_record_id TEXT NOT NULL REFERENCES competency_records(id) ON DELETE RESTRICT,
    evaluation_id TEXT NOT NULL REFERENCES reproduction_evaluations(id) ON DELETE RESTRICT,
    PRIMARY KEY (competency_record_id, evaluation_id)
);

CREATE TABLE competency_record_objectives (
    competency_record_id TEXT NOT NULL REFERENCES competency_records(id) ON DELETE RESTRICT,
    objective_version_id TEXT NOT NULL REFERENCES learning_objective_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (competency_record_id, objective_version_id)
);

CREATE TRIGGER learning_project_requires_published_framework
BEFORE INSERT ON learning_projects
WHEN (SELECT status FROM knowledge_framework_versions WHERE id = NEW.framework_version_id) <> 'published'
BEGIN
    SELECT RAISE(ABORT, 'learning projects require an exact published framework version');
END;

CREATE TRIGGER learning_project_framework_pin_immutable
BEFORE UPDATE OF learner_id, framework_id, framework_version_id ON learning_projects
BEGIN
    SELECT RAISE(ABORT, 'learning project learner and framework version pins are immutable');
END;

CREATE TRIGGER learning_objective_version_no_update
BEFORE UPDATE ON learning_objective_versions
BEGIN
    SELECT RAISE(ABORT, 'learning objective versions are immutable');
END;
CREATE TRIGGER learning_objective_version_no_delete
BEFORE DELETE ON learning_objective_versions
BEGIN
    SELECT RAISE(ABORT, 'learning objective versions cannot be deleted');
END;

CREATE TRIGGER learning_objective_prerequisite_no_update
BEFORE UPDATE ON learning_objective_prerequisites
BEGIN
    SELECT RAISE(ABORT, 'objective prerequisites are immutable');
END;
CREATE TRIGGER learning_objective_prerequisite_no_delete
BEFORE DELETE ON learning_objective_prerequisites
BEGIN
    SELECT RAISE(ABORT, 'objective prerequisites cannot be deleted');
END;

CREATE TRIGGER assessment_version_published_no_content_update
BEFORE UPDATE ON assessment_versions
WHEN OLD.status = 'published' AND (
    NEW.assessment_id <> OLD.assessment_id OR NEW.version_no <> OLD.version_no
    OR NEW.learning_project_id <> OLD.learning_project_id OR NEW.framework_id <> OLD.framework_id
    OR NEW.framework_version_id <> OLD.framework_version_id
    OR NEW.rubric_version_id <> OLD.rubric_version_id
    OR NEW.change_rationale <> OLD.change_rationale OR NEW.created_by IS NOT OLD.created_by
    OR NEW.created_at_us <> OLD.created_at_us
    OR NEW.published_at_us IS NOT OLD.published_at_us
)
BEGIN
    SELECT RAISE(ABORT, 'published assessment versions are immutable');
END;
CREATE TRIGGER assessment_publication_guard
BEFORE UPDATE OF status ON assessment_versions
WHEN NEW.status = 'published' AND OLD.status <> 'published'
 AND (
    (SELECT status FROM assessment_rubric_versions WHERE id = NEW.rubric_version_id) <> 'published'
    OR NOT EXISTS (
        SELECT 1 FROM assessment_reviews r
        WHERE r.assessment_version_id = NEW.id
        ORDER BY r.reviewed_at_us DESC, r.id DESC LIMIT 1
    )
    OR (
        SELECT r.review_status FROM assessment_reviews r
        WHERE r.assessment_version_id = NEW.id
        ORDER BY r.reviewed_at_us DESC, r.id DESC LIMIT 1
    ) <> 'approved'
    OR NOT EXISTS (
        SELECT 1 FROM assessment_questions q WHERE q.assessment_version_id = NEW.id
    )
)
BEGIN
    SELECT RAISE(ABORT, 'assessment publication requires an approved review, published rubric, and questions');
END;
CREATE TRIGGER assessment_version_no_delete
BEFORE DELETE ON assessment_versions
BEGIN
    SELECT RAISE(ABORT, 'assessment versions cannot be deleted');
END;
CREATE TRIGGER assessment_question_published_no_insert
BEFORE INSERT ON assessment_questions
WHEN (SELECT status FROM assessment_versions WHERE id = NEW.assessment_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published assessment questions are immutable');
END;
CREATE TRIGGER assessment_question_published_no_update
BEFORE UPDATE ON assessment_questions
WHEN (SELECT status FROM assessment_versions WHERE id = OLD.assessment_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published assessment questions are immutable');
END;
CREATE TRIGGER assessment_question_no_delete
BEFORE DELETE ON assessment_questions
BEGIN
    SELECT RAISE(ABORT, 'assessment questions cannot be deleted');
END;
CREATE TRIGGER assessment_version_objectives_published_no_insert
BEFORE INSERT ON assessment_version_objectives
WHEN (SELECT status FROM assessment_versions WHERE id = NEW.assessment_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published assessment objectives are immutable');
END;
CREATE TRIGGER assessment_version_objectives_no_delete
BEFORE DELETE ON assessment_version_objectives
BEGIN
    SELECT RAISE(ABORT, 'assessment objective links cannot be deleted');
END;
CREATE TRIGGER assessment_version_objectives_no_update
BEFORE UPDATE ON assessment_version_objectives
BEGIN
    SELECT RAISE(ABORT, 'assessment objective links are immutable');
END;

CREATE TRIGGER rubric_version_published_no_content_update
BEFORE UPDATE ON assessment_rubric_versions
WHEN OLD.status = 'published' AND (
    NEW.rubric_id <> OLD.rubric_id OR NEW.version_no <> OLD.version_no
    OR NEW.learning_project_id <> OLD.learning_project_id
    OR NEW.framework_version_id <> OLD.framework_version_id
    OR NEW.pass_threshold <> OLD.pass_threshold
    OR NEW.created_by IS NOT OLD.created_by OR NEW.created_at_us <> OLD.created_at_us
    OR NEW.published_at_us IS NOT OLD.published_at_us
)
BEGIN
    SELECT RAISE(ABORT, 'published rubric versions are immutable');
END;
CREATE TRIGGER rubric_publication_guard
BEFORE UPDATE OF status ON assessment_rubric_versions
WHEN NEW.status = 'published' AND OLD.status <> 'published'
 AND NOT EXISTS (
    SELECT 1 FROM assessment_rubric_criteria c WHERE c.rubric_version_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'rubric publication requires at least one criterion');
END;
CREATE TRIGGER rubric_criterion_published_no_insert
BEFORE INSERT ON assessment_rubric_criteria
WHEN (SELECT status FROM assessment_rubric_versions WHERE id = NEW.rubric_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published rubric criteria are immutable');
END;
CREATE TRIGGER rubric_criterion_published_no_update
BEFORE UPDATE ON assessment_rubric_criteria
WHEN (SELECT status FROM assessment_rubric_versions WHERE id = OLD.rubric_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published rubric criteria are immutable');
END;
CREATE TRIGGER rubric_criterion_no_delete
BEFORE DELETE ON assessment_rubric_criteria
BEGIN
    SELECT RAISE(ABORT, 'rubric criteria cannot be deleted');
END;

CREATE TRIGGER assessment_attempt_submission_guard
BEFORE UPDATE ON assessment_attempts
WHEN OLD.status = 'submitted'
BEGIN
    SELECT RAISE(ABORT, 'submitted assessment attempts are immutable');
END;
CREATE TRIGGER assessment_attempt_learner_match
BEFORE INSERT ON assessment_attempts
WHEN NOT EXISTS (
    SELECT 1 FROM learning_projects p
    WHERE p.id = NEW.learning_project_id AND p.learner_id = NEW.learner_id
)
BEGIN
    SELECT RAISE(ABORT, 'assessment attempt learner must own its learning project');
END;
CREATE TRIGGER assessment_attempt_response_insert_guard
BEFORE INSERT ON attempt_responses
WHEN (SELECT status FROM assessment_attempts WHERE id = NEW.attempt_id) <> 'in_progress'
BEGIN
    SELECT RAISE(ABORT, 'responses can only be recorded on an in-progress attempt');
END;
CREATE TRIGGER assessment_attempt_response_update_guard
BEFORE UPDATE ON attempt_responses
WHEN (SELECT status FROM assessment_attempts WHERE id = OLD.attempt_id) <> 'in_progress'
 OR NEW.attempt_id <> OLD.attempt_id
 OR NEW.assessment_question_id <> OLD.assessment_question_id
 OR NOT EXISTS (
    SELECT 1 FROM assessment_attempts a
    JOIN assessment_questions q ON q.assessment_version_id = a.assessment_version_id
    WHERE a.id = NEW.attempt_id AND q.id = NEW.assessment_question_id
 )
BEGIN
    SELECT RAISE(ABORT, 'submitted attempt responses are immutable');
END;
CREATE TRIGGER assessment_attempt_response_delete_guard
BEFORE DELETE ON attempt_responses
WHEN (SELECT status FROM assessment_attempts WHERE id = OLD.attempt_id) <> 'in_progress'
BEGIN
    SELECT RAISE(ABORT, 'submitted attempt responses cannot be deleted');
END;
CREATE TRIGGER attempt_response_question_version_match
BEFORE INSERT ON attempt_responses
WHEN NOT EXISTS (
    SELECT 1 FROM assessment_attempts a
    JOIN assessment_questions q ON q.assessment_version_id = a.assessment_version_id
    WHERE a.id = NEW.attempt_id AND q.id = NEW.assessment_question_id
)
BEGIN
    SELECT RAISE(ABORT, 'attempt response question must belong to its exact assessment version');
END;

CREATE TRIGGER attempt_evaluation_submitted_guard
BEFORE INSERT ON attempt_evaluations
WHEN NOT EXISTS (
    SELECT 1 FROM assessment_attempts a
    JOIN assessment_versions av ON av.id = a.assessment_version_id
    WHERE a.id = NEW.attempt_id AND a.status = 'submitted'
      AND av.rubric_version_id = NEW.rubric_version_id
)
BEGIN
    SELECT RAISE(ABORT, 'attempt evaluation must use the submitted attempt rubric version');
END;
CREATE TRIGGER attempt_evaluation_correction_guard
BEFORE INSERT ON attempt_evaluations
WHEN (
    (NEW.revision_no = 1 AND NEW.correction_of_id IS NOT NULL)
    OR (NEW.revision_no > 1 AND (
        NEW.correction_of_id IS NULL
        OR NOT EXISTS (
            SELECT 1 FROM attempt_evaluations previous
            WHERE previous.id = NEW.correction_of_id
              AND previous.attempt_id = NEW.attempt_id
              AND previous.revision_no = NEW.revision_no - 1
        )
    ))
)
BEGIN
    SELECT RAISE(ABORT, 'evaluation corrections must explicitly supersede the immediately preceding revision');
END;
CREATE TRIGGER attempt_evaluation_no_update
BEFORE UPDATE ON attempt_evaluations
BEGIN
    SELECT RAISE(ABORT, 'attempt evaluation revisions are immutable');
END;
CREATE TRIGGER attempt_evaluation_no_delete
BEFORE DELETE ON attempt_evaluations
BEGIN
    SELECT RAISE(ABORT, 'attempt evaluation revisions cannot be deleted');
END;
CREATE TRIGGER attempt_evaluation_score_criterion_match
BEFORE INSERT ON attempt_evaluation_scores
WHEN NOT EXISTS (
    SELECT 1 FROM attempt_evaluations e
    JOIN assessment_rubric_criteria c ON c.rubric_version_id = e.rubric_version_id
    WHERE e.id = NEW.evaluation_id AND c.id = NEW.rubric_criterion_id
      AND NEW.score <= c.max_points
)
BEGIN
    SELECT RAISE(ABORT, 'criterion score must use the exact rubric and stay within its maximum');
END;
CREATE TRIGGER attempt_evaluation_scores_no_update
BEFORE UPDATE ON attempt_evaluation_scores
BEGIN
    SELECT RAISE(ABORT, 'evaluation scores are immutable');
END;
CREATE TRIGGER attempt_evaluation_scores_no_delete
BEFORE DELETE ON attempt_evaluation_scores
BEGIN
    SELECT RAISE(ABORT, 'evaluation scores cannot be deleted');
END;
CREATE TRIGGER evaluator_feedback_no_update
BEFORE UPDATE ON evaluator_feedback
BEGIN
    SELECT RAISE(ABORT, 'evaluator feedback is immutable');
END;
CREATE TRIGGER evaluator_feedback_no_delete
BEFORE DELETE ON evaluator_feedback
BEGIN
    SELECT RAISE(ABORT, 'evaluator feedback cannot be deleted');
END;

CREATE TRIGGER competency_criterion_no_update
BEFORE UPDATE ON competency_criteria
BEGIN
    SELECT RAISE(ABORT, 'competency criteria are immutable');
END;
CREATE TRIGGER competency_criterion_no_delete
BEFORE DELETE ON competency_criteria
BEGIN
    SELECT RAISE(ABORT, 'competency criteria cannot be deleted');
END;
CREATE TRIGGER mastery_rule_version_no_update
BEFORE UPDATE ON mastery_rule_versions
BEGIN
    SELECT RAISE(ABORT, 'mastery rule versions are immutable');
END;
CREATE TRIGGER mastery_rule_version_no_delete
BEFORE DELETE ON mastery_rule_versions
BEGIN
    SELECT RAISE(ABORT, 'mastery rule versions cannot be deleted');
END;

CREATE TRIGGER reproduction_submission_task_match
BEFORE INSERT ON reproduction_submissions
WHEN NOT EXISTS (
    SELECT 1 FROM reproduction_tasks t
    JOIN learning_projects p ON p.id = t.learning_project_id
    WHERE t.id = NEW.task_id AND p.learner_id = NEW.learner_id
)
BEGIN
    SELECT RAISE(ABORT, 'reproduction submission must match its learning project learner');
END;
CREATE TRIGGER reproduction_submission_no_update
BEFORE UPDATE ON reproduction_submissions
BEGIN
    SELECT RAISE(ABORT, 'reproduction submissions are immutable');
END;
CREATE TRIGGER reproduction_submission_no_delete
BEFORE DELETE ON reproduction_submissions
BEGIN
    SELECT RAISE(ABORT, 'reproduction submissions cannot be deleted');
END;
CREATE TRIGGER reproduction_evaluation_pin_guard
BEFORE INSERT ON reproduction_evaluations
WHEN NOT EXISTS (
    SELECT 1 FROM reproduction_submissions s
    JOIN reproduction_tasks t ON t.id = s.task_id
    WHERE s.id = NEW.submission_id AND t.rubric_version_id = NEW.rubric_version_id
      AND s.learner_id <> NEW.evaluator_id
)
BEGIN
    SELECT RAISE(ABORT, 'reproduction evaluation requires the task rubric and a distinct evaluator');
END;
CREATE TRIGGER reproduction_evaluation_no_update
BEFORE UPDATE ON reproduction_evaluations
BEGIN
    SELECT RAISE(ABORT, 'reproduction evaluations are immutable');
END;
CREATE TRIGGER reproduction_evaluation_no_delete
BEFORE DELETE ON reproduction_evaluations
BEGIN
    SELECT RAISE(ABORT, 'reproduction evaluations cannot be deleted');
END;
CREATE TRIGGER reproduction_evaluation_score_criterion_match
BEFORE INSERT ON reproduction_evaluation_scores
WHEN NOT EXISTS (
    SELECT 1 FROM reproduction_evaluations e
    JOIN assessment_rubric_criteria c ON c.rubric_version_id = e.rubric_version_id
    WHERE e.id = NEW.evaluation_id AND c.id = NEW.rubric_criterion_id
      AND NEW.score <= c.max_points
)
BEGIN
    SELECT RAISE(ABORT, 'reproduction score must match its exact rubric criterion');
END;
CREATE TRIGGER reproduction_evaluation_scores_no_update
BEFORE UPDATE ON reproduction_evaluation_scores
BEGIN
    SELECT RAISE(ABORT, 'reproduction evaluation scores are immutable');
END;
CREATE TRIGGER reproduction_evaluation_scores_no_delete
BEFORE DELETE ON reproduction_evaluation_scores
BEGIN
    SELECT RAISE(ABORT, 'reproduction evaluation scores cannot be deleted');
END;

CREATE TRIGGER competency_record_no_update
BEFORE UPDATE ON competency_records
BEGIN
    SELECT RAISE(ABORT, 'competency records are immutable');
END;
CREATE TRIGGER competency_record_no_delete
BEFORE DELETE ON competency_records
BEGIN
    SELECT RAISE(ABORT, 'competency records cannot be deleted');
END;
CREATE TRIGGER competency_record_attempt_match
BEFORE INSERT ON competency_record_attempt_evaluations
WHEN NOT EXISTS (
    SELECT 1 FROM competency_records cr
    JOIN attempt_evaluations e ON e.id = NEW.evaluation_id
    JOIN assessment_attempts a ON a.id = e.attempt_id
    WHERE cr.id = NEW.competency_record_id
      AND cr.learner_id = a.learner_id
      AND cr.learning_project_id = a.learning_project_id
      AND cr.framework_version_id = a.framework_version_id
      AND EXISTS (
          SELECT 1 FROM assessment_version_objectives avo
          JOIN competency_criteria cc ON cc.objective_version_id = avo.objective_version_id
          WHERE avo.assessment_version_id = a.assessment_version_id
            AND cc.competency_id = cr.competency_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'competency evidence must use the same learner and learning project');
END;
CREATE TRIGGER competency_record_reproduction_match
BEFORE INSERT ON competency_record_reproduction_evaluations
WHEN NOT EXISTS (
    SELECT 1 FROM competency_records cr
    JOIN reproduction_evaluations e ON e.id = NEW.evaluation_id
    JOIN reproduction_submissions s ON s.id = e.submission_id
    JOIN reproduction_tasks t ON t.id = s.task_id
    WHERE cr.id = NEW.competency_record_id
      AND cr.learner_id = s.learner_id
      AND cr.learning_project_id = t.learning_project_id
      AND cr.framework_version_id = t.framework_version_id
      AND EXISTS (
          SELECT 1 FROM competency_criteria cc
          WHERE cc.competency_id = cr.competency_id
            AND cc.objective_version_id = t.objective_version_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'reproduction evidence must use the same learner and learning project');
END;
CREATE TRIGGER competency_record_children_no_update
BEFORE UPDATE ON competency_record_criteria
BEGIN
    SELECT RAISE(ABORT, 'competency record criteria are immutable');
END;
CREATE TRIGGER competency_record_children_no_delete
BEFORE DELETE ON competency_record_criteria
BEGIN
    SELECT RAISE(ABORT, 'competency record criteria cannot be deleted');
END;
CREATE TRIGGER competency_record_attempts_no_update
BEFORE UPDATE ON competency_record_attempt_evaluations
BEGIN
    SELECT RAISE(ABORT, 'competency record assessment evidence is immutable');
END;
CREATE TRIGGER competency_record_attempts_no_delete
BEFORE DELETE ON competency_record_attempt_evaluations
BEGIN
    SELECT RAISE(ABORT, 'competency record assessment evidence cannot be deleted');
END;
CREATE TRIGGER competency_record_reproductions_no_update
BEFORE UPDATE ON competency_record_reproduction_evaluations
BEGIN
    SELECT RAISE(ABORT, 'competency record reproduction evidence is immutable');
END;
CREATE TRIGGER competency_record_reproductions_no_delete
BEFORE DELETE ON competency_record_reproduction_evaluations
BEGIN
    SELECT RAISE(ABORT, 'competency record reproduction evidence cannot be deleted');
END;
CREATE TRIGGER competency_record_objectives_no_update
BEFORE UPDATE ON competency_record_objectives
BEGIN
    SELECT RAISE(ABORT, 'competency record objective links are immutable');
END;
CREATE TRIGGER competency_record_objectives_no_delete
BEFORE DELETE ON competency_record_objectives
BEGIN
    SELECT RAISE(ABORT, 'competency record objective links cannot be deleted');
END;
CREATE TRIGGER competency_record_criterion_match
BEFORE INSERT ON competency_record_criteria
WHEN NOT EXISTS (
    SELECT 1 FROM competency_records cr
    JOIN competency_criteria cc ON cc.competency_id = cr.competency_id
    WHERE cr.id = NEW.competency_record_id AND cc.id = NEW.competency_criterion_id
)
BEGIN
    SELECT RAISE(ABORT, 'competency record criterion must belong to its competency');
END;
CREATE TRIGGER competency_record_objective_match
BEFORE INSERT ON competency_record_objectives
WHEN NOT EXISTS (
    SELECT 1 FROM competency_records cr
    JOIN competency_criteria cc ON cc.competency_id = cr.competency_id
    WHERE cr.id = NEW.competency_record_id
      AND cc.objective_version_id = NEW.objective_version_id
)
BEGIN
    SELECT RAISE(ABORT, 'competency record objective must belong to its competency');
END;
