CREATE TABLE analysis_schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_methods (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE analysis_method_versions (
    id TEXT PRIMARY KEY,
    method_id TEXT NOT NULL REFERENCES analysis_methods(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) > 0),
    code_uri TEXT NOT NULL CHECK (length(trim(code_uri)) > 0),
    code_revision TEXT NOT NULL CHECK (length(trim(code_revision)) > 0),
    dependency_lock_sha256 TEXT CHECK (
        dependency_lock_sha256 IS NULL OR
        (length(dependency_lock_sha256) = 64 AND dependency_lock_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    environment_json TEXT NOT NULL CHECK (json_valid(environment_json)),
    assumptions_json TEXT NOT NULL CHECK (json_valid(assumptions_json)),
    test_suite_uri TEXT NOT NULL CHECK (length(trim(test_suite_uri)) > 0),
    test_suite_sha256 TEXT NOT NULL CHECK (
        length(test_suite_sha256) = 64 AND test_suite_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    tests_total INTEGER NOT NULL CHECK (tests_total > 0),
    tests_passed INTEGER NOT NULL CHECK (tests_passed >= 0 AND tests_passed <= tests_total),
    verification_status TEXT NOT NULL CHECK (verification_status IN ('passed', 'failed')),
    content_sha256 TEXT NOT NULL CHECK (
        length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (method_id, version_no),
    UNIQUE (id, method_id),
    CHECK (
        (verification_status = 'passed' AND tests_passed = tests_total)
        OR (verification_status = 'failed' AND tests_passed < tests_total)
    )
);
CREATE INDEX idx_analysis_method_versions
    ON analysis_method_versions(method_id, version_no DESC);

CREATE TABLE analysis_metric_definitions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    metric_key TEXT NOT NULL CHECK (length(trim(metric_key)) BETWEEN 1 AND 100),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id),
    UNIQUE (project_id, metric_key)
);

CREATE TABLE analysis_metric_definition_versions (
    id TEXT PRIMARY KEY,
    metric_definition_id TEXT NOT NULL
        REFERENCES analysis_metric_definitions(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) > 0),
    formula TEXT NOT NULL CHECK (length(trim(formula)) > 0),
    unit TEXT NOT NULL CHECK (length(trim(unit)) > 0),
    test_suite_uri TEXT NOT NULL CHECK (length(trim(test_suite_uri)) > 0),
    test_suite_sha256 TEXT NOT NULL CHECK (
        length(test_suite_sha256) = 64 AND test_suite_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    tests_total INTEGER NOT NULL CHECK (tests_total > 0),
    tests_passed INTEGER NOT NULL CHECK (tests_passed >= 0 AND tests_passed <= tests_total),
    verification_status TEXT NOT NULL CHECK (verification_status IN ('passed', 'failed')),
    content_sha256 TEXT NOT NULL CHECK (
        length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (metric_definition_id, version_no),
    UNIQUE (id, metric_definition_id),
    CHECK (
        (verification_status = 'passed' AND tests_passed = tests_total)
        OR (verification_status = 'failed' AND tests_passed < tests_total)
    )
);
CREATE INDEX idx_analysis_metric_versions
    ON analysis_metric_definition_versions(metric_definition_id, version_no DESC);

CREATE TABLE analysis_definitions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE analysis_definition_versions (
    id TEXT PRIMARY KEY,
    definition_id TEXT NOT NULL REFERENCES analysis_definitions(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) > 0),
    parent_version_id TEXT REFERENCES analysis_definition_versions(id) ON DELETE RESTRICT,
    change_rationale TEXT,
    method_version_id TEXT NOT NULL
        REFERENCES analysis_method_versions(id) ON DELETE RESTRICT,
    specification_json TEXT NOT NULL CHECK (json_valid(specification_json)),
    specification_sha256 TEXT NOT NULL CHECK (
        length(specification_sha256) = 64 AND specification_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (definition_id, version_no),
    UNIQUE (id, definition_id),
    CHECK (parent_version_id IS NULL OR parent_version_id <> id),
    CHECK ((parent_version_id IS NULL AND change_rationale IS NULL)
        OR (parent_version_id IS NOT NULL AND length(trim(change_rationale)) > 0))
);
CREATE INDEX idx_analysis_definition_versions
    ON analysis_definition_versions(definition_id, version_no DESC);

CREATE TABLE analysis_definition_metric_versions (
    definition_version_id TEXT NOT NULL
        REFERENCES analysis_definition_versions(id) ON DELETE RESTRICT,
    metric_definition_version_id TEXT NOT NULL
        REFERENCES analysis_metric_definition_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (definition_version_id, metric_definition_version_id)
);

CREATE TABLE analysis_definition_events (
    id TEXT PRIMARY KEY,
    definition_version_id TEXT NOT NULL
        REFERENCES analysis_definition_versions(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('draft', 'review', 'locked', 'retired')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_analysis_definition_events
    ON analysis_definition_events(definition_version_id, occurred_at_us, id);

CREATE TABLE analysis_runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    definition_version_id TEXT NOT NULL
        REFERENCES analysis_definition_versions(id) ON DELETE RESTRICT,
    parameters_json TEXT NOT NULL CHECK (json_valid(parameters_json)),
    environment_json TEXT NOT NULL CHECK (json_valid(environment_json)),
    random_seeds_json TEXT NOT NULL CHECK (json_valid(random_seeds_json)),
    protocol_version TEXT NOT NULL CHECK (length(trim(protocol_version)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);
CREATE INDEX idx_analysis_runs_project
    ON analysis_runs(project_id, created_at_us, id);
CREATE INDEX idx_analysis_runs_definition
    ON analysis_runs(definition_version_id, created_at_us, id);

CREATE TABLE analysis_run_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'queued', 'running', 'completed', 'failed', 'cancelled', 'inconclusive'
    )),
    summary TEXT NOT NULL DEFAULT '',
    failure_summary TEXT NOT NULL DEFAULT '',
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL,
    CHECK (
        lifecycle_status NOT IN ('queued', 'running')
        OR length(failure_summary) = 0
    )
);

CREATE TABLE analysis_run_inputs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    split_group_id TEXT,
    dataset_checksum TEXT NOT NULL CHECK (
        length(dataset_checksum) = 64 AND dataset_checksum NOT GLOB '*[^0-9a-f]*'
    ),
    sample_count INTEGER NOT NULL CHECK (sample_count > 0),
    transformation_lineage_json TEXT NOT NULL CHECK (json_valid(transformation_lineage_json)),
    input_manifest_sha256 TEXT NOT NULL CHECK (
        length(input_manifest_sha256) = 64 AND input_manifest_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    UNIQUE (run_id, dataset_version_id, role),
    UNIQUE (id, dataset_version_id),
    FOREIGN KEY (split_group_id, dataset_version_id)
        REFERENCES experiment_dataset_split_groups(id, dataset_version_id) ON DELETE RESTRICT
);
CREATE INDEX idx_analysis_run_inputs
    ON analysis_run_inputs(run_id, role, dataset_version_id);

CREATE TABLE analysis_run_input_samples (
    input_id TEXT NOT NULL,
    dataset_version_id TEXT NOT NULL,
    sample_id TEXT NOT NULL,
    split_key TEXT,
    feature_available_at_us INTEGER,
    prediction_at_us INTEGER,
    decision_at_us INTEGER,
    outcome_available_at_us INTEGER,
    PRIMARY KEY (input_id, sample_id),
    FOREIGN KEY (input_id, dataset_version_id)
        REFERENCES analysis_run_inputs(id, dataset_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (dataset_version_id, sample_id)
        REFERENCES experiment_dataset_samples(dataset_version_id, sample_id) ON DELETE RESTRICT,
    CHECK (
        (feature_available_at_us IS NULL AND prediction_at_us IS NULL AND decision_at_us IS NULL)
        OR (feature_available_at_us IS NOT NULL AND prediction_at_us IS NOT NULL
            AND decision_at_us IS NOT NULL)
    ),
    CHECK (feature_available_at_us IS NULL OR feature_available_at_us <= prediction_at_us),
    CHECK (prediction_at_us IS NULL OR prediction_at_us <= decision_at_us),
    CHECK (outcome_available_at_us IS NULL OR prediction_at_us IS NULL
        OR outcome_available_at_us >= prediction_at_us)
);

CREATE TABLE analysis_run_integrity_declarations (
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (category IN (
        'overfitting', 'data_leakage', 'selection_bias', 'look_ahead',
        'multiple_testing', 'dataset_separation', 'out_of_sample',
        'walk_forward', 'forward_testing', 'reproducibility'
    )),
    applicability TEXT NOT NULL CHECK (
        applicability IN ('applicable', 'not_applicable', 'not_assessed')
    ),
    rationale TEXT NOT NULL DEFAULT '',
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    PRIMARY KEY (run_id, category),
    CHECK (applicability <> 'not_applicable' OR length(trim(rationale)) > 0)
);

CREATE TABLE analysis_integrity_findings (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (length(trim(category)) > 0),
    code TEXT NOT NULL CHECK (length(trim(code)) > 0),
    severity TEXT NOT NULL CHECK (severity IN ('warning', 'blocking')),
    message TEXT NOT NULL CHECK (length(trim(message)) > 0),
    UNIQUE (run_id, code)
);

CREATE TABLE analysis_run_logs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
    message TEXT NOT NULL CHECK (length(trim(message)) > 0),
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_run_errors (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    error_kind TEXT NOT NULL CHECK (length(trim(error_kind)) > 0),
    message TEXT NOT NULL CHECK (length(trim(message)) > 0),
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_run_deviations (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    description TEXT NOT NULL CHECK (length(trim(description)) > 0),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    impact_assessment TEXT NOT NULL CHECK (length(trim(impact_assessment)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_outputs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    output_kind TEXT NOT NULL CHECK (length(trim(output_kind)) > 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    payload_sha256 TEXT NOT NULL CHECK (
        length(payload_sha256) = 64 AND payload_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    artifact_uri TEXT,
    artifact_sha256 TEXT CHECK (
        artifact_sha256 IS NULL OR
        (length(artifact_sha256) = 64 AND artifact_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    media_type TEXT,
    byte_length INTEGER CHECK (byte_length IS NULL OR byte_length >= 0),
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, run_id),
    CHECK ((artifact_uri IS NULL) = (artifact_sha256 IS NULL))
);

CREATE TABLE analysis_metrics (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    output_id TEXT NOT NULL UNIQUE,
    metric_definition_version_id TEXT NOT NULL
        REFERENCES analysis_metric_definition_versions(id) ON DELETE RESTRICT,
    method_version_id TEXT NOT NULL REFERENCES analysis_method_versions(id) ON DELETE RESTRICT,
    estimand TEXT NOT NULL CHECK (length(trim(estimand)) > 0),
    sample_size INTEGER NOT NULL CHECK (sample_size > 0),
    effect_estimate TEXT NOT NULL CHECK (length(trim(effect_estimate)) > 0),
    unit TEXT NOT NULL CHECK (length(trim(unit)) > 0),
    uncertainty_kind TEXT NOT NULL CHECK (
        uncertainty_kind IN ('interval', 'not_meaningful')
    ),
    uncertainty_lower TEXT,
    uncertainty_upper TEXT,
    uncertainty_level TEXT,
    uncertainty_reason TEXT,
    assumptions_json TEXT NOT NULL CHECK (json_valid(assumptions_json)),
    exclusions_json TEXT NOT NULL CHECK (json_valid(exclusions_json)),
    missing_data_json TEXT NOT NULL CHECK (json_valid(missing_data_json)),
    multiplicity_treatment TEXT NOT NULL CHECK (length(trim(multiplicity_treatment)) > 0),
    p_value TEXT,
    FOREIGN KEY (output_id, run_id) REFERENCES analysis_outputs(id, run_id) ON DELETE RESTRICT,
    CHECK (
        (uncertainty_kind = 'interval' AND uncertainty_lower IS NOT NULL
         AND uncertainty_upper IS NOT NULL AND uncertainty_level IS NOT NULL
         AND uncertainty_reason IS NULL)
        OR
        (uncertainty_kind = 'not_meaningful' AND uncertainty_lower IS NULL
         AND uncertainty_upper IS NULL AND uncertainty_level IS NULL
         AND length(trim(uncertainty_reason)) > 0)
    )
);

CREATE TABLE analysis_evidence (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    target_source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    target_hypothesis_version_id TEXT
        REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    target_finding_version_id TEXT,
    artifact_analysis_output_id TEXT,
    artifact_experiment_run_id TEXT REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    artifact_dataset_version_id TEXT
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    artifact_observation_id TEXT REFERENCES research_observations(id) ON DELETE RESTRICT,
    artifact_source_segment_id TEXT REFERENCES source_version_segments(id) ON DELETE RESTRICT,
    artifact_source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    polarity TEXT NOT NULL CHECK (
        polarity IN ('supports', 'contradicts', 'qualifies', 'neutral')
    ),
    scope TEXT NOT NULL CHECK (length(trim(scope)) > 0),
    quality_rationale TEXT NOT NULL CHECK (length(trim(quality_rationale)) > 0),
    limitations TEXT NOT NULL CHECK (length(trim(limitations)) > 0),
    provenance_json TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (target_finding_version_id)
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    FOREIGN KEY (artifact_analysis_output_id)
        REFERENCES analysis_outputs(id) ON DELETE RESTRICT,
    CHECK (
        (target_source_claim_id IS NOT NULL)
        + (target_hypothesis_version_id IS NOT NULL)
        + (target_finding_version_id IS NOT NULL) = 1
    ),
    CHECK (
        (artifact_analysis_output_id IS NOT NULL)
        + (artifact_experiment_run_id IS NOT NULL)
        + (artifact_dataset_version_id IS NOT NULL)
        + (artifact_observation_id IS NOT NULL)
        + (artifact_source_segment_id IS NOT NULL)
        + (artifact_source_claim_id IS NOT NULL) = 1
    )
);
CREATE INDEX idx_analysis_evidence_project
    ON analysis_evidence(project_id, created_at_us, id);

CREATE TABLE analysis_evidence_events (
    id TEXT PRIMARY KEY,
    evidence_id TEXT NOT NULL REFERENCES analysis_evidence(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN (
        'recorded', 'assessed', 'accepted', 'contested', 'rejected', 'superseded', 'retracted'
    )),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_findings (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE analysis_finding_versions (
    id TEXT PRIMARY KEY,
    finding_id TEXT NOT NULL REFERENCES analysis_findings(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    parent_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    change_rationale TEXT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    target_source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    target_hypothesis_version_id TEXT
        REFERENCES hypothesis_definition_versions(id) ON DELETE RESTRICT,
    scope TEXT NOT NULL CHECK (length(trim(scope)) > 0),
    applicability TEXT NOT NULL CHECK (length(trim(applicability)) > 0),
    uncertainty TEXT NOT NULL CHECK (length(trim(uncertainty)) > 0),
    limitations TEXT NOT NULL CHECK (length(trim(limitations)) > 0),
    conclusion_kind TEXT NOT NULL CHECK (
        conclusion_kind IN ('result', 'null_result', 'contradictory', 'inconclusive')
    ),
    content_sha256 TEXT NOT NULL CHECK (
        length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (finding_id, version_no),
    UNIQUE (id, finding_id),
    CHECK ((target_source_claim_id IS NOT NULL) + (target_hypothesis_version_id IS NOT NULL) = 1),
    CHECK ((parent_version_id IS NULL AND change_rationale IS NULL)
        OR (parent_version_id IS NOT NULL AND length(trim(change_rationale)) > 0))
);
CREATE INDEX idx_analysis_finding_versions
    ON analysis_finding_versions(finding_id, version_no DESC);

CREATE TABLE analysis_finding_version_runs (
    finding_version_id TEXT NOT NULL
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    PRIMARY KEY (finding_version_id, run_id)
);

CREATE TABLE analysis_finding_version_evidence (
    finding_version_id TEXT NOT NULL
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    evidence_id TEXT NOT NULL REFERENCES analysis_evidence(id) ON DELETE RESTRICT,
    synthesis_polarity TEXT NOT NULL CHECK (
        synthesis_polarity IN ('supports', 'contradicts', 'qualifies', 'neutral')
    ),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    PRIMARY KEY (finding_version_id, evidence_id)
);

CREATE TABLE analysis_finding_events (
    id TEXT PRIMARY KEY,
    finding_version_id TEXT NOT NULL
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN (
        'draft', 'review', 'accepted', 'contested', 'unsupported',
        'inconclusive', 'superseded', 'retracted'
    )),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);

CREATE TABLE analysis_finding_relations (
    id TEXT PRIMARY KEY,
    source_version_id TEXT NOT NULL
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    target_version_id TEXT NOT NULL
        REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (
        relation_kind IN ('contests', 'contradicts', 'qualifies', 'supersedes')
    ),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (source_version_id <> target_version_id),
    UNIQUE (source_version_id, target_version_id, relation_kind)
);
