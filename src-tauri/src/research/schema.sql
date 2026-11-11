CREATE TABLE research_projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    aim TEXT NOT NULL DEFAULT '',
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'proposed', 'active', 'paused', 'completed', 'archived'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    updated_at_us INTEGER NOT NULL
);

CREATE TABLE research_domains (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT '',
    parent_domain_id TEXT REFERENCES research_domains(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'proposed', 'active', 'deprecated'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (name, parent_domain_id)
);

CREATE TABLE research_pathways (
    id TEXT PRIMARY KEY,
    pathway_key TEXT NOT NULL UNIQUE CHECK (pathway_key IN (
        'existing_knowledge_acquisition',
        'existing_knowledge_investigation',
        'first_principles_discovery'
    )),
    title TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE research_project_pathways (
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    pathway_id TEXT NOT NULL REFERENCES research_pathways(id) ON DELETE RESTRICT,
    selected_at_us INTEGER NOT NULL,
    PRIMARY KEY (project_id, pathway_id)
);

CREATE TABLE research_project_domains (
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    domain_id TEXT NOT NULL REFERENCES research_domains(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL DEFAULT '',
    assigned_at_us INTEGER NOT NULL,
    PRIMARY KEY (project_id, domain_id)
);

CREATE TABLE research_project_knowledge_adoptions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    framework_id TEXT NOT NULL REFERENCES knowledge_frameworks(id) ON DELETE RESTRICT,
    framework_version_id TEXT NOT NULL REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    use_context TEXT NOT NULL,
    adopted_by TEXT,
    adopted_at_us INTEGER NOT NULL,
    UNIQUE (project_id, framework_version_id)
);

CREATE TABLE literature_reviews (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    search_scope TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN ('draft', 'reviewed')),
    reviewer_id TEXT,
    reviewed_at_us INTEGER,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK ((lifecycle_status = 'reviewed') = (reviewed_at_us IS NOT NULL))
);

CREATE TABLE literature_review_sources (
    id TEXT PRIMARY KEY,
    literature_review_id TEXT NOT NULL REFERENCES literature_reviews(id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL,
    source_version_id TEXT NOT NULL,
    source_location_id TEXT,
    source_claim_id TEXT,
    citation_note TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (source_version_id, source_id)
        REFERENCES source_versions(id, source_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_location_id, source_id, source_version_id)
        REFERENCES source_version_segments(id, source_id, source_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_claim_id, source_id, source_version_id)
        REFERENCES source_claims(id, source_id, source_version_id) ON DELETE RESTRICT,
    CHECK (source_location_id IS NULL OR source_version_id IS NOT NULL),
    CHECK (source_claim_id IS NULL OR source_version_id IS NOT NULL)
);
CREATE INDEX idx_literature_review_sources_review
    ON literature_review_sources(literature_review_id, source_version_id);

CREATE TABLE prior_knowledge_assessments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    assessment TEXT NOT NULL,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN ('draft', 'reviewed')),
    reviewer_id TEXT,
    reviewed_at_us INTEGER,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK ((lifecycle_status = 'reviewed') = (reviewed_at_us IS NOT NULL))
);

CREATE TABLE prior_knowledge_sources (
    id TEXT PRIMARY KEY,
    assessment_id TEXT NOT NULL REFERENCES prior_knowledge_assessments(id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL,
    source_version_id TEXT NOT NULL,
    source_location_id TEXT,
    source_claim_id TEXT,
    citation_note TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (source_version_id, source_id)
        REFERENCES source_versions(id, source_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_location_id, source_id, source_version_id)
        REFERENCES source_version_segments(id, source_id, source_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_claim_id, source_id, source_version_id)
        REFERENCES source_claims(id, source_id, source_version_id) ON DELETE RESTRICT
);

CREATE TABLE research_gaps (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    domain_id TEXT REFERENCES research_domains(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    statement TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'candidate', 'assessed', 'accepted', 'rejected', 'addressed', 'superseded'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    updated_at_us INTEGER NOT NULL
);

CREATE TABLE research_gap_literature_reviews (
    gap_id TEXT NOT NULL REFERENCES research_gaps(id) ON DELETE RESTRICT,
    literature_review_id TEXT NOT NULL REFERENCES literature_reviews(id) ON DELETE RESTRICT,
    PRIMARY KEY (gap_id, literature_review_id)
);

CREATE TABLE research_gap_prior_assessments (
    gap_id TEXT NOT NULL REFERENCES research_gaps(id) ON DELETE RESTRICT,
    assessment_id TEXT NOT NULL REFERENCES prior_knowledge_assessments(id) ON DELETE RESTRICT,
    PRIMARY KEY (gap_id, assessment_id)
);

CREATE TABLE research_gap_review_exceptions (
    gap_id TEXT PRIMARY KEY REFERENCES research_gaps(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    reviewer_id TEXT,
    review_decision TEXT NOT NULL CHECK (review_decision IN ('accepted', 'rejected')),
    reviewed_at_us INTEGER NOT NULL
);

CREATE TRIGGER research_gap_acceptance_requires_reviewed_basis
BEFORE UPDATE OF lifecycle_status ON research_gaps
WHEN NEW.lifecycle_status = 'accepted' AND OLD.lifecycle_status <> 'accepted'
 AND NOT EXISTS (
    SELECT 1
    FROM research_gap_literature_reviews gl
    JOIN literature_reviews lr ON lr.id = gl.literature_review_id
    JOIN literature_review_sources ls ON ls.literature_review_id = lr.id
    WHERE gl.gap_id = NEW.id AND lr.project_id = NEW.project_id
      AND lr.lifecycle_status = 'reviewed'
 )
 AND NOT EXISTS (
    SELECT 1
    FROM research_gap_prior_assessments ga
    JOIN prior_knowledge_assessments pa ON pa.id = ga.assessment_id
    WHERE ga.gap_id = NEW.id AND pa.project_id = NEW.project_id
      AND pa.lifecycle_status = 'reviewed'
 )
 AND NOT EXISTS (
    SELECT 1 FROM research_gap_review_exceptions e
    WHERE e.gap_id = NEW.id AND e.review_decision = 'accepted'
 )
BEGIN
    SELECT RAISE(ABORT, 'accepting a research gap requires reviewed evidence or a reviewed exception');
END;

CREATE TABLE research_questions (
    id TEXT PRIMARY KEY REFERENCES questions(id) ON DELETE RESTRICT,
    project_id TEXT REFERENCES research_projects(id) ON DELETE RESTRICT,
    research_gap_id TEXT REFERENCES research_gaps(id) ON DELETE RESTRICT,
    domain_id TEXT REFERENCES research_domains(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'legacy_unassessed', 'draft', 'review', 'accepted', 'rejected',
        'deferred', 'refined', 'superseded'
    )),
    ownership_state TEXT NOT NULL CHECK (ownership_state IN (
        'project', 'unassigned', 'unknown'
    )),
    migration_note TEXT,
    rationale TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    mapped_at_us INTEGER NOT NULL,
    CHECK (
        (ownership_state = 'project' AND project_id IS NOT NULL)
        OR (ownership_state IN ('unassigned', 'unknown') AND project_id IS NULL)
    ),
    UNIQUE (id, project_id)
);
CREATE INDEX idx_research_questions_project
    ON research_questions(project_id, lifecycle_status, id);
CREATE INDEX idx_research_questions_gap
    ON research_questions(research_gap_id, id);

CREATE TABLE research_question_lineage (
    id TEXT PRIMARY KEY,
    parent_question_id TEXT NOT NULL REFERENCES research_questions(id) ON DELETE RESTRICT,
    derived_question_id TEXT NOT NULL REFERENCES research_questions(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('derived_from', 'refined_from', 'supersedes')),
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (parent_question_id <> derived_question_id),
    UNIQUE (parent_question_id, derived_question_id, relation_kind)
);

CREATE TABLE research_phenomena (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    domain_id TEXT REFERENCES research_domains(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    description TEXT NOT NULL,
    population_scope TEXT NOT NULL,
    context TEXT NOT NULL,
    temporal_boundary TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'identified', 'characterized', 'retired'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);

CREATE TABLE observation_plans (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    phenomenon_id TEXT NOT NULL REFERENCES research_phenomena(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE observation_plan_versions (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES observation_plans(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    measurement_method TEXT NOT NULL,
    sample_frame TEXT NOT NULL,
    conditions_template TEXT NOT NULL,
    inclusion_rules TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'draft', 'reviewed', 'preregistered', 'withdrawn'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    preregistered_at_us INTEGER,
    UNIQUE (plan_id, version_no),
    UNIQUE (id, plan_id),
    CHECK ((lifecycle_status = 'preregistered') = (preregistered_at_us IS NOT NULL))
);
CREATE TRIGGER observation_plan_versions_locked_no_update
BEFORE UPDATE ON observation_plan_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered observation-plan versions are immutable');
END;
CREATE TRIGGER observation_plan_versions_locked_no_delete
BEFORE DELETE ON observation_plan_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered observation-plan versions are immutable');
END;

CREATE TABLE research_dataset_version_references (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) > 0),
    external_uri TEXT,
    content_sha256 TEXT CHECK (
        content_sha256 IS NULL OR
        (length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    source_version_id TEXT REFERENCES source_versions(id) ON DELETE RESTRICT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (project_id, title, version_label),
    CHECK (external_uri IS NOT NULL OR content_sha256 IS NOT NULL OR source_version_id IS NOT NULL)
);
CREATE TRIGGER research_dataset_references_no_update
BEFORE UPDATE ON research_dataset_version_references
BEGIN
    SELECT RAISE(ABORT, 'dataset-version references are immutable');
END;
CREATE TRIGGER research_dataset_references_no_delete
BEFORE DELETE ON research_dataset_version_references
BEGIN
    SELECT RAISE(ABORT, 'dataset-version references are immutable');
END;

CREATE TABLE research_observations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    phenomenon_id TEXT NOT NULL REFERENCES research_phenomena(id) ON DELETE RESTRICT,
    plan_id TEXT NOT NULL REFERENCES observation_plans(id) ON DELETE RESTRICT,
    plan_version_id TEXT NOT NULL REFERENCES observation_plan_versions(id) ON DELETE RESTRICT,
    dataset_version_reference_id TEXT REFERENCES research_dataset_version_references(id) ON DELETE RESTRICT,
    measured_at_us INTEGER NOT NULL,
    value_decimal TEXT,
    value_state TEXT NOT NULL CHECK (value_state IN (
        'observed', 'not_measured', 'not_recorded', 'unknown',
        'not_applicable', 'withheld', 'invalid'
    )),
    unit TEXT,
    measurement_method TEXT NOT NULL,
    conditions TEXT NOT NULL,
    sample_identifier TEXT,
    quality_status TEXT NOT NULL CHECK (quality_status IN (
        'unreviewed', 'accepted', 'excluded', 'retracted'
    )),
    supersedes_observation_id TEXT REFERENCES research_observations(id) ON DELETE RESTRICT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (plan_version_id, plan_id)
        REFERENCES observation_plan_versions(id, plan_id) ON DELETE RESTRICT,
    CHECK (
        (value_state = 'observed' AND value_decimal IS NOT NULL AND unit IS NOT NULL)
        OR (value_state <> 'observed' AND value_decimal IS NULL)
    ),
    CHECK (supersedes_observation_id IS NULL OR supersedes_observation_id <> id)
);
CREATE INDEX idx_research_observations_project
    ON research_observations(project_id, measured_at_us, id);
CREATE TRIGGER research_observations_no_update
BEFORE UPDATE ON research_observations
BEGIN
    SELECT RAISE(ABORT, 'observations are immutable; create a superseding observation');
END;
CREATE TRIGGER research_observations_no_delete
BEFORE DELETE ON research_observations
BEGIN
    SELECT RAISE(ABORT, 'observations cannot be deleted');
END;

CREATE TABLE research_patterns (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    statement TEXT NOT NULL,
    analysis_method TEXT NOT NULL,
    uncertainty TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'candidate', 'examined', 'supported', 'unsupported', 'retired'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);

CREATE TABLE research_pattern_observations (
    pattern_id TEXT NOT NULL REFERENCES research_patterns(id) ON DELETE RESTRICT,
    observation_id TEXT NOT NULL REFERENCES research_observations(id) ON DELETE RESTRICT,
    PRIMARY KEY (pattern_id, observation_id)
);

CREATE TABLE research_pattern_dataset_versions (
    pattern_id TEXT NOT NULL REFERENCES research_patterns(id) ON DELETE RESTRICT,
    dataset_version_reference_id TEXT NOT NULL
        REFERENCES research_dataset_version_references(id) ON DELETE RESTRICT,
    PRIMARY KEY (pattern_id, dataset_version_reference_id)
);

CREATE TABLE research_ideas (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    content TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'captured', 'explored', 'promoted', 'abandoned'
    )),
    phenomenon_id TEXT REFERENCES research_phenomena(id) ON DELETE RESTRICT,
    observation_id TEXT REFERENCES research_observations(id) ON DELETE RESTRICT,
    pattern_id TEXT REFERENCES research_patterns(id) ON DELETE RESTRICT,
    question_id TEXT REFERENCES research_questions(id) ON DELETE RESTRICT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (phenomenon_id IS NOT NULL) + (observation_id IS NOT NULL) +
        (pattern_id IS NOT NULL) + (question_id IS NOT NULL) <= 1
    )
);

CREATE TABLE research_hypotheses (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES research_projects(id) ON DELETE RESTRICT,
    research_question_id TEXT REFERENCES research_questions(id) ON DELETE RESTRICT,
    legacy_hypothesis_id TEXT UNIQUE REFERENCES hypotheses(id) ON DELETE RESTRICT,
    parent_hypothesis_id TEXT REFERENCES research_hypotheses(id) ON DELETE RESTRICT,
    ownership_state TEXT NOT NULL CHECK (ownership_state IN (
        'project', 'unassigned', 'unknown'
    )),
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'legacy_untyped', 'draft', 'review', 'preregistered', 'testing', 'closed'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (ownership_state = 'project' AND project_id IS NOT NULL)
        OR (ownership_state IN ('unassigned', 'unknown') AND project_id IS NULL)
    ),
    CHECK (research_question_id IS NOT NULL OR lifecycle_status = 'legacy_untyped')
);

CREATE TABLE hypothesis_definition_versions (
    id TEXT PRIMARY KEY,
    hypothesis_id TEXT NOT NULL REFERENCES research_hypotheses(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    parent_version_id TEXT REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    statement_h0 TEXT NOT NULL CHECK (length(trim(statement_h0)) > 0),
    statement_h1 TEXT NOT NULL CHECK (length(trim(statement_h1)) > 0),
    population_scope TEXT NOT NULL CHECK (length(trim(population_scope)) > 0),
    research_scope TEXT NOT NULL CHECK (length(trim(research_scope)) > 0),
    expected_direction TEXT NOT NULL CHECK (expected_direction IN (
        'positive', 'negative', 'non_directional', 'equivalence', 'other'
    )),
    expected_effect_decimal TEXT,
    expected_effect_unit TEXT,
    falsification_criteria TEXT NOT NULL CHECK (length(trim(falsification_criteria)) > 0),
    assumptions TEXT NOT NULL DEFAULT '',
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'draft', 'review', 'preregistered'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    preregistered_at_us INTEGER,
    UNIQUE (hypothesis_id, version_no),
    UNIQUE (id, hypothesis_id),
    CHECK (
        (expected_effect_decimal IS NULL AND expected_effect_unit IS NULL)
        OR (expected_effect_decimal IS NOT NULL AND expected_effect_unit IS NOT NULL)
    ),
    CHECK ((lifecycle_status = 'preregistered') = (preregistered_at_us IS NOT NULL))
);
CREATE INDEX idx_hypothesis_definition_versions
    ON hypothesis_definition_versions(hypothesis_id, version_no DESC);
CREATE TRIGGER hypothesis_preregistered_versions_no_update
BEFORE UPDATE ON hypothesis_definition_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered hypothesis definitions are immutable');
END;
CREATE TRIGGER hypothesis_preregistered_versions_no_delete
BEFORE DELETE ON hypothesis_definition_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered hypothesis definitions are immutable');
END;

CREATE TABLE hypothesis_variables (
    id TEXT PRIMARY KEY,
    hypothesis_version_id TEXT NOT NULL REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN ('independent', 'dependent', 'covariate')),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    operational_definition TEXT NOT NULL CHECK (length(trim(operational_definition)) > 0),
    value_kind TEXT NOT NULL CHECK (value_kind IN (
        'continuous', 'discrete', 'categorical', 'qualitative', 'other'
    )),
    unit TEXT,
    measurement_method TEXT NOT NULL CHECK (length(trim(measurement_method)) > 0),
    UNIQUE (hypothesis_version_id, role, name)
);
CREATE TRIGGER hypothesis_preregistered_variables_no_insert
BEFORE INSERT ON hypothesis_variables
WHEN (SELECT lifecycle_status FROM hypothesis_definition_versions WHERE id = NEW.hypothesis_version_id) = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered hypothesis variables are immutable');
END;
CREATE TRIGGER hypothesis_preregistered_variables_no_update
BEFORE UPDATE ON hypothesis_variables
WHEN (SELECT lifecycle_status FROM hypothesis_definition_versions WHERE id = OLD.hypothesis_version_id) = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered hypothesis variables are immutable');
END;
CREATE TRIGGER hypothesis_preregistered_variables_no_delete
BEFORE DELETE ON hypothesis_variables
WHEN (SELECT lifecycle_status FROM hypothesis_definition_versions WHERE id = OLD.hypothesis_version_id) = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered hypothesis variables are immutable');
END;

CREATE TABLE hypothesis_competitions (
    hypothesis_id TEXT NOT NULL REFERENCES research_hypotheses(id) ON DELETE RESTRICT,
    competing_hypothesis_id TEXT NOT NULL REFERENCES research_hypotheses(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    PRIMARY KEY (hypothesis_id, competing_hypothesis_id),
    CHECK (hypothesis_id <> competing_hypothesis_id)
);

CREATE TABLE hypothesis_model_references (
    id TEXT PRIMARY KEY,
    hypothesis_id TEXT NOT NULL REFERENCES research_hypotheses(id) ON DELETE RESTRICT,
    model_id TEXT NOT NULL,
    model_version_id TEXT NOT NULL,
    label TEXT NOT NULL,
    rationale TEXT NOT NULL,
    resolution_status TEXT NOT NULL DEFAULT 'unresolved'
        CHECK (resolution_status = 'unresolved'),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (hypothesis_id, model_id, model_version_id)
);

CREATE TABLE hypothesis_source_claim_evidence (
    id TEXT PRIMARY KEY,
    hypothesis_version_id TEXT NOT NULL REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    source_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    polarity TEXT NOT NULL CHECK (polarity IN ('supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (hypothesis_version_id, source_claim_id, polarity)
);

CREATE TABLE hypothesis_observation_evidence (
    id TEXT PRIMARY KEY,
    hypothesis_version_id TEXT NOT NULL REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    observation_id TEXT NOT NULL REFERENCES research_observations(id) ON DELETE RESTRICT,
    polarity TEXT NOT NULL CHECK (polarity IN ('supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (hypothesis_version_id, observation_id, polarity)
);

CREATE TABLE hypothesis_pattern_evidence (
    id TEXT PRIMARY KEY,
    hypothesis_version_id TEXT NOT NULL REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    pattern_id TEXT NOT NULL REFERENCES research_patterns(id) ON DELETE RESTRICT,
    polarity TEXT NOT NULL CHECK (polarity IN ('supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (hypothesis_version_id, pattern_id, polarity)
);

CREATE TABLE research_experiment_designs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE research_experiment_design_versions (
    id TEXT PRIMARY KEY,
    design_id TEXT NOT NULL REFERENCES research_experiment_designs(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    description TEXT NOT NULL,
    protocol TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'draft', 'review', 'preregistered', 'withdrawn'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    preregistered_at_us INTEGER,
    UNIQUE (design_id, version_no),
    UNIQUE (id, design_id),
    CHECK ((lifecycle_status = 'preregistered') = (preregistered_at_us IS NOT NULL))
);
CREATE TRIGGER research_design_preregistered_no_update
BEFORE UPDATE ON research_experiment_design_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered design versions are immutable');
END;
CREATE TRIGGER research_design_preregistered_no_delete
BEFORE DELETE ON research_experiment_design_versions
WHEN OLD.lifecycle_status = 'preregistered'
BEGIN
    SELECT RAISE(ABORT, 'preregistered design versions are immutable');
END;

CREATE TABLE hypothesis_design_links (
    hypothesis_version_id TEXT NOT NULL REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    design_version_id TEXT NOT NULL REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    relationship_kind TEXT NOT NULL CHECK (relationship_kind IN ('tests', 'competing_design')),
    rationale TEXT NOT NULL,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    PRIMARY KEY (hypothesis_version_id, design_version_id, relationship_kind)
);

CREATE TABLE research_provenance_events (
    id TEXT PRIMARY KEY,
    target_kind TEXT NOT NULL CHECK (target_kind IN (
        'project', 'domain', 'gap', 'question', 'phenomenon', 'observation_plan',
        'observation', 'pattern', 'idea', 'hypothesis', 'hypothesis_version',
        'experiment_design', 'literature_review', 'prior_knowledge_assessment'
    )),
    target_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    actor_id TEXT,
    rationale TEXT NOT NULL DEFAULT '',
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_research_provenance_target
    ON research_provenance_events(target_kind, target_id, occurred_at_us, id);
CREATE TRIGGER research_provenance_events_no_update
BEFORE UPDATE ON research_provenance_events
BEGIN
    SELECT RAISE(ABORT, 'research provenance events are append-only');
END;
CREATE TRIGGER research_provenance_events_no_delete
BEFORE DELETE ON research_provenance_events
BEGIN
    SELECT RAISE(ABORT, 'research provenance events are append-only');
END;
