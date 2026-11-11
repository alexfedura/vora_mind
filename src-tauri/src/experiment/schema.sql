CREATE TABLE IF NOT EXISTS experiment_schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at_us INTEGER NOT NULL
);

CREATE TABLE experiment_design_version_specs (
    design_version_id TEXT PRIMARY KEY
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    format_version INTEGER NOT NULL CHECK (format_version = 1),
    specification_json TEXT NOT NULL CHECK (json_valid(specification_json)),
    specification_sha256 TEXT NOT NULL CHECK (
        length(specification_sha256) = 64 AND
        specification_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_at_us INTEGER NOT NULL
);

CREATE TABLE experiment_design_version_titles (
    design_version_id TEXT PRIMARY KEY
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200)
);

CREATE TABLE experiment_design_version_lineage (
    design_version_id TEXT PRIMARY KEY,
    design_id TEXT NOT NULL,
    parent_version_id TEXT NOT NULL,
    change_rationale TEXT NOT NULL CHECK (length(trim(change_rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (design_version_id, design_id)
        REFERENCES research_experiment_design_versions(id, design_id) ON DELETE RESTRICT,
    FOREIGN KEY (parent_version_id, design_id)
        REFERENCES research_experiment_design_versions(id, design_id) ON DELETE RESTRICT,
    CHECK (design_version_id <> parent_version_id)
);

CREATE TABLE experiment_preregistrations (
    id TEXT PRIMARY KEY,
    design_version_id TEXT NOT NULL UNIQUE
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    specification_sha256 TEXT NOT NULL CHECK (
        length(specification_sha256) = 64 AND
        specification_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    reviewer_id TEXT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    preregistered_at_us INTEGER NOT NULL
);

CREATE TABLE experiment_datasets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, project_id)
);

CREATE TABLE experiment_dataset_versions (
    id TEXT PRIMARY KEY,
    dataset_id TEXT NOT NULL REFERENCES experiment_datasets(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    schema_id TEXT NOT NULL CHECK (length(trim(schema_id)) > 0),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    sample_schema_json TEXT NOT NULL CHECK (json_valid(sample_schema_json)),
    expected_sample_count INTEGER NOT NULL CHECK (expected_sample_count >= 0),
    staged_revision INTEGER NOT NULL DEFAULT 0 CHECK (staged_revision >= 0),
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN ('staged', 'sealed')),
    content_sha256 TEXT CHECK (
        content_sha256 IS NULL OR
        (length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    sealed_at_us INTEGER,
    UNIQUE (dataset_id, version_no),
    UNIQUE (id, dataset_id),
    CHECK (
        (lifecycle_status = 'staged' AND content_sha256 IS NULL AND sealed_at_us IS NULL)
        OR
        (lifecycle_status = 'sealed' AND content_sha256 IS NOT NULL AND sealed_at_us IS NOT NULL)
    )
);

CREATE TABLE experiment_dataset_assets (
    id TEXT PRIMARY KEY,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    storage_key TEXT NOT NULL UNIQUE CHECK (length(trim(storage_key)) > 0),
    original_name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    byte_length INTEGER NOT NULL CHECK (byte_length >= 0),
    content_sha256 TEXT NOT NULL CHECK (
        length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, dataset_version_id)
);

CREATE TABLE experiment_dataset_samples (
    id TEXT PRIMARY KEY,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    sample_id TEXT NOT NULL CHECK (length(trim(sample_id)) > 0),
    sample_kind TEXT NOT NULL CHECK (sample_kind IN ('data', 'trade')),
    occurred_at_us INTEGER,
    record_json TEXT NOT NULL CHECK (json_valid(record_json)),
    created_at_us INTEGER NOT NULL,
    UNIQUE (dataset_version_id, sample_id),
    UNIQUE (id, dataset_version_id)
);
CREATE INDEX idx_experiment_dataset_samples_page
    ON experiment_dataset_samples(dataset_version_id, sample_id);
CREATE INDEX idx_experiment_dataset_samples_time
    ON experiment_dataset_samples(dataset_version_id, occurred_at_us, sample_id);

CREATE TABLE experiment_dataset_transformations (
    id TEXT PRIMARY KEY,
    output_dataset_version_id TEXT NOT NULL UNIQUE
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    method TEXT NOT NULL CHECK (length(trim(method)) > 0),
    parameters_json TEXT NOT NULL CHECK (json_valid(parameters_json)),
    provenance_json TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_by TEXT,
    created_at_us INTEGER NOT NULL
);

CREATE TABLE experiment_dataset_transformation_inputs (
    transformation_id TEXT NOT NULL
        REFERENCES experiment_dataset_transformations(id) ON DELETE RESTRICT,
    input_dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    input_role TEXT NOT NULL CHECK (input_role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    PRIMARY KEY (transformation_id, input_dataset_version_id),
    CHECK (length(trim(input_role)) > 0)
);

CREATE TABLE experiment_dataset_split_groups (
    id TEXT PRIMARY KEY,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    split_group_key TEXT NOT NULL CHECK (length(trim(split_group_key)) > 0),
    require_disjoint INTEGER NOT NULL CHECK (require_disjoint IN (0, 1)),
    created_at_us INTEGER NOT NULL,
    UNIQUE (dataset_version_id, split_group_key),
    UNIQUE (id, dataset_version_id)
);

CREATE TABLE experiment_sample_split_assignments (
    id TEXT PRIMARY KEY,
    split_group_id TEXT NOT NULL,
    dataset_version_id TEXT NOT NULL,
    sample_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    split_key TEXT NOT NULL CHECK (length(trim(split_key)) > 0),
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (split_group_id, dataset_version_id)
        REFERENCES experiment_dataset_split_groups(id, dataset_version_id) ON DELETE RESTRICT,
    FOREIGN KEY (dataset_version_id, sample_id)
        REFERENCES experiment_dataset_samples(dataset_version_id, sample_id) ON DELETE RESTRICT,
    UNIQUE (split_group_id, sample_id, role, split_key)
);
CREATE INDEX idx_experiment_split_assignments_role
    ON experiment_sample_split_assignments(split_group_id, role, split_key);

CREATE TABLE experiment_design_dataset_pins (
    design_version_id TEXT NOT NULL
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    rationale TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (design_version_id, dataset_version_id, role)
);

CREATE TABLE experiment_runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    design_version_id TEXT NOT NULL
        REFERENCES research_experiment_design_versions(id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN (
        'created', 'ready', 'running', 'completed', 'failed', 'cancelled', 'inconclusive'
    )),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    started_at_us INTEGER,
    ended_at_us INTEGER,
    completion_summary TEXT,
    failure_details TEXT,
    CHECK (
        lifecycle_status <> 'running'
        OR (started_at_us IS NOT NULL AND ended_at_us IS NULL)
    ),
    CHECK (
        lifecycle_status NOT IN ('completed', 'failed', 'cancelled', 'inconclusive')
        OR ended_at_us IS NOT NULL
    )
);
CREATE INDEX idx_experiment_runs_design
    ON experiment_runs(design_version_id, created_at_us, id);

CREATE TABLE experiment_run_dataset_pins (
    run_id TEXT NOT NULL REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    dataset_version_id TEXT NOT NULL
        REFERENCES experiment_dataset_versions(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN (
        'training', 'validation', 'test', 'exploratory', 'prospective', 'forward'
    )),
    usage_kind TEXT NOT NULL CHECK (usage_kind IN ('input', 'output')),
    PRIMARY KEY (run_id, dataset_version_id, role, usage_kind)
);

CREATE TABLE experiment_run_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT,
    actor_id TEXT,
    details TEXT NOT NULL DEFAULT '',
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_experiment_run_events
    ON experiment_run_events(run_id, occurred_at_us, id);

CREATE TABLE experiment_run_deviations (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    description TEXT NOT NULL CHECK (length(trim(description)) > 0),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    impact_assessment TEXT NOT NULL DEFAULT '',
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE experiment_run_logs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    severity TEXT NOT NULL CHECK (severity IN ('debug', 'info', 'warning', 'error')),
    message TEXT NOT NULL,
    details_json TEXT NOT NULL CHECK (json_valid(details_json)),
    recorded_at_us INTEGER NOT NULL
);
CREATE INDEX idx_experiment_run_logs
    ON experiment_run_logs(run_id, recorded_at_us, id);

CREATE TRIGGER experiment_design_spec_no_update
BEFORE UPDATE ON experiment_design_version_specs
BEGIN
    SELECT RAISE(ABORT, 'experiment design specifications are immutable; create a new design version');
END;
CREATE TRIGGER experiment_design_spec_no_delete
BEFORE DELETE ON experiment_design_version_specs
BEGIN
    SELECT RAISE(ABORT, 'experiment design specifications are immutable');
END;
CREATE TRIGGER experiment_design_title_no_update
BEFORE UPDATE ON experiment_design_version_titles
BEGIN
    SELECT RAISE(ABORT, 'experiment design version titles are immutable');
END;
CREATE TRIGGER experiment_design_title_no_delete
BEFORE DELETE ON experiment_design_version_titles
BEGIN
    SELECT RAISE(ABORT, 'experiment design version titles are immutable');
END;
CREATE TRIGGER experiment_design_version_typed_immutable
BEFORE UPDATE ON research_experiment_design_versions
WHEN EXISTS (
    SELECT 1 FROM experiment_design_version_specs s
    WHERE s.design_version_id = OLD.id
)
 AND NOT (
    OLD.lifecycle_status IN ('draft', 'review')
    AND NEW.lifecycle_status = 'preregistered'
    AND OLD.id = NEW.id
    AND OLD.design_id = NEW.design_id
    AND OLD.version_no = NEW.version_no
    AND OLD.description = NEW.description
    AND OLD.protocol = NEW.protocol
    AND OLD.created_by IS NEW.created_by
    AND OLD.created_at_us = NEW.created_at_us
    AND NEW.preregistered_at_us IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'typed experiment design versions are immutable except for preregistration');
END;
CREATE TRIGGER experiment_design_lineage_no_update
BEFORE UPDATE ON experiment_design_version_lineage
BEGIN
    SELECT RAISE(ABORT, 'experiment design lineage is append-only');
END;
CREATE TRIGGER experiment_design_lineage_no_delete
BEFORE DELETE ON experiment_design_version_lineage
BEGIN
    SELECT RAISE(ABORT, 'experiment design lineage is append-only');
END;
CREATE TRIGGER experiment_preregistrations_no_update
BEFORE UPDATE ON experiment_preregistrations
BEGIN
    SELECT RAISE(ABORT, 'preregistration records are immutable');
END;
CREATE TRIGGER experiment_preregistrations_no_delete
BEFORE DELETE ON experiment_preregistrations
BEGIN
    SELECT RAISE(ABORT, 'preregistration records are immutable');
END;
CREATE TRIGGER experiment_dataset_version_sealed_no_update
BEFORE UPDATE ON experiment_dataset_versions
WHEN OLD.lifecycle_status = 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'sealed dataset versions are immutable');
END;
CREATE TRIGGER experiment_dataset_version_sealed_no_delete
BEFORE DELETE ON experiment_dataset_versions
WHEN OLD.lifecycle_status = 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'sealed dataset versions are immutable');
END;
CREATE TRIGGER experiment_dataset_version_identity_immutable
BEFORE UPDATE OF id, dataset_id, version_no, schema_id, schema_version,
                 sample_schema_json, expected_sample_count, created_by, created_at_us
ON experiment_dataset_versions
BEGIN
    SELECT RAISE(ABORT, 'dataset version identity and schema are immutable');
END;
CREATE TRIGGER experiment_dataset_asset_staged_insert
BEFORE INSERT ON experiment_dataset_assets
WHEN (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'staged'
BEGIN
    SELECT RAISE(ABORT, 'assets can only be added to staged dataset versions');
END;
CREATE TRIGGER experiment_dataset_asset_immutable
BEFORE UPDATE ON experiment_dataset_assets
BEGIN
    SELECT RAISE(ABORT, 'dataset assets are immutable');
END;
CREATE TRIGGER experiment_dataset_asset_delete_guard
BEFORE DELETE ON experiment_dataset_assets
BEGIN
    SELECT RAISE(ABORT, 'dataset assets are immutable');
END;
CREATE TRIGGER experiment_dataset_asset_revision
AFTER INSERT ON experiment_dataset_assets
BEGIN
    UPDATE experiment_dataset_versions SET staged_revision = staged_revision + 1
    WHERE id = NEW.dataset_version_id AND lifecycle_status = 'staged';
END;
CREATE TRIGGER experiment_dataset_sample_staged_insert
BEFORE INSERT ON experiment_dataset_samples
WHEN (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'staged'
BEGIN
    SELECT RAISE(ABORT, 'samples can only be added to staged dataset versions');
END;
CREATE TRIGGER experiment_dataset_sample_no_update
BEFORE UPDATE ON experiment_dataset_samples
BEGIN
    SELECT RAISE(ABORT, 'dataset samples are immutable');
END;
CREATE TRIGGER experiment_dataset_sample_no_delete
BEFORE DELETE ON experiment_dataset_samples
BEGIN
    SELECT RAISE(ABORT, 'dataset samples are immutable');
END;
CREATE TRIGGER experiment_dataset_sample_revision
AFTER INSERT ON experiment_dataset_samples
BEGIN
    UPDATE experiment_dataset_versions SET staged_revision = staged_revision + 1
    WHERE id = NEW.dataset_version_id AND lifecycle_status = 'staged';
END;
CREATE TRIGGER experiment_split_group_staged_insert
BEFORE INSERT ON experiment_dataset_split_groups
WHEN (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'staged'
BEGIN
    SELECT RAISE(ABORT, 'splits can only be added to staged dataset versions');
END;
CREATE TRIGGER experiment_split_assignment_staged_insert
BEFORE INSERT ON experiment_sample_split_assignments
WHEN (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'staged'
BEGIN
    SELECT RAISE(ABORT, 'split assignments can only be added to staged dataset versions');
END;
CREATE TRIGGER experiment_split_assignment_disjoint_guard
BEFORE INSERT ON experiment_sample_split_assignments
WHEN (SELECT require_disjoint FROM experiment_dataset_split_groups
      WHERE id = NEW.split_group_id) = 1
 AND EXISTS (
    SELECT 1 FROM experiment_sample_split_assignments existing
    WHERE existing.split_group_id = NEW.split_group_id
      AND existing.sample_id = NEW.sample_id
 )
BEGIN
    SELECT RAISE(ABORT, 'sample split assignments must be disjoint for this split group');
END;
CREATE TRIGGER experiment_split_assignment_no_update
BEFORE UPDATE ON experiment_sample_split_assignments
BEGIN
    SELECT RAISE(ABORT, 'split assignments are immutable');
END;
CREATE TRIGGER experiment_split_assignment_no_delete
BEFORE DELETE ON experiment_sample_split_assignments
BEGIN
    SELECT RAISE(ABORT, 'split assignments are immutable');
END;
CREATE TRIGGER experiment_split_group_no_update
BEFORE UPDATE ON experiment_dataset_split_groups
BEGIN
    SELECT RAISE(ABORT, 'dataset split groups are immutable');
END;
CREATE TRIGGER experiment_split_group_no_delete
BEFORE DELETE ON experiment_dataset_split_groups
BEGIN
    SELECT RAISE(ABORT, 'dataset split groups are immutable');
END;
CREATE TRIGGER experiment_split_group_revision
AFTER INSERT ON experiment_dataset_split_groups
BEGIN
    UPDATE experiment_dataset_versions SET staged_revision = staged_revision + 1
    WHERE id = NEW.dataset_version_id AND lifecycle_status = 'staged';
END;
CREATE TRIGGER experiment_split_assignment_revision
AFTER INSERT ON experiment_sample_split_assignments
BEGIN
    UPDATE experiment_dataset_versions SET staged_revision = staged_revision + 1
    WHERE id = NEW.dataset_version_id AND lifecycle_status = 'staged';
END;
CREATE TRIGGER experiment_design_dataset_pin_draft_insert
BEFORE INSERT ON experiment_design_dataset_pins
WHEN (SELECT lifecycle_status FROM research_experiment_design_versions WHERE id = NEW.design_version_id) <> 'draft'
 OR (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'design pins require a draft design and sealed dataset version');
END;
CREATE TRIGGER experiment_design_dataset_pin_no_update
BEFORE UPDATE ON experiment_design_dataset_pins
BEGIN
    SELECT RAISE(ABORT, 'design dataset pins are immutable');
END;
CREATE TRIGGER experiment_design_dataset_pin_no_delete
BEFORE DELETE ON experiment_design_dataset_pins
BEGIN
    SELECT RAISE(ABORT, 'design dataset pins are immutable');
END;
CREATE TRIGGER experiment_run_dataset_pin_insert
BEFORE INSERT ON experiment_run_dataset_pins
WHEN
    (NEW.usage_kind = 'input' AND
     (SELECT lifecycle_status FROM experiment_runs WHERE id = NEW.run_id) <> 'created')
    OR
    (NEW.usage_kind = 'output' AND
     (SELECT lifecycle_status FROM experiment_runs WHERE id = NEW.run_id) <> 'running')
    OR
    (SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = NEW.dataset_version_id) <> 'sealed'
BEGIN
    SELECT RAISE(ABORT, 'run dataset pins require sealed versions and valid run lifecycle state');
END;
CREATE TRIGGER experiment_run_dataset_pin_no_update
BEFORE UPDATE ON experiment_run_dataset_pins
BEGIN
    SELECT RAISE(ABORT, 'run dataset pins are immutable');
END;
CREATE TRIGGER experiment_run_dataset_pin_no_delete
BEFORE DELETE ON experiment_run_dataset_pins
BEGIN
    SELECT RAISE(ABORT, 'run dataset pins are immutable');
END;
CREATE TRIGGER experiment_run_transition
BEFORE UPDATE OF lifecycle_status ON experiment_runs
WHEN NOT (
    (OLD.lifecycle_status = 'created' AND NEW.lifecycle_status IN ('ready', 'cancelled'))
    OR (OLD.lifecycle_status = 'ready' AND NEW.lifecycle_status IN ('running', 'cancelled'))
    OR (OLD.lifecycle_status = 'running' AND NEW.lifecycle_status IN (
        'completed', 'failed', 'cancelled', 'inconclusive'
    ))
)
BEGIN
    SELECT RAISE(ABORT, 'invalid experiment run lifecycle transition');
END;
CREATE TRIGGER experiment_transformation_no_update
BEFORE UPDATE ON experiment_dataset_transformations
BEGIN
    SELECT RAISE(ABORT, 'dataset transformations are immutable');
END;
CREATE TRIGGER experiment_transformation_no_delete
BEFORE DELETE ON experiment_dataset_transformations
BEGIN
    SELECT RAISE(ABORT, 'dataset transformations are immutable');
END;
CREATE TRIGGER experiment_transformation_input_no_update
BEFORE UPDATE ON experiment_dataset_transformation_inputs
BEGIN
    SELECT RAISE(ABORT, 'dataset transformation inputs are immutable');
END;
CREATE TRIGGER experiment_transformation_input_no_delete
BEFORE DELETE ON experiment_dataset_transformation_inputs
BEGIN
    SELECT RAISE(ABORT, 'dataset transformation inputs are immutable');
END;
CREATE TRIGGER experiment_run_no_delete
BEFORE DELETE ON experiment_runs
BEGIN
    SELECT RAISE(ABORT, 'experiment runs cannot be deleted or reused');
END;
CREATE TRIGGER experiment_run_identity_immutable
BEFORE UPDATE OF id, project_id, design_version_id, created_by, created_at_us ON experiment_runs
BEGIN
    SELECT RAISE(ABORT, 'experiment run identity and design pin are immutable');
END;
CREATE TRIGGER experiment_run_event_no_update
BEFORE UPDATE ON experiment_run_events
BEGIN
    SELECT RAISE(ABORT, 'experiment run events are append-only');
END;
CREATE TRIGGER experiment_run_event_no_delete
BEFORE DELETE ON experiment_run_events
BEGIN
    SELECT RAISE(ABORT, 'experiment run events are append-only');
END;
CREATE TRIGGER experiment_run_deviation_no_update
BEFORE UPDATE ON experiment_run_deviations
BEGIN
    SELECT RAISE(ABORT, 'experiment run deviations are immutable');
END;
CREATE TRIGGER experiment_run_deviation_no_delete
BEFORE DELETE ON experiment_run_deviations
BEGIN
    SELECT RAISE(ABORT, 'experiment run deviations are immutable');
END;
CREATE TRIGGER experiment_run_log_no_update
BEFORE UPDATE ON experiment_run_logs
BEGIN
    SELECT RAISE(ABORT, 'experiment run logs are append-only');
END;
CREATE TRIGGER experiment_run_log_no_delete
BEFORE DELETE ON experiment_run_logs
BEGIN
    SELECT RAISE(ABORT, 'experiment run logs are append-only');
END;
