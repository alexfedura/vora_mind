CREATE TABLE IF NOT EXISTS analysis_schema_migrations (
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
    output_key TEXT NOT NULL CHECK (length(trim(output_key)) > 0),
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
    UNIQUE (run_id, output_key),
    CHECK ((artifact_uri IS NULL) = (artifact_sha256 IS NULL))
);

CREATE TABLE analysis_metrics (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    output_id TEXT NOT NULL,
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

CREATE TRIGGER analysis_methods_immutable_update
BEFORE UPDATE ON analysis_methods
BEGIN
    SELECT RAISE(ABORT, 'analysis method identities are immutable');
END;
CREATE TRIGGER analysis_methods_immutable_delete
BEFORE DELETE ON analysis_methods
BEGIN
    SELECT RAISE(ABORT, 'analysis method identities cannot be deleted');
END;
CREATE TRIGGER analysis_method_versions_immutable_update
BEFORE UPDATE ON analysis_method_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis method versions are immutable');
END;
CREATE TRIGGER analysis_method_versions_immutable_delete
BEFORE DELETE ON analysis_method_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis method versions cannot be deleted');
END;
CREATE TRIGGER analysis_metric_definitions_immutable_update
BEFORE UPDATE ON analysis_metric_definitions
BEGIN
    SELECT RAISE(ABORT, 'metric definition identities are immutable');
END;
CREATE TRIGGER analysis_metric_definitions_immutable_delete
BEFORE DELETE ON analysis_metric_definitions
BEGIN
    SELECT RAISE(ABORT, 'metric definition identities cannot be deleted');
END;
CREATE TRIGGER analysis_metric_versions_immutable_update
BEFORE UPDATE ON analysis_metric_definition_versions
BEGIN
    SELECT RAISE(ABORT, 'metric definition versions are immutable');
END;
CREATE TRIGGER analysis_metric_versions_immutable_delete
BEFORE DELETE ON analysis_metric_definition_versions
BEGIN
    SELECT RAISE(ABORT, 'metric definition versions cannot be deleted');
END;
CREATE TRIGGER analysis_definitions_immutable_update
BEFORE UPDATE ON analysis_definitions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition identities are immutable');
END;
CREATE TRIGGER analysis_definitions_immutable_delete
BEFORE DELETE ON analysis_definitions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition identities cannot be deleted');
END;
CREATE TRIGGER analysis_definition_versions_immutable_update
BEFORE UPDATE ON analysis_definition_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition versions are immutable');
END;
CREATE TRIGGER analysis_definition_versions_immutable_delete
BEFORE DELETE ON analysis_definition_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition versions cannot be deleted');
END;
CREATE TRIGGER analysis_definition_metric_versions_immutable_update
BEFORE UPDATE ON analysis_definition_metric_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition metric pins are immutable');
END;
CREATE TRIGGER analysis_definition_metric_versions_immutable_delete
BEFORE DELETE ON analysis_definition_metric_versions
BEGIN
    SELECT RAISE(ABORT, 'analysis definition metric pins are immutable');
END;
CREATE TRIGGER analysis_definition_events_immutable_update
BEFORE UPDATE ON analysis_definition_events
BEGIN
    SELECT RAISE(ABORT, 'analysis definition events are append-only');
END;
CREATE TRIGGER analysis_definition_events_immutable_delete
BEFORE DELETE ON analysis_definition_events
BEGIN
    SELECT RAISE(ABORT, 'analysis definition events are append-only');
END;
CREATE TRIGGER analysis_definition_events_transition
BEFORE INSERT ON analysis_definition_events
WHEN (
    (SELECT COUNT(*) FROM analysis_definition_events
     WHERE definition_version_id = NEW.definition_version_id) = 0
    AND NEW.status <> 'draft'
)
OR (
    (SELECT status FROM analysis_definition_events
     WHERE definition_version_id = NEW.definition_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'draft'
    AND NEW.status NOT IN ('review', 'retired')
)
OR (
    (SELECT status FROM analysis_definition_events
     WHERE definition_version_id = NEW.definition_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'review'
    AND NEW.status NOT IN ('locked', 'retired')
)
OR (
    (SELECT status FROM analysis_definition_events
     WHERE definition_version_id = NEW.definition_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) IN ('locked', 'retired')
)
BEGIN
    SELECT RAISE(ABORT, 'invalid analysis definition lifecycle transition');
END;
CREATE TRIGGER analysis_definition_lock_requires_verified_versions
BEFORE INSERT ON analysis_definition_events
WHEN NEW.status = 'locked'
 AND (
    (SELECT mv.verification_status
     FROM analysis_definition_versions dv
     JOIN analysis_method_versions mv ON mv.id = dv.method_version_id
     WHERE dv.id = NEW.definition_version_id) <> 'passed'
    OR EXISTS (
        SELECT 1
        FROM analysis_definition_metric_versions dm
        JOIN analysis_metric_definition_versions mv
          ON mv.id = dm.metric_definition_version_id
        WHERE dm.definition_version_id = NEW.definition_version_id
          AND mv.verification_status <> 'passed'
    )
    OR NOT EXISTS (
        SELECT 1 FROM analysis_definition_metric_versions
        WHERE definition_version_id = NEW.definition_version_id
    )
 )
BEGIN
    SELECT RAISE(ABORT, 'locking requires verified method and metric versions');
END;
CREATE TRIGGER analysis_runs_immutable_update
BEFORE UPDATE ON analysis_runs
BEGIN
    SELECT RAISE(ABORT, 'analysis runs are immutable');
END;
CREATE TRIGGER analysis_runs_immutable_delete
BEFORE DELETE ON analysis_runs
BEGIN
    SELECT RAISE(ABORT, 'analysis runs cannot be deleted');
END;
CREATE TRIGGER analysis_run_events_transition
BEFORE INSERT ON analysis_run_events
WHEN (
    (SELECT COUNT(*) FROM analysis_run_events WHERE run_id = NEW.run_id) = 0
    AND NEW.lifecycle_status <> 'queued'
)
OR (
    (SELECT lifecycle_status FROM analysis_run_events
     WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'queued'
    AND NEW.lifecycle_status NOT IN ('running', 'failed', 'cancelled')
)
OR (
    (SELECT lifecycle_status FROM analysis_run_events
     WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'running'
    AND NEW.lifecycle_status NOT IN ('completed', 'failed', 'cancelled', 'inconclusive')
)
OR (
    (SELECT lifecycle_status FROM analysis_run_events
     WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
      IN ('completed', 'failed', 'cancelled', 'inconclusive')
)
OR (
    NEW.lifecycle_status = 'failed'
    AND NOT EXISTS (SELECT 1 FROM analysis_run_errors WHERE run_id = NEW.run_id)
)
OR (
    NEW.lifecycle_status = 'completed'
    AND NOT EXISTS (SELECT 1 FROM analysis_outputs WHERE run_id = NEW.run_id)
)
BEGIN
    SELECT RAISE(ABORT, 'invalid analysis run lifecycle transition or missing terminal record');
END;
CREATE TRIGGER analysis_run_events_immutable_update
BEFORE UPDATE ON analysis_run_events
BEGIN
    SELECT RAISE(ABORT, 'analysis run events are append-only');
END;
CREATE TRIGGER analysis_run_events_immutable_delete
BEFORE DELETE ON analysis_run_events
BEGIN
    SELECT RAISE(ABORT, 'analysis run events are append-only');
END;

CREATE TRIGGER analysis_run_inputs_immutable_update
BEFORE UPDATE ON analysis_run_inputs
BEGIN
    SELECT RAISE(ABORT, 'analysis run inputs are immutable');
END;
CREATE TRIGGER analysis_run_inputs_immutable_delete
BEFORE DELETE ON analysis_run_inputs
BEGIN
    SELECT RAISE(ABORT, 'analysis run inputs are immutable');
END;
CREATE TRIGGER analysis_run_input_samples_immutable_update
BEFORE UPDATE ON analysis_run_input_samples
BEGIN
    SELECT RAISE(ABORT, 'analysis run sample assignments are immutable');
END;
CREATE TRIGGER analysis_run_input_samples_immutable_delete
BEFORE DELETE ON analysis_run_input_samples
BEGIN
    SELECT RAISE(ABORT, 'analysis run sample assignments are immutable');
END;
CREATE TRIGGER analysis_run_integrity_immutable_update
BEFORE UPDATE ON analysis_run_integrity_declarations
BEGIN
    SELECT RAISE(ABORT, 'analysis integrity declarations are immutable');
END;
CREATE TRIGGER analysis_run_integrity_immutable_delete
BEFORE DELETE ON analysis_run_integrity_declarations
BEGIN
    SELECT RAISE(ABORT, 'analysis integrity declarations are immutable');
END;
CREATE TRIGGER analysis_integrity_findings_immutable_update
BEFORE UPDATE ON analysis_integrity_findings
BEGIN
    SELECT RAISE(ABORT, 'analysis integrity findings are immutable');
END;
CREATE TRIGGER analysis_integrity_findings_immutable_delete
BEFORE DELETE ON analysis_integrity_findings
BEGIN
    SELECT RAISE(ABORT, 'analysis integrity findings are immutable');
END;
CREATE TRIGGER analysis_run_logs_immutable_update
BEFORE UPDATE ON analysis_run_logs
BEGIN
    SELECT RAISE(ABORT, 'analysis logs are append-only');
END;
CREATE TRIGGER analysis_run_logs_immutable_delete
BEFORE DELETE ON analysis_run_logs
BEGIN
    SELECT RAISE(ABORT, 'analysis logs are append-only');
END;
CREATE TRIGGER analysis_run_errors_immutable_update
BEFORE UPDATE ON analysis_run_errors
BEGIN
    SELECT RAISE(ABORT, 'analysis errors are append-only');
END;
CREATE TRIGGER analysis_run_errors_immutable_delete
BEFORE DELETE ON analysis_run_errors
BEGIN
    SELECT RAISE(ABORT, 'analysis errors are append-only');
END;
CREATE TRIGGER analysis_run_deviations_immutable_update
BEFORE UPDATE ON analysis_run_deviations
BEGIN
    SELECT RAISE(ABORT, 'analysis deviations are append-only');
END;
CREATE TRIGGER analysis_run_deviations_immutable_delete
BEFORE DELETE ON analysis_run_deviations
BEGIN
    SELECT RAISE(ABORT, 'analysis deviations are append-only');
END;
CREATE TRIGGER analysis_outputs_immutable_update
BEFORE UPDATE ON analysis_outputs
BEGIN
    SELECT RAISE(ABORT, 'analysis outputs are immutable');
END;
CREATE TRIGGER analysis_outputs_immutable_delete
BEFORE DELETE ON analysis_outputs
BEGIN
    SELECT RAISE(ABORT, 'analysis outputs are immutable');
END;
CREATE TRIGGER analysis_metrics_immutable_update
BEFORE UPDATE ON analysis_metrics
BEGIN
    SELECT RAISE(ABORT, 'analysis metrics are immutable');
END;
CREATE TRIGGER analysis_metrics_immutable_delete
BEFORE DELETE ON analysis_metrics
BEGIN
    SELECT RAISE(ABORT, 'analysis metrics are immutable');
END;
CREATE TRIGGER analysis_evidence_immutable_update
BEFORE UPDATE ON analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'evidence is immutable; append an event or create a new record');
END;
CREATE TRIGGER analysis_evidence_immutable_delete
BEFORE DELETE ON analysis_evidence
BEGIN
    SELECT RAISE(ABORT, 'evidence cannot be deleted');
END;
CREATE TRIGGER analysis_evidence_events_transition
BEFORE INSERT ON analysis_evidence_events
WHEN (
    (SELECT COUNT(*) FROM analysis_evidence_events WHERE evidence_id = NEW.evidence_id) = 0
    AND NEW.status <> 'recorded'
)
OR (
    (SELECT status FROM analysis_evidence_events
     WHERE evidence_id = NEW.evidence_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'recorded'
    AND NEW.status NOT IN ('assessed', 'contested', 'rejected', 'retracted')
)
OR (
    (SELECT status FROM analysis_evidence_events
     WHERE evidence_id = NEW.evidence_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'assessed'
    AND NEW.status NOT IN ('accepted', 'contested', 'rejected', 'retracted')
)
OR (
    (SELECT status FROM analysis_evidence_events
     WHERE evidence_id = NEW.evidence_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'accepted'
    AND NEW.status NOT IN ('contested', 'superseded', 'retracted')
)
OR (
    (SELECT status FROM analysis_evidence_events
     WHERE evidence_id = NEW.evidence_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'contested'
    AND NEW.status NOT IN ('accepted', 'rejected', 'superseded', 'retracted')
)
OR (
    (SELECT status FROM analysis_evidence_events
     WHERE evidence_id = NEW.evidence_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
      IN ('rejected', 'superseded', 'retracted')
)
BEGIN
    SELECT RAISE(ABORT, 'invalid evidence lifecycle transition');
END;
CREATE TRIGGER analysis_evidence_events_immutable_update
BEFORE UPDATE ON analysis_evidence_events
BEGIN
    SELECT RAISE(ABORT, 'evidence events are append-only');
END;
CREATE TRIGGER analysis_evidence_events_immutable_delete
BEFORE DELETE ON analysis_evidence_events
BEGIN
    SELECT RAISE(ABORT, 'evidence events are append-only');
END;
CREATE TRIGGER analysis_findings_immutable_update
BEFORE UPDATE ON analysis_findings
BEGIN
    SELECT RAISE(ABORT, 'finding identities are immutable');
END;
CREATE TRIGGER analysis_findings_immutable_delete
BEFORE DELETE ON analysis_findings
BEGIN
    SELECT RAISE(ABORT, 'finding identities cannot be deleted');
END;
CREATE TRIGGER analysis_finding_versions_immutable_update
BEFORE UPDATE ON analysis_finding_versions
BEGIN
    SELECT RAISE(ABORT, 'finding versions are immutable');
END;
CREATE TRIGGER analysis_finding_versions_immutable_delete
BEFORE DELETE ON analysis_finding_versions
BEGIN
    SELECT RAISE(ABORT, 'finding versions cannot be deleted');
END;
CREATE TRIGGER analysis_finding_version_runs_immutable_update
BEFORE UPDATE ON analysis_finding_version_runs
BEGIN
    SELECT RAISE(ABORT, 'finding analysis lineage is immutable');
END;
CREATE TRIGGER analysis_finding_version_runs_immutable_delete
BEFORE DELETE ON analysis_finding_version_runs
BEGIN
    SELECT RAISE(ABORT, 'finding analysis lineage is immutable');
END;
CREATE TRIGGER analysis_finding_version_evidence_immutable_update
BEFORE UPDATE ON analysis_finding_version_evidence
BEGIN
    SELECT RAISE(ABORT, 'finding evidence links are immutable');
END;
CREATE TRIGGER analysis_finding_version_evidence_immutable_delete
BEFORE DELETE ON analysis_finding_version_evidence
BEGIN
    SELECT RAISE(ABORT, 'finding evidence links are immutable');
END;
CREATE TRIGGER analysis_finding_events_transition
BEFORE INSERT ON analysis_finding_events
WHEN (
    (SELECT COUNT(*) FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id) = 0
    AND NEW.status <> 'draft'
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'draft'
    AND NEW.status NOT IN ('review', 'retracted')
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'review'
    AND NEW.status NOT IN ('accepted', 'contested', 'unsupported', 'inconclusive', 'retracted')
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'accepted'
    AND NEW.status NOT IN ('contested', 'superseded', 'retracted')
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) = 'contested'
    AND NEW.status NOT IN ('accepted', 'unsupported', 'inconclusive', 'superseded', 'retracted')
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) IN ('unsupported', 'inconclusive')
    AND NEW.status NOT IN ('contested', 'superseded', 'retracted')
)
OR (
    (SELECT status FROM analysis_finding_events
     WHERE finding_version_id = NEW.finding_version_id
     ORDER BY occurred_at_us DESC, id DESC LIMIT 1) IN ('superseded', 'retracted')
)
BEGIN
    SELECT RAISE(ABORT, 'invalid finding lifecycle transition');
END;
CREATE TRIGGER analysis_finding_events_immutable_update
BEFORE UPDATE ON analysis_finding_events
BEGIN
    SELECT RAISE(ABORT, 'finding events are append-only');
END;
CREATE TRIGGER analysis_finding_events_immutable_delete
BEFORE DELETE ON analysis_finding_events
BEGIN
    SELECT RAISE(ABORT, 'finding events are append-only');
END;
CREATE TRIGGER analysis_finding_relations_immutable_update
BEFORE UPDATE ON analysis_finding_relations
BEGIN
    SELECT RAISE(ABORT, 'finding relations are immutable');
END;
CREATE TRIGGER analysis_finding_relations_immutable_delete
BEFORE DELETE ON analysis_finding_relations
BEGIN
    SELECT RAISE(ABORT, 'finding relations are immutable');
END;

CREATE TRIGGER analysis_run_logs_only_while_open
BEFORE INSERT ON analysis_run_logs
WHEN (SELECT lifecycle_status FROM analysis_run_events
      WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
     NOT IN ('queued', 'running')
BEGIN
    SELECT RAISE(ABORT, 'analysis logs may only be added to queued or running analyses');
END;
CREATE TRIGGER analysis_run_errors_only_while_open
BEFORE INSERT ON analysis_run_errors
WHEN (SELECT lifecycle_status FROM analysis_run_events
      WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
     NOT IN ('queued', 'running')
BEGIN
    SELECT RAISE(ABORT, 'analysis errors may only be added to queued or running analyses');
END;
CREATE TRIGGER analysis_run_deviations_only_while_open
BEFORE INSERT ON analysis_run_deviations
WHEN (SELECT lifecycle_status FROM analysis_run_events
      WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
     NOT IN ('queued', 'running')
BEGIN
    SELECT RAISE(ABORT, 'analysis deviations may only be added to queued or running analyses');
END;
CREATE TRIGGER analysis_outputs_only_while_running
BEFORE INSERT ON analysis_outputs
WHEN (SELECT lifecycle_status FROM analysis_run_events
      WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) <> 'running'
BEGIN
    SELECT RAISE(ABORT, 'analysis outputs may only be finalized while running');
END;
CREATE TRIGGER analysis_metrics_only_while_running
BEFORE INSERT ON analysis_metrics
WHEN (SELECT lifecycle_status FROM analysis_run_events
      WHERE run_id = NEW.run_id ORDER BY occurred_at_us DESC, id DESC LIMIT 1) <> 'running'
BEGIN
    SELECT RAISE(ABORT, 'analysis metrics may only be finalized while running');
END;
