CREATE TABLE knowledge_concept_versions (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES knowledge_concepts(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    created_in_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    definition TEXT NOT NULL CHECK (length(trim(definition)) > 0),
    parent_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    reviewed_by TEXT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (concept_id, version_no),
    UNIQUE (id, concept_id, created_in_project_id),
    CHECK (parent_version_id IS NULL OR parent_version_id <> id)
);

CREATE TABLE knowledge_principle_versions (
    id TEXT PRIMARY KEY,
    principle_id TEXT NOT NULL REFERENCES knowledge_principles(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    created_in_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    parent_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    reviewed_by TEXT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (principle_id, version_no),
    UNIQUE (id, principle_id, created_in_project_id),
    CHECK (parent_version_id IS NULL OR parent_version_id <> id)
);

CREATE TABLE knowledge_transfer_origins (
    id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN (
        'concept_version', 'principle_version', 'framework_version',
        'finding_version', 'model_version'
    )),
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    origin_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_kind = 'concept_version' AND concept_version_id IS NOT NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'principle_version' AND concept_version_id IS NULL
            AND principle_version_id IS NOT NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'framework_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NOT NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'finding_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NOT NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'model_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX idx_transfer_origin_concept
    ON knowledge_transfer_origins(concept_version_id) WHERE concept_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_principle
    ON knowledge_transfer_origins(principle_version_id) WHERE principle_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_framework
    ON knowledge_transfer_origins(framework_version_id) WHERE framework_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_finding
    ON knowledge_transfer_origins(finding_version_id) WHERE finding_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_model
    ON knowledge_transfer_origins(model_version_id) WHERE model_version_id IS NOT NULL;

CREATE TABLE knowledge_transfer_eligibility (
    id TEXT PRIMARY KEY,
    origin_id TEXT NOT NULL REFERENCES knowledge_transfer_origins(id) ON DELETE RESTRICT,
    license_identifier TEXT NOT NULL CHECK (
        length(trim(license_identifier)) > 0 AND lower(trim(license_identifier)) <> 'unknown'
    ),
    license_uri TEXT,
    access_scope TEXT NOT NULL CHECK (access_scope IN ('workspace', 'selected_projects')),
    allowed_purposes_json TEXT NOT NULL CHECK (
        json_valid(allowed_purposes_json)
        AND json_type(allowed_purposes_json) = 'array'
        AND json_array_length(allowed_purposes_json) > 0
    ),
    attribution_text TEXT NOT NULL CHECK (length(trim(attribution_text)) > 0),
    permission_basis TEXT NOT NULL CHECK (length(trim(permission_basis)) > 0),
    permission_granted INTEGER NOT NULL CHECK (permission_granted = 1),
    reviewed_by TEXT,
    reviewed_at_us INTEGER NOT NULL
);
CREATE INDEX idx_transfer_eligibility_origin
    ON knowledge_transfer_eligibility(origin_id, reviewed_at_us DESC);

CREATE TABLE knowledge_transfer_eligibility_projects (
    eligibility_id TEXT NOT NULL
        REFERENCES knowledge_transfer_eligibility(id) ON DELETE RESTRICT,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    PRIMARY KEY (eligibility_id, project_id)
);

CREATE TABLE knowledge_transfer_eligibility_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    eligibility_id TEXT NOT NULL
        REFERENCES knowledge_transfer_eligibility(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('eligible', 'restricted', 'withdrawn')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_transfer_eligibility_events
    ON knowledge_transfer_eligibility_events(eligibility_id, sequence DESC);

CREATE TABLE knowledge_transfers (
    id TEXT PRIMARY KEY,
    eligibility_id TEXT NOT NULL
        REFERENCES knowledge_transfer_eligibility(id) ON DELETE RESTRICT,
    origin_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    receiving_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    intended_purpose TEXT NOT NULL CHECK (intended_purpose IN (
        'research', 'learning', 'commercial', 'public_distribution'
    )),
    use_context TEXT NOT NULL CHECK (length(trim(use_context)) > 0),
    attribution_text TEXT NOT NULL CHECK (length(trim(attribution_text)) > 0),
    requested_by TEXT,
    requested_at_us INTEGER NOT NULL,
    reasserts_transfer_id TEXT REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    CHECK (origin_project_id <> receiving_project_id)
);
CREATE INDEX idx_knowledge_transfers_origin
    ON knowledge_transfers(origin_project_id, requested_at_us, id);
CREATE INDEX idx_knowledge_transfers_recipient
    ON knowledge_transfers(receiving_project_id, requested_at_us, id);

CREATE TABLE knowledge_transfer_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('proposed', 'accepted', 'rejected', 'withdrawn')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_transfer_events
    ON knowledge_transfer_events(transfer_id, sequence DESC);
CREATE UNIQUE INDEX idx_transfer_single_acceptance
    ON knowledge_transfer_events(transfer_id) WHERE status = 'accepted';

CREATE TABLE knowledge_transfer_adaptations (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    receiving_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN (
        'concept_version', 'principle_version', 'framework_version',
        'finding_version', 'model_version'
    )),
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    adapted_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_kind = 'concept_version' AND concept_version_id IS NOT NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'principle_version' AND concept_version_id IS NULL
            AND principle_version_id IS NOT NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'framework_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NOT NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'finding_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NOT NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'model_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NOT NULL)
    )
);
CREATE INDEX idx_transfer_adaptations_transfer
    ON knowledge_transfer_adaptations(transfer_id, created_at_us, id);

CREATE TABLE knowledge_transfer_downstream_usage (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    usage_kind TEXT NOT NULL CHECK (usage_kind IN (
        'concept_version', 'principle_version', 'framework_version',
        'finding_version', 'model_version', 'model_evaluation',
        'analysis_run', 'experiment_run'
    )),
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    model_evaluation_id TEXT REFERENCES model_evaluations(id) ON DELETE RESTRICT,
    analysis_run_id TEXT REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    experiment_run_id TEXT REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    use_context TEXT NOT NULL CHECK (length(trim(use_context)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL,
    CHECK (
        (usage_kind = 'concept_version' AND concept_version_id IS NOT NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'principle_version' AND concept_version_id IS NULL
            AND principle_version_id IS NOT NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'framework_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NOT NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'finding_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NOT NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'model_version' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NOT NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'model_evaluation' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NOT NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'analysis_run' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NOT NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'experiment_run' AND concept_version_id IS NULL
            AND principle_version_id IS NULL AND framework_version_id IS NULL
            AND finding_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NOT NULL)
    )
);
CREATE INDEX idx_transfer_downstream_usage
    ON knowledge_transfer_downstream_usage(transfer_id, recorded_at_us, id);

CREATE TRIGGER knowledge_concept_versions_no_update
BEFORE UPDATE ON knowledge_concept_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge concept versions are immutable');
END;
CREATE TRIGGER knowledge_concept_versions_no_delete
BEFORE DELETE ON knowledge_concept_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge concept versions are immutable');
END;
CREATE TRIGGER knowledge_principle_versions_no_update
BEFORE UPDATE ON knowledge_principle_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge principle versions are immutable');
END;
CREATE TRIGGER knowledge_principle_versions_no_delete
BEFORE DELETE ON knowledge_principle_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge principle versions are immutable');
END;
CREATE TRIGGER knowledge_transfer_origins_no_update
BEFORE UPDATE ON knowledge_transfer_origins
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer origins are immutable');
END;
CREATE TRIGGER knowledge_transfer_origins_no_delete
BEFORE DELETE ON knowledge_transfer_origins
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer origins are immutable');
END;
CREATE TRIGGER knowledge_transfer_eligibility_no_update
BEFORE UPDATE ON knowledge_transfer_eligibility
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer eligibility records are immutable');
END;
CREATE TRIGGER knowledge_transfer_eligibility_no_delete
BEFORE DELETE ON knowledge_transfer_eligibility
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer eligibility records are immutable');
END;
CREATE TRIGGER knowledge_transfer_events_no_update
BEFORE UPDATE ON knowledge_transfer_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer events are append-only');
END;
CREATE TRIGGER knowledge_transfer_events_no_delete
BEFORE DELETE ON knowledge_transfer_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer events are append-only');
END;
CREATE TRIGGER knowledge_transfer_eligibility_events_no_update
BEFORE UPDATE ON knowledge_transfer_eligibility_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer eligibility events are append-only');
END;
CREATE TRIGGER knowledge_transfer_eligibility_events_no_delete
BEFORE DELETE ON knowledge_transfer_eligibility_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer eligibility events are append-only');
END;
CREATE TRIGGER knowledge_transfer_adaptations_no_update
BEFORE UPDATE ON knowledge_transfer_adaptations
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer adaptations are immutable');
END;
CREATE TRIGGER knowledge_transfer_adaptations_no_delete
BEFORE DELETE ON knowledge_transfer_adaptations
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer adaptations are immutable');
END;
CREATE TRIGGER knowledge_transfer_downstream_usage_no_update
BEFORE UPDATE ON knowledge_transfer_downstream_usage
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer usage is immutable');
END;
CREATE TRIGGER knowledge_transfer_downstream_usage_no_delete
BEFORE DELETE ON knowledge_transfer_downstream_usage
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer usage is immutable');
END;

CREATE TRIGGER knowledge_transfers_validate_origin
BEFORE INSERT ON knowledge_transfers
WHEN NEW.origin_project_id <> (
    SELECT o.origin_project_id
    FROM knowledge_transfer_eligibility e
    JOIN knowledge_transfer_origins o ON o.id = e.origin_id
    WHERE e.id = NEW.eligibility_id
)
BEGIN
    SELECT RAISE(ABORT, 'transfer origin does not match the pinned eligibility record');
END;
CREATE TRIGGER knowledge_transfer_adaptations_validate
BEFORE INSERT ON knowledge_transfer_adaptations
WHEN NOT EXISTS (
    SELECT 1
    FROM knowledge_transfers t
    JOIN knowledge_transfer_events e ON e.transfer_id = t.id
    JOIN knowledge_transfer_eligibility eligibility ON eligibility.id = t.eligibility_id
    JOIN knowledge_transfer_origins origin ON origin.id = eligibility.origin_id
    WHERE t.id = NEW.transfer_id
      AND t.receiving_project_id = NEW.receiving_project_id
      AND e.status = 'accepted'
      AND e.sequence = (
          SELECT MAX(latest.sequence) FROM knowledge_transfer_events latest
          WHERE latest.transfer_id = t.id
      )
      AND origin.artifact_kind = NEW.artifact_kind
      AND (
          (NEW.artifact_kind = 'concept_version'
              AND origin.concept_version_id <> NEW.concept_version_id
              AND EXISTS (SELECT 1 FROM knowledge_concept_versions v
                  WHERE v.id = NEW.concept_version_id
                    AND v.created_in_project_id = NEW.receiving_project_id))
          OR (NEW.artifact_kind = 'principle_version'
              AND origin.principle_version_id <> NEW.principle_version_id
              AND EXISTS (SELECT 1 FROM knowledge_principle_versions v
                  WHERE v.id = NEW.principle_version_id
                    AND v.created_in_project_id = NEW.receiving_project_id))
          OR (NEW.artifact_kind = 'framework_version'
              AND origin.framework_version_id <> NEW.framework_version_id
              AND EXISTS (SELECT 1 FROM knowledge_transfer_origins target
                  WHERE target.framework_version_id = NEW.framework_version_id
                    AND target.origin_project_id = NEW.receiving_project_id))
          OR (NEW.artifact_kind = 'finding_version'
              AND origin.finding_version_id <> NEW.finding_version_id
              AND EXISTS (SELECT 1 FROM analysis_finding_versions v
                  JOIN analysis_findings f ON f.id = v.finding_id
                  WHERE v.id = NEW.finding_version_id
                    AND f.project_id = NEW.receiving_project_id))
          OR (NEW.artifact_kind = 'model_version'
              AND origin.model_version_id <> NEW.model_version_id
              AND EXISTS (SELECT 1 FROM model_versions v
                  WHERE v.id = NEW.model_version_id
                    AND v.created_in_project_id = NEW.receiving_project_id))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'adaptation must be a distinct accepted artifact version in the receiving project');
END;
