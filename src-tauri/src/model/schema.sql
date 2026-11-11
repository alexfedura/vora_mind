CREATE TABLE IF NOT EXISTS model_schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at_us INTEGER NOT NULL
);

CREATE TABLE scientific_models (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT '',
    origin_project_id TEXT REFERENCES research_projects(id) ON DELETE RESTRICT,
    domain_id TEXT REFERENCES research_domains(id) ON DELETE RESTRICT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);

CREATE TABLE model_status_events (
    id TEXT PRIMARY KEY,
    model_id TEXT NOT NULL REFERENCES scientific_models(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'deprecated', 'replaced', 'retired')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);

CREATE TABLE model_versions (
    id TEXT PRIMARY KEY,
    model_id TEXT NOT NULL REFERENCES scientific_models(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    created_in_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    change_class TEXT NOT NULL CHECK (
        change_class IN ('refinement', 'extension', 'revision', 'replacement')
    ),
    change_rationale TEXT NOT NULL CHECK (length(trim(change_rationale)) > 0),
    expected_implications TEXT NOT NULL CHECK (length(trim(expected_implications)) > 0),
    human_specification TEXT NOT NULL CHECK (length(trim(human_specification)) > 0),
    machine_specification_json TEXT NOT NULL CHECK (json_valid(machine_specification_json)),
    scope TEXT NOT NULL CHECK (length(trim(scope)) > 0),
    dependencies_json TEXT NOT NULL CHECK (json_valid(dependencies_json)),
    environment_json TEXT NOT NULL CHECK (json_valid(environment_json)),
    content_sha256 TEXT NOT NULL CHECK (
        length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (model_id, version_no),
    UNIQUE (id, model_id)
);
CREATE INDEX idx_model_versions_model ON model_versions(model_id, version_no DESC);
CREATE INDEX idx_model_versions_project ON model_versions(created_in_project_id, created_at_us);

CREATE TABLE model_version_parents (
    version_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    parent_version_id TEXT NOT NULL,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    PRIMARY KEY (version_id, parent_version_id),
    FOREIGN KEY (version_id, model_id) REFERENCES model_versions(id, model_id) ON DELETE RESTRICT,
    FOREIGN KEY (parent_version_id, model_id) REFERENCES model_versions(id, model_id) ON DELETE RESTRICT,
    CHECK (version_id <> parent_version_id)
);

CREATE TABLE model_version_events (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (
        lifecycle_status IN ('draft', 'review', 'released', 'superseded', 'withdrawn')
    ),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_model_version_events
    ON model_version_events(model_version_id, occurred_at_us, id);

CREATE TABLE model_version_parameters (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    value TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK (value_kind IN ('decimal', 'integer', 'boolean', 'string', 'json')),
    unit TEXT,
    lower_bound TEXT,
    upper_bound TEXT,
    default_value TEXT,
    role TEXT NOT NULL CHECK (length(trim(role)) > 0),
    tuneable INTEGER NOT NULL CHECK (tuneable IN (0, 1)),
    UNIQUE (model_version_id, name)
);

CREATE TABLE model_version_dependencies (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    version TEXT NOT NULL CHECK (length(trim(version)) > 0),
    source_uri TEXT,
    checksum_sha256 TEXT CHECK (
        checksum_sha256 IS NULL OR
        (length(checksum_sha256) = 64 AND checksum_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    UNIQUE (model_version_id, name, version)
);

CREATE TABLE model_version_artifacts (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    artifact_kind TEXT NOT NULL CHECK (length(trim(artifact_kind)) > 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    artifact_uri TEXT NOT NULL CHECK (length(trim(artifact_uri)) > 0),
    checksum_sha256 TEXT NOT NULL CHECK (
        length(checksum_sha256) = 64 AND checksum_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    media_type TEXT,
    byte_length INTEGER CHECK (byte_length IS NULL OR byte_length >= 0)
);

CREATE TABLE model_version_assumptions (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    assumption_key TEXT NOT NULL CHECK (length(trim(assumption_key)) > 0),
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    UNIQUE (model_version_id, assumption_key),
    UNIQUE (id, model_version_id, assumption_key)
);

CREATE TABLE model_version_impacts (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    item_kind TEXT NOT NULL CHECK (item_kind IN ('claim', 'assumption')),
    impact_kind TEXT NOT NULL CHECK (impact_kind IN ('added', 'changed', 'removed', 'affected')),
    source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    hypothesis_version_id TEXT REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    assumption_key TEXT,
    previous_statement TEXT,
    current_statement TEXT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    FOREIGN KEY (model_version_id, assumption_key)
        REFERENCES model_version_assumptions(model_version_id, assumption_key) ON DELETE RESTRICT,
    CHECK (
        (item_kind = 'claim'
         AND ((source_claim_id IS NOT NULL) + (hypothesis_version_id IS NOT NULL)
              + (finding_version_id IS NOT NULL) = 1)
         AND assumption_key IS NULL)
        OR
        (item_kind = 'assumption'
         AND source_claim_id IS NULL AND hypothesis_version_id IS NULL
         AND finding_version_id IS NULL AND assumption_key IS NOT NULL)
    ),
    CHECK (impact_kind <> 'added' OR current_statement IS NOT NULL),
    CHECK (impact_kind <> 'removed' OR previous_statement IS NOT NULL)
);

CREATE TABLE model_contributions (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN (
        'contributes_to', 'contradicts', 'qualifies', 'tested_in',
        'dataset_use', 'evidenced_by'
    )),
    hypothesis_version_id TEXT
        REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    design_version_id TEXT
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    experiment_run_id TEXT REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    dataset_version_id TEXT REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    dataset_role TEXT CHECK (dataset_role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    split_group_id TEXT,
    evidence_id TEXT REFERENCES model_evidence(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (split_group_id, dataset_version_id)
        REFERENCES experiment_dataset_split_groups(id, dataset_version_id) ON DELETE RESTRICT,
    CHECK (
        (hypothesis_version_id IS NOT NULL)
        + (finding_version_id IS NOT NULL)
        + (design_version_id IS NOT NULL)
        + (dataset_version_id IS NOT NULL)
        + (evidence_id IS NOT NULL) = 1
    ),
    CHECK (
        (relation_kind = 'contributes_to' AND
            (hypothesis_version_id IS NOT NULL OR finding_version_id IS NOT NULL
             OR design_version_id IS NOT NULL) AND experiment_run_id IS NULL
             AND dataset_role IS NULL AND split_group_id IS NULL AND evidence_id IS NULL)
        OR (relation_kind = 'contradicts' AND
            (hypothesis_version_id IS NOT NULL OR finding_version_id IS NOT NULL)
            AND design_version_id IS NULL AND experiment_run_id IS NULL
            AND dataset_role IS NULL AND split_group_id IS NULL AND evidence_id IS NULL)
        OR (relation_kind = 'qualifies' AND finding_version_id IS NOT NULL
            AND hypothesis_version_id IS NULL AND design_version_id IS NULL
            AND experiment_run_id IS NULL AND dataset_role IS NULL
            AND split_group_id IS NULL AND evidence_id IS NULL)
        OR (relation_kind = 'tested_in' AND design_version_id IS NOT NULL
            AND experiment_run_id IS NOT NULL AND hypothesis_version_id IS NULL
            AND finding_version_id IS NULL AND dataset_version_id IS NULL
            AND dataset_role IS NULL AND split_group_id IS NULL AND evidence_id IS NULL)
        OR (relation_kind = 'dataset_use' AND dataset_version_id IS NOT NULL
            AND dataset_role IS NOT NULL AND hypothesis_version_id IS NULL
            AND finding_version_id IS NULL AND design_version_id IS NULL
            AND experiment_run_id IS NULL AND split_group_id IS NULL AND evidence_id IS NULL)
        OR (relation_kind = 'evidenced_by' AND evidence_id IS NOT NULL
            AND hypothesis_version_id IS NULL AND finding_version_id IS NULL
            AND design_version_id IS NULL AND experiment_run_id IS NULL
            AND dataset_version_id IS NULL AND dataset_role IS NULL
            AND split_group_id IS NULL)
    )
);
CREATE UNIQUE INDEX idx_model_contribution_hypothesis
    ON model_contributions(model_version_id, hypothesis_version_id, relation_kind)
    WHERE hypothesis_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_model_contribution_finding
    ON model_contributions(model_version_id, finding_version_id, relation_kind)
    WHERE finding_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_model_contribution_experiment
    ON model_contributions(
        model_version_id, design_version_id, COALESCE(experiment_run_id, ''), relation_kind
    )
    WHERE design_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_model_contribution_dataset
    ON model_contributions(
        model_version_id, dataset_version_id, dataset_role, COALESCE(split_group_id, '')
    )
    WHERE dataset_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_model_contribution_evidence
    ON model_contributions(model_version_id, evidence_id)
    WHERE evidence_id IS NOT NULL;

CREATE TABLE model_evidence (
    id TEXT PRIMARY KEY,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    artifact_analysis_output_id TEXT REFERENCES analysis_outputs(id) ON DELETE RESTRICT,
    artifact_experiment_run_id TEXT REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    artifact_dataset_version_id TEXT REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    artifact_observation_id TEXT REFERENCES research_observations(id) ON DELETE RESTRICT,
    artifact_source_segment_id TEXT REFERENCES source_version_segments(id) ON DELETE RESTRICT,
    artifact_source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    artifact_finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    polarity TEXT NOT NULL CHECK (polarity IN ('supports', 'contradicts', 'qualifies', 'neutral')),
    scope TEXT NOT NULL CHECK (length(trim(scope)) > 0),
    quality_rationale TEXT NOT NULL CHECK (length(trim(quality_rationale)) > 0),
    limitations TEXT NOT NULL CHECK (length(trim(limitations)) > 0),
    provenance_json TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_analysis_output_id IS NOT NULL)
        + (artifact_experiment_run_id IS NOT NULL)
        + (artifact_dataset_version_id IS NOT NULL)
        + (artifact_observation_id IS NOT NULL)
        + (artifact_source_segment_id IS NOT NULL)
        + (artifact_source_claim_id IS NOT NULL)
        + (artifact_finding_version_id IS NOT NULL) = 1
    )
);
CREATE INDEX idx_model_evidence_version ON model_evidence(model_version_id, created_at_us);

CREATE TABLE model_evaluations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    analysis_run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    analysis_run_input_id TEXT NOT NULL,
    dataset_version_id TEXT NOT NULL,
    analysis_metric_id TEXT NOT NULL REFERENCES analysis_metrics(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    split_group_id TEXT,
    limitations TEXT NOT NULL CHECK (length(trim(limitations)) > 0),
    created_by TEXT,
    evaluated_at_us INTEGER NOT NULL,
    FOREIGN KEY (analysis_run_input_id, dataset_version_id)
        REFERENCES analysis_run_inputs(id, dataset_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (split_group_id, dataset_version_id)
        REFERENCES experiment_dataset_split_groups(id, dataset_version_id) ON DELETE RESTRICT
);
CREATE INDEX idx_model_evaluations_version ON model_evaluations(model_version_id, evaluated_at_us);
CREATE INDEX idx_model_evaluations_project ON model_evaluations(project_id, evaluated_at_us);

CREATE TABLE model_validations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    target_model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    target_finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    target_source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    criteria_json TEXT NOT NULL CHECK (json_valid(criteria_json)),
    criteria_sha256 TEXT NOT NULL CHECK (
        length(criteria_sha256) = 64 AND criteria_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    scope TEXT NOT NULL CHECK (length(trim(scope)) > 0),
    population_scope TEXT NOT NULL CHECK (length(trim(population_scope)) > 0),
    evaluation_method TEXT NOT NULL CHECK (length(trim(evaluation_method)) > 0),
    evaluator_identity TEXT NOT NULL CHECK (length(trim(evaluator_identity)) > 0),
    independence_state TEXT NOT NULL CHECK (
        independence_state IN ('independent', 'not_independent', 'unknown')
    ),
    independence_rationale TEXT NOT NULL,
    conflict_of_interest TEXT NOT NULL CHECK (length(trim(conflict_of_interest)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (target_model_version_id IS NOT NULL)
        + (target_finding_version_id IS NOT NULL)
        + (target_source_claim_id IS NOT NULL) = 1
    ),
    CHECK (json_type(criteria_json) = 'array' AND json_array_length(criteria_json) > 0),
    CHECK (independence_state = 'independent' OR length(trim(independence_rationale)) > 0)
);
CREATE INDEX idx_model_validations_project ON model_validations(project_id, created_at_us);

CREATE TABLE model_validation_events (
    id TEXT PRIMARY KEY,
    validation_id TEXT NOT NULL REFERENCES model_validations(id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK (outcome IN ('pass', 'fail', 'inconclusive')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    deviations_json TEXT NOT NULL CHECK (json_valid(deviations_json)),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_model_validation_single_result
    ON model_validation_events(validation_id);

CREATE TABLE model_validation_analysis_evidence (
    validation_id TEXT NOT NULL REFERENCES model_validations(id) ON DELETE RESTRICT,
    evidence_id TEXT NOT NULL REFERENCES analysis_evidence(id) ON DELETE RESTRICT,
    PRIMARY KEY (validation_id, evidence_id)
);

CREATE TABLE model_validation_model_evidence (
    validation_id TEXT NOT NULL REFERENCES model_validations(id) ON DELETE RESTRICT,
    evidence_id TEXT NOT NULL REFERENCES model_evidence(id) ON DELETE RESTRICT,
    PRIMARY KEY (validation_id, evidence_id)
);

CREATE TABLE model_replications (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    model_version_id TEXT NOT NULL REFERENCES model_versions(id) ON DELETE RESTRICT,
    original_experiment_run_id TEXT NOT NULL REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    original_design_version_id TEXT NOT NULL
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    replication_kind TEXT NOT NULL CHECK (replication_kind IN ('direct', 'conceptual')),
    independent_team_operator TEXT NOT NULL CHECK (length(trim(independent_team_operator)) > 0),
    protocol_context TEXT NOT NULL CHECK (length(trim(protocol_context)) > 0),
    dataset_context_json TEXT NOT NULL CHECK (json_valid(dataset_context_json)),
    outcome TEXT NOT NULL CHECK (outcome IN ('supports', 'contradicts', 'mixed', 'inconclusive')),
    result_summary TEXT NOT NULL CHECK (length(trim(result_summary)) > 0),
    deviations_json TEXT NOT NULL CHECK (json_valid(deviations_json)),
    provenance_json TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);
CREATE INDEX idx_model_replications_version ON model_replications(model_version_id, created_at_us);

CREATE TABLE model_replication_datasets (
    replication_id TEXT NOT NULL REFERENCES model_replications(id) ON DELETE RESTRICT,
    dataset_version_id TEXT NOT NULL REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    split_group_id TEXT,
    FOREIGN KEY (split_group_id, dataset_version_id)
        REFERENCES experiment_dataset_split_groups(id, dataset_version_id) ON DELETE RESTRICT,
    PRIMARY KEY (replication_id, dataset_version_id, role)
);

CREATE TABLE model_replication_analysis_evidence (
    replication_id TEXT NOT NULL REFERENCES model_replications(id) ON DELETE RESTRICT,
    evidence_id TEXT NOT NULL REFERENCES analysis_evidence(id) ON DELETE RESTRICT,
    PRIMARY KEY (replication_id, evidence_id)
);

CREATE TABLE model_replication_model_evidence (
    replication_id TEXT NOT NULL REFERENCES model_replications(id) ON DELETE RESTRICT,
    evidence_id TEXT NOT NULL REFERENCES model_evidence(id) ON DELETE RESTRICT,
    PRIMARY KEY (replication_id, evidence_id)
);

CREATE TRIGGER model_contribution_dataset_require_sealed
BEFORE INSERT ON model_contributions
WHEN NEW.relation_kind = 'dataset_use'
 AND (SELECT lifecycle_status FROM experiment_dataset_versions
      WHERE id = NEW.dataset_version_id) <> 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'model dataset contributions require a sealed dataset version');
END;
CREATE TRIGGER model_evaluation_exact_pins
BEFORE INSERT ON model_evaluations
WHEN NOT EXISTS (
    SELECT 1
    FROM analysis_runs r
    JOIN analysis_run_inputs i ON i.id = NEW.analysis_run_input_id
    JOIN analysis_metrics m ON m.id = NEW.analysis_metric_id
    JOIN experiment_dataset_versions d ON d.id = NEW.dataset_version_id
    JOIN analysis_run_events e ON e.run_id = r.id AND e.lifecycle_status = 'completed'
    WHERE r.id = NEW.analysis_run_id
      AND r.project_id = NEW.project_id
      AND i.run_id = r.id
      AND i.dataset_version_id = NEW.dataset_version_id
      AND i.role = NEW.role
      AND i.split_group_id IS NEW.split_group_id
      AND m.run_id = r.id
      AND d.lifecycle_status = 'sealed'
      AND NOT EXISTS (
          SELECT 1 FROM analysis_run_events later
          WHERE later.run_id = r.id
            AND (later.occurred_at_us > e.occurred_at_us
                 OR (later.occurred_at_us = e.occurred_at_us AND later.id > e.id))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'model evaluation must pin a completed run, exact input, dataset, role, split, and metric');
END;
CREATE TRIGGER model_contribution_experiment_run_design_match
BEFORE INSERT ON model_contributions
WHEN NEW.experiment_run_id IS NOT NULL
 AND NOT EXISTS (
     SELECT 1 FROM experiment_runs
     WHERE id = NEW.experiment_run_id AND design_version_id = NEW.design_version_id
 )
BEGIN
    SELECT RAISE(ABORT, 'model experiment contribution must pin the run design version');
END;
CREATE TRIGGER model_replication_original_pin
BEFORE INSERT ON model_replications
WHEN NOT EXISTS (
    SELECT 1 FROM experiment_runs r
    WHERE r.id = NEW.original_experiment_run_id
      AND r.design_version_id = NEW.original_design_version_id
      AND r.lifecycle_status IN ('completed', 'inconclusive')
)
BEGIN
    SELECT RAISE(ABORT, 'replication must pin a completed or inconclusive original run and design');
END;
CREATE TRIGGER model_replication_dataset_sealed
BEFORE INSERT ON model_replication_datasets
WHEN (SELECT lifecycle_status FROM experiment_dataset_versions
      WHERE id = NEW.dataset_version_id) <> 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'replication dataset pins require sealed versions');
END;
CREATE TRIGGER model_contribution_evidence_identity_match
BEFORE INSERT ON model_contributions
WHEN NEW.evidence_id IS NOT NULL AND NOT EXISTS (
    SELECT 1
    FROM model_versions target
    JOIN model_evidence e ON e.id = NEW.evidence_id
    JOIN model_versions evidence_version ON evidence_version.id = e.model_version_id
    WHERE target.id = NEW.model_version_id
      AND target.model_id = evidence_version.model_id
)
BEGIN
    SELECT RAISE(ABORT, 'model evidence contributions must remain within one model identity');
END;
CREATE TRIGGER model_replication_evidence_version_match
BEFORE INSERT ON model_replication_model_evidence
WHEN NOT EXISTS (
    SELECT 1 FROM model_replications r
    JOIN model_evidence e ON e.id = NEW.evidence_id
    WHERE r.id = NEW.replication_id AND r.model_version_id = e.model_version_id
)
BEGIN
    SELECT RAISE(ABORT, 'replication evidence must target the replicated model version');
END;

CREATE TRIGGER model_identity_no_update
BEFORE UPDATE ON scientific_models
BEGIN
    SELECT RAISE(ABORT, 'model identities are immutable');
END;
CREATE TRIGGER model_identity_no_delete
BEFORE DELETE ON scientific_models
BEGIN
    SELECT RAISE(ABORT, 'model identities cannot be deleted');
END;
CREATE TRIGGER model_status_events_no_update
BEFORE UPDATE ON model_status_events
BEGIN
    SELECT RAISE(ABORT, 'model status events are immutable');
END;
CREATE TRIGGER model_status_events_no_delete
BEFORE DELETE ON model_status_events
BEGIN
    SELECT RAISE(ABORT, 'model status events cannot be deleted');
END;
CREATE TRIGGER model_versions_no_update
BEFORE UPDATE ON model_versions
BEGIN
    SELECT RAISE(ABORT, 'model versions are immutable');
END;
CREATE TRIGGER model_version_events_no_update
BEFORE UPDATE ON model_version_events
BEGIN
    SELECT RAISE(ABORT, 'model version lifecycle events are immutable');
END;
CREATE TRIGGER model_version_events_no_delete
BEFORE DELETE ON model_version_events
BEGIN
    SELECT RAISE(ABORT, 'model version lifecycle events cannot be deleted');
END;
CREATE TRIGGER model_versions_no_delete
BEFORE DELETE ON model_versions
BEGIN
    SELECT RAISE(ABORT, 'model versions cannot be deleted');
END;
CREATE TRIGGER model_version_children_no_update
BEFORE UPDATE ON model_version_parameters
BEGIN
    SELECT RAISE(ABORT, 'model version parameters are immutable');
END;
CREATE TRIGGER model_version_parameters_no_delete
BEFORE DELETE ON model_version_parameters
BEGIN
    SELECT RAISE(ABORT, 'model version parameters cannot be deleted');
END;
CREATE TRIGGER model_version_dependencies_no_update
BEFORE UPDATE ON model_version_dependencies
BEGIN
    SELECT RAISE(ABORT, 'model version dependencies are immutable');
END;
CREATE TRIGGER model_version_dependencies_no_delete
BEFORE DELETE ON model_version_dependencies
BEGIN
    SELECT RAISE(ABORT, 'model version dependencies cannot be deleted');
END;
CREATE TRIGGER model_version_artifacts_no_update
BEFORE UPDATE ON model_version_artifacts
BEGIN
    SELECT RAISE(ABORT, 'model version artifacts are immutable');
END;
CREATE TRIGGER model_version_artifacts_no_delete
BEFORE DELETE ON model_version_artifacts
BEGIN
    SELECT RAISE(ABORT, 'model version artifacts cannot be deleted');
END;
CREATE TRIGGER model_version_assumptions_no_update
BEFORE UPDATE ON model_version_assumptions
BEGIN
    SELECT RAISE(ABORT, 'model version assumptions are immutable');
END;
CREATE TRIGGER model_version_assumptions_no_delete
BEFORE DELETE ON model_version_assumptions
BEGIN
    SELECT RAISE(ABORT, 'model version assumptions cannot be deleted');
END;
CREATE TRIGGER model_version_parents_no_update
BEFORE UPDATE ON model_version_parents
BEGIN
    SELECT RAISE(ABORT, 'model version parent links are immutable');
END;
CREATE TRIGGER model_version_parents_no_delete
BEFORE DELETE ON model_version_parents
BEGIN
    SELECT RAISE(ABORT, 'model version parent links cannot be deleted');
END;
CREATE TRIGGER model_version_impacts_no_update
BEFORE UPDATE ON model_version_impacts
BEGIN
    SELECT RAISE(ABORT, 'model version impact records are immutable');
END;
CREATE TRIGGER model_version_impacts_no_delete
BEFORE DELETE ON model_version_impacts
BEGIN
    SELECT RAISE(ABORT, 'model version impact records cannot be deleted');
END;
CREATE TRIGGER model_contributions_no_update
BEFORE UPDATE ON model_contributions
BEGIN
    SELECT RAISE(ABORT, 'model contributions are immutable');
END;
CREATE TRIGGER model_contributions_no_delete
BEFORE DELETE ON model_contributions
BEGIN
    SELECT RAISE(ABORT, 'model contributions cannot be deleted');
END;
CREATE TRIGGER model_evidence_no_update
BEFORE UPDATE ON model_evidence
BEGIN
    SELECT RAISE(ABORT, 'model evidence is immutable');
END;
CREATE TRIGGER model_evidence_no_delete
BEFORE DELETE ON model_evidence
BEGIN
    SELECT RAISE(ABORT, 'model evidence cannot be deleted');
END;
CREATE TRIGGER model_validation_analysis_evidence_no_update
BEFORE UPDATE ON model_validation_analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'validation evidence links are immutable');
END;
CREATE TRIGGER model_validation_analysis_evidence_no_delete
BEFORE DELETE ON model_validation_analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'validation evidence links cannot be deleted');
END;
CREATE TRIGGER model_validation_model_evidence_no_update
BEFORE UPDATE ON model_validation_model_evidence
BEGIN
    SELECT RAISE(ABORT, 'validation evidence links are immutable');
END;
CREATE TRIGGER model_validation_model_evidence_no_delete
BEFORE DELETE ON model_validation_model_evidence
BEGIN
    SELECT RAISE(ABORT, 'validation evidence links cannot be deleted');
END;
CREATE TRIGGER model_replication_datasets_no_update
BEFORE UPDATE ON model_replication_datasets
BEGIN
    SELECT RAISE(ABORT, 'replication dataset pins are immutable');
END;
CREATE TRIGGER model_replication_datasets_no_delete
BEFORE DELETE ON model_replication_datasets
BEGIN
    SELECT RAISE(ABORT, 'replication dataset pins cannot be deleted');
END;
CREATE TRIGGER model_replication_analysis_evidence_no_update
BEFORE UPDATE ON model_replication_analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'replication evidence links are immutable');
END;
CREATE TRIGGER model_replication_analysis_evidence_no_delete
BEFORE DELETE ON model_replication_analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'replication evidence links cannot be deleted');
END;
CREATE TRIGGER model_replication_model_evidence_no_update
BEFORE UPDATE ON model_replication_model_evidence
BEGIN
    SELECT RAISE(ABORT, 'replication evidence links are immutable');
END;
CREATE TRIGGER model_replication_model_evidence_no_delete
BEFORE DELETE ON model_replication_model_evidence
BEGIN
    SELECT RAISE(ABORT, 'replication evidence links cannot be deleted');
END;
CREATE TRIGGER model_evaluations_no_update
BEFORE UPDATE ON model_evaluations
BEGIN
    SELECT RAISE(ABORT, 'model evaluations are immutable');
END;
CREATE TRIGGER model_evaluations_no_delete
BEFORE DELETE ON model_evaluations
BEGIN
    SELECT RAISE(ABORT, 'model evaluations cannot be deleted');
END;
CREATE TRIGGER model_validations_no_update
BEFORE UPDATE ON model_validations
BEGIN
    SELECT RAISE(ABORT, 'validation plans are immutable');
END;
CREATE TRIGGER model_validation_events_no_update
BEFORE UPDATE ON model_validation_events
BEGIN
    SELECT RAISE(ABORT, 'validation outcomes are immutable');
END;
CREATE TRIGGER model_validation_events_no_delete
BEFORE DELETE ON model_validation_events
BEGIN
    SELECT RAISE(ABORT, 'validation outcomes cannot be deleted');
END;
CREATE TRIGGER model_replications_no_update
BEFORE UPDATE ON model_replications
BEGIN
    SELECT RAISE(ABORT, 'replication records are immutable');
END;
CREATE TRIGGER model_replications_no_delete
BEFORE DELETE ON model_replications
BEGIN
    SELECT RAISE(ABORT, 'replication records cannot be deleted');
END;
