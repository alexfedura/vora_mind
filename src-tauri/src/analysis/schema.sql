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
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL,
    CHECK (
        (lifecycle_status IN ('queued', 'running') AND length(failure_summary) = 0)
        OR lifecycle_status IN ('completed', 'failed', 'cancelled', 'inconclusive')
    )
);
