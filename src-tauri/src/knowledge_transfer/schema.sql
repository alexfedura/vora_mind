CREATE TABLE knowledge_concept_versions (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES knowledge_concepts(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    created_in_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    definition TEXT NOT NULL CHECK (length(trim(definition)) > 0),
    parent_version_id TEXT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (concept_id, version_no),
    UNIQUE (id, concept_id),
    UNIQUE (id, concept_id, created_in_project_id),
    FOREIGN KEY (parent_version_id, concept_id)
        REFERENCES knowledge_concept_versions(id, concept_id) ON DELETE RESTRICT,
    CHECK (parent_version_id IS NULL OR parent_version_id <> id)
);

CREATE TABLE knowledge_concept_version_events (
    id TEXT PRIMARY KEY,
    version_id TEXT NOT NULL REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('draft', 'reviewed', 'deprecated')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_concept_version_events
    ON knowledge_concept_version_events(version_id, occurred_at_us DESC, id DESC);

CREATE TABLE knowledge_principle_versions (
    id TEXT PRIMARY KEY,
    principle_id TEXT NOT NULL REFERENCES knowledge_principles(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    created_in_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    parent_version_id TEXT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (principle_id, version_no),
    UNIQUE (id, principle_id),
    UNIQUE (id, principle_id, created_in_project_id),
    FOREIGN KEY (parent_version_id, principle_id)
        REFERENCES knowledge_principle_versions(id, principle_id) ON DELETE RESTRICT,
    CHECK (parent_version_id IS NULL OR parent_version_id <> id)
);

CREATE TABLE knowledge_principle_version_events (
    id TEXT PRIMARY KEY,
    version_id TEXT NOT NULL REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('draft', 'reviewed', 'deprecated')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_principle_version_events
    ON knowledge_principle_version_events(version_id, occurred_at_us DESC, id DESC);

CREATE TABLE knowledge_transfer_origins (
    id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN (
        'finding_version', 'concept_version', 'principle_version',
        'framework_version', 'model_version'
    )),
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    origin_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_kind = 'finding_version' AND finding_version_id IS NOT NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'concept_version' AND finding_version_id IS NULL
            AND concept_version_id IS NOT NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'principle_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NOT NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'framework_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NOT NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'model_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX idx_transfer_origin_finding
    ON knowledge_transfer_origins(finding_version_id) WHERE finding_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_concept
    ON knowledge_transfer_origins(concept_version_id) WHERE concept_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_principle
    ON knowledge_transfer_origins(principle_version_id) WHERE principle_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_framework
    ON knowledge_transfer_origins(framework_version_id) WHERE framework_version_id IS NOT NULL;
CREATE UNIQUE INDEX idx_transfer_origin_model
    ON knowledge_transfer_origins(model_version_id) WHERE model_version_id IS NOT NULL;

CREATE TABLE transfer_eligibilities (
    id TEXT PRIMARY KEY,
    origin_id TEXT NOT NULL REFERENCES knowledge_transfer_origins(id) ON DELETE RESTRICT,
    license_identifier TEXT NOT NULL CHECK (
        length(trim(license_identifier)) > 0 AND lower(trim(license_identifier)) <> 'unknown'
    ),
    license_uri TEXT,
    access_scope TEXT NOT NULL CHECK (access_scope IN ('workspace', 'selected_projects')),
    permitted_purposes_json TEXT NOT NULL CHECK (
        json_valid(permitted_purposes_json)
        AND json_type(permitted_purposes_json) = 'array'
        AND json_array_length(permitted_purposes_json) > 0
    ),
    attribution_text TEXT NOT NULL CHECK (length(trim(attribution_text)) > 0),
    permission_basis TEXT NOT NULL CHECK (length(trim(permission_basis)) > 0),
    permission_granted INTEGER NOT NULL CHECK (permission_granted = 1),
    reviewed_by TEXT NOT NULL CHECK (length(trim(reviewed_by)) > 0),
    reviewed_at_us INTEGER NOT NULL
);
CREATE INDEX idx_transfer_eligibilities_origin
    ON transfer_eligibilities(origin_id, reviewed_at_us DESC, id DESC);

CREATE TABLE transfer_eligibility_projects (
    eligibility_id TEXT NOT NULL
        REFERENCES transfer_eligibilities(id) ON DELETE RESTRICT,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    PRIMARY KEY (eligibility_id, project_id)
);

CREATE TABLE transfer_restrictions (
    id TEXT PRIMARY KEY,
    eligibility_id TEXT NOT NULL
        REFERENCES transfer_eligibilities(id) ON DELETE RESTRICT,
    state TEXT NOT NULL CHECK (state IN ('eligible', 'restricted', 'withdrawn')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_transfer_restrictions_current
    ON transfer_restrictions(eligibility_id, occurred_at_us DESC, id DESC);

CREATE TABLE knowledge_transfers (
    id TEXT PRIMARY KEY,
    origin_id TEXT NOT NULL REFERENCES knowledge_transfer_origins(id) ON DELETE RESTRICT,
    eligibility_id TEXT NOT NULL
        REFERENCES transfer_eligibilities(id) ON DELETE RESTRICT,
    origin_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    receiving_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    intended_purpose TEXT NOT NULL CHECK (intended_purpose IN (
        'research', 'learning', 'commercial', 'public_distribution'
    )),
    requested_by TEXT,
    requested_at_us INTEGER NOT NULL,
    CHECK (origin_project_id <> receiving_project_id),
    UNIQUE (id, origin_id, origin_project_id, receiving_project_id)
);
CREATE INDEX idx_knowledge_transfers_origin
    ON knowledge_transfers(origin_project_id, requested_at_us, id);
CREATE INDEX idx_knowledge_transfers_recipient
    ON knowledge_transfers(receiving_project_id, requested_at_us, id);
CREATE INDEX idx_knowledge_transfers_eligibility
    ON knowledge_transfers(eligibility_id, requested_at_us);

CREATE TABLE transfer_attributions (
    transfer_id TEXT PRIMARY KEY REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    attribution_text TEXT NOT NULL CHECK (length(trim(attribution_text)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE transfer_contexts (
    transfer_id TEXT PRIMARY KEY REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    context TEXT NOT NULL CHECK (length(trim(context)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL
);

CREATE TABLE knowledge_transfer_events (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    state TEXT NOT NULL CHECK (state IN ('accepted', 'rejected', 'withdrawn')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_transfer_events
    ON knowledge_transfer_events(transfer_id, occurred_at_us DESC, id DESC);
CREATE UNIQUE INDEX idx_transfer_single_decision
    ON knowledge_transfer_events(transfer_id) WHERE state IN ('accepted', 'rejected');

CREATE TABLE transfer_adaptations (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    origin_id TEXT NOT NULL REFERENCES knowledge_transfer_origins(id) ON DELETE RESTRICT,
    receiving_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN (
        'finding_version', 'concept_version', 'principle_version',
        'framework_version', 'model_version'
    )),
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    adapted_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (
        (artifact_kind = 'finding_version' AND finding_version_id IS NOT NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'concept_version' AND finding_version_id IS NULL
            AND concept_version_id IS NOT NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'principle_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NOT NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'framework_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NOT NULL AND model_version_id IS NULL)
        OR (artifact_kind = 'model_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NOT NULL)
    )
);
CREATE INDEX idx_transfer_adaptations_transfer
    ON transfer_adaptations(transfer_id, created_at_us, id);

CREATE TABLE research_lineage (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    adaptation_id TEXT REFERENCES transfer_adaptations(id) ON DELETE RESTRICT,
    origin_id TEXT NOT NULL REFERENCES knowledge_transfer_origins(id) ON DELETE RESTRICT,
    origin_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    receiving_project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('adopted', 'based_on', 'derived_from')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL,
    CHECK (
        (relation_kind = 'adopted' AND adaptation_id IS NULL)
        OR (relation_kind IN ('based_on', 'derived_from') AND adaptation_id IS NOT NULL)
    ),
    UNIQUE (transfer_id, adaptation_id, relation_kind)
);
CREATE INDEX idx_research_lineage_origin
    ON research_lineage(origin_project_id, origin_id, occurred_at_us);
CREATE INDEX idx_research_lineage_recipient
    ON research_lineage(receiving_project_id, occurred_at_us);
CREATE UNIQUE INDEX idx_research_lineage_single_adoption
    ON research_lineage(transfer_id, relation_kind)
    WHERE adaptation_id IS NULL;

CREATE TABLE downstream_usage (
    id TEXT PRIMARY KEY,
    transfer_id TEXT NOT NULL REFERENCES knowledge_transfers(id) ON DELETE RESTRICT,
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    usage_kind TEXT NOT NULL CHECK (usage_kind IN (
        'finding_version', 'concept_version', 'principle_version',
        'framework_version', 'model_version', 'model_evaluation',
        'analysis_run', 'experiment_run'
    )),
    finding_version_id TEXT REFERENCES analysis_finding_versions(id) ON DELETE RESTRICT,
    concept_version_id TEXT REFERENCES knowledge_concept_versions(id) ON DELETE RESTRICT,
    principle_version_id TEXT REFERENCES knowledge_principle_versions(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    model_version_id TEXT REFERENCES model_versions(id) ON DELETE RESTRICT,
    model_evaluation_id TEXT REFERENCES model_evaluations(id) ON DELETE RESTRICT,
    analysis_run_id TEXT REFERENCES analysis_runs(id) ON DELETE RESTRICT,
    experiment_run_id TEXT REFERENCES experiment_runs(id) ON DELETE RESTRICT,
    use_context TEXT NOT NULL CHECK (length(trim(use_context)) > 0),
    recorded_by TEXT,
    recorded_at_us INTEGER NOT NULL,
    CHECK (
        (usage_kind = 'finding_version' AND finding_version_id IS NOT NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'concept_version' AND finding_version_id IS NULL
            AND concept_version_id IS NOT NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'principle_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NOT NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'framework_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NOT NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'model_version' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NOT NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'model_evaluation' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NOT NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'analysis_run' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NOT NULL
            AND experiment_run_id IS NULL)
        OR (usage_kind = 'experiment_run' AND finding_version_id IS NULL
            AND concept_version_id IS NULL AND principle_version_id IS NULL
            AND framework_version_id IS NULL AND model_version_id IS NULL
            AND model_evaluation_id IS NULL AND analysis_run_id IS NULL
            AND experiment_run_id IS NOT NULL)
    )
);
CREATE INDEX idx_downstream_usage_transfer ON downstream_usage(transfer_id, recorded_at_us);
CREATE INDEX idx_downstream_usage_project ON downstream_usage(project_id, recorded_at_us);

CREATE TRIGGER transfer_origin_project_match
BEFORE INSERT ON knowledge_transfer_origins
WHEN NOT (
    (NEW.artifact_kind = 'finding_version' AND EXISTS (
        SELECT 1 FROM analysis_finding_versions v
        JOIN analysis_findings f ON f.id = v.finding_id
        WHERE v.id = NEW.finding_version_id AND f.project_id = NEW.origin_project_id
          AND (SELECT e.status FROM analysis_finding_events e
               WHERE e.finding_version_id = v.id
               ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1)
               IN ('accepted', 'contested', 'inconclusive', 'unsupported', 'superseded')
    ))
    OR (NEW.artifact_kind = 'concept_version' AND EXISTS (
        SELECT 1 FROM knowledge_concept_versions v
        JOIN knowledge_concept_version_events e ON e.version_id = v.id
        WHERE v.id = NEW.concept_version_id
          AND v.created_in_project_id = NEW.origin_project_id AND e.status = 'reviewed'
          AND NOT EXISTS (
              SELECT 1 FROM knowledge_concept_version_events later
              WHERE later.version_id = e.version_id
                AND (later.occurred_at_us, later.id) > (e.occurred_at_us, e.id)
          )
    ))
    OR (NEW.artifact_kind = 'principle_version' AND EXISTS (
        SELECT 1 FROM knowledge_principle_versions v
        JOIN knowledge_principle_version_events e ON e.version_id = v.id
        WHERE v.id = NEW.principle_version_id
          AND v.created_in_project_id = NEW.origin_project_id AND e.status = 'reviewed'
          AND NOT EXISTS (
              SELECT 1 FROM knowledge_principle_version_events later
              WHERE later.version_id = e.version_id
                AND (later.occurred_at_us, later.id) > (e.occurred_at_us, e.id)
          )
    ))
    OR (NEW.artifact_kind = 'framework_version' AND EXISTS (
        SELECT 1 FROM knowledge_framework_versions v
        WHERE v.id = NEW.framework_version_id AND v.status = 'published'
    ))
    OR (NEW.artifact_kind = 'model_version' AND EXISTS (
        SELECT 1 FROM model_versions v
        JOIN model_version_events e ON e.model_version_id = v.id
        LEFT JOIN scientific_models m ON m.id = v.model_id
        WHERE v.id = NEW.model_version_id
          AND (v.created_in_project_id = NEW.origin_project_id
               OR m.origin_project_id = NEW.origin_project_id)
          AND e.lifecycle_status = 'released'
          AND NOT EXISTS (
              SELECT 1 FROM model_version_events later
              WHERE later.model_version_id = e.model_version_id
                AND (later.occurred_at_us, later.id) > (e.occurred_at_us, e.id)
          )
    ))
)
BEGIN
    SELECT RAISE(ABORT, 'transfer origin must match an eligible exact artifact and project');
END;

CREATE TRIGGER transfer_eligibility_project_scope
BEFORE INSERT ON transfer_eligibility_projects
WHEN (SELECT access_scope FROM transfer_eligibilities WHERE id = NEW.eligibility_id)
     <> 'selected_projects'
BEGIN
    SELECT RAISE(ABORT, 'recipient allowlists are valid only for selected-project eligibility');
END;

CREATE TRIGGER knowledge_transfer_pin_match
BEFORE INSERT ON knowledge_transfers
WHEN NOT EXISTS (
    SELECT 1 FROM transfer_eligibilities e
    JOIN knowledge_transfer_origins o ON o.id = e.origin_id
    WHERE e.id = NEW.eligibility_id AND o.id = NEW.origin_id
      AND o.origin_project_id = NEW.origin_project_id
      AND (
          SELECT state FROM transfer_restrictions r
          WHERE r.eligibility_id = e.id ORDER BY r.occurred_at_us DESC, r.id DESC LIMIT 1
      ) = 'eligible'
      AND (
          e.access_scope = 'workspace'
          OR EXISTS (
              SELECT 1 FROM transfer_eligibility_projects ep
              WHERE ep.eligibility_id = e.id AND ep.project_id = NEW.receiving_project_id
          )
      )
      AND EXISTS (
          SELECT 1 FROM json_each(e.permitted_purposes_json) p
          WHERE p.value = NEW.intended_purpose
      )
)
BEGIN
    SELECT RAISE(ABORT, 'transfer must pin currently eligible origin, recipient scope, and purpose');
END;

CREATE TRIGGER transfer_event_acceptance_guard
BEFORE INSERT ON knowledge_transfer_events
WHEN NEW.state = 'accepted' AND NOT EXISTS (
    SELECT 1 FROM knowledge_transfers t
    JOIN transfer_eligibilities e ON e.id = t.eligibility_id
    WHERE t.id = NEW.transfer_id
      AND (SELECT state FROM transfer_restrictions r
           WHERE r.eligibility_id = e.id ORDER BY r.occurred_at_us DESC, r.id DESC LIMIT 1)
          = 'eligible'
      AND EXISTS (
          SELECT 1 FROM json_each(e.permitted_purposes_json) p
          WHERE p.value = t.intended_purpose
      )
)
BEGIN
    SELECT RAISE(ABORT, 'transfer acceptance requires current eligibility');
END;

CREATE TRIGGER transfer_adaptation_integrity
BEFORE INSERT ON transfer_adaptations
WHEN NOT EXISTS (
    SELECT 1 FROM knowledge_transfers t
    JOIN knowledge_transfer_origins o ON o.id = t.origin_id
    JOIN knowledge_transfer_events accepted
      ON accepted.transfer_id = t.id AND accepted.state = 'accepted'
    WHERE t.id = NEW.transfer_id AND o.id = NEW.origin_id
      AND t.receiving_project_id = NEW.receiving_project_id
      AND (
        (NEW.artifact_kind = 'finding_version' AND EXISTS (
            SELECT 1 FROM analysis_finding_versions target
            JOIN analysis_findings f ON f.id = target.finding_id
            WHERE target.id = NEW.finding_version_id
              AND f.project_id = NEW.receiving_project_id
              AND target.finding_id <> (
                  SELECT source.finding_id FROM analysis_finding_versions source
                  WHERE source.id = o.finding_version_id
              )
        ))
        OR (NEW.artifact_kind = 'concept_version' AND EXISTS (
            SELECT 1 FROM knowledge_concept_versions target
            WHERE target.id = NEW.concept_version_id
              AND target.created_in_project_id = NEW.receiving_project_id
              AND target.concept_id <> (
                  SELECT source.concept_id FROM knowledge_concept_versions source
                  WHERE source.id = o.concept_version_id
              )
        ))
        OR (NEW.artifact_kind = 'principle_version' AND EXISTS (
            SELECT 1 FROM knowledge_principle_versions target
            WHERE target.id = NEW.principle_version_id
              AND target.created_in_project_id = NEW.receiving_project_id
              AND target.principle_id <> (
                  SELECT source.principle_id FROM knowledge_principle_versions source
                  WHERE source.id = o.principle_version_id
              )
        ))
        OR (NEW.artifact_kind = 'framework_version' AND EXISTS (
            SELECT 1 FROM knowledge_framework_versions target
            WHERE target.id = NEW.framework_version_id
              AND target.framework_id <> (
                  SELECT source.framework_id FROM knowledge_framework_versions source
                  WHERE source.id = o.framework_version_id
              )
              AND target.status = 'published'
        ))
        OR (NEW.artifact_kind = 'model_version' AND EXISTS (
            SELECT 1 FROM model_versions target
            WHERE target.id = NEW.model_version_id
              AND target.created_in_project_id = NEW.receiving_project_id
              AND target.model_id <> (
                  SELECT source.model_id FROM model_versions source
                  WHERE source.id = o.model_version_id
              )
        ))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'adaptation must be a distinct exact artifact version created in the receiving project');
END;

CREATE TRIGGER research_lineage_transfer_match
BEFORE INSERT ON research_lineage
WHEN NOT EXISTS (
    SELECT 1 FROM knowledge_transfers t
    JOIN knowledge_transfer_origins o ON o.id = t.origin_id
    JOIN knowledge_transfer_events e ON e.transfer_id = t.id AND e.state = 'accepted'
    WHERE t.id = NEW.transfer_id AND o.id = NEW.origin_id
      AND o.origin_project_id = NEW.origin_project_id
      AND t.receiving_project_id = NEW.receiving_project_id
      AND (
          (NEW.relation_kind = 'adopted' AND NEW.adaptation_id IS NULL)
          OR EXISTS (
              SELECT 1 FROM transfer_adaptations a
              WHERE a.id = NEW.adaptation_id AND a.transfer_id = t.id
                AND a.origin_id = o.id
          )
      )
)
BEGIN
    SELECT RAISE(ABORT, 'lineage must match an accepted transfer and its exact origin/adaptation');
END;

CREATE TRIGGER downstream_usage_transfer_match
BEFORE INSERT ON downstream_usage
WHEN NOT EXISTS (
    SELECT 1 FROM knowledge_transfers t
    JOIN knowledge_transfer_events e ON e.transfer_id = t.id AND e.state = 'accepted'
    WHERE t.id = NEW.transfer_id AND t.receiving_project_id = NEW.project_id
      AND (
        (NEW.usage_kind = 'finding_version' AND EXISTS (
            SELECT 1 FROM analysis_finding_versions v
            JOIN analysis_findings f ON f.id = v.finding_id
            WHERE v.id = NEW.finding_version_id AND f.project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'concept_version' AND EXISTS (
            SELECT 1 FROM knowledge_concept_versions v
            WHERE v.id = NEW.concept_version_id AND v.created_in_project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'principle_version' AND EXISTS (
            SELECT 1 FROM knowledge_principle_versions v
            WHERE v.id = NEW.principle_version_id AND v.created_in_project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'framework_version' AND EXISTS (
            SELECT 1 FROM knowledge_framework_versions v WHERE v.id = NEW.framework_version_id
        ))
        OR (NEW.usage_kind = 'model_version' AND EXISTS (
            SELECT 1 FROM model_versions v
            WHERE v.id = NEW.model_version_id AND v.created_in_project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'model_evaluation' AND EXISTS (
            SELECT 1 FROM model_evaluations v
            WHERE v.id = NEW.model_evaluation_id AND v.project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'analysis_run' AND EXISTS (
            SELECT 1 FROM analysis_runs v
            WHERE v.id = NEW.analysis_run_id AND v.project_id = NEW.project_id
        ))
        OR (NEW.usage_kind = 'experiment_run' AND EXISTS (
            SELECT 1 FROM experiment_runs v
            WHERE v.id = NEW.experiment_run_id AND v.project_id = NEW.project_id
        ))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'downstream use must belong to the accepted transfer recipient project');
END;

CREATE TRIGGER transfer_concept_versions_no_update
BEFORE UPDATE ON knowledge_concept_versions
BEGIN
    SELECT RAISE(ABORT, 'concept versions are immutable');
END;
CREATE TRIGGER transfer_concept_versions_no_delete
BEFORE DELETE ON knowledge_concept_versions
BEGIN
    SELECT RAISE(ABORT, 'concept versions cannot be deleted');
END;
CREATE TRIGGER transfer_concept_version_events_no_update
BEFORE UPDATE ON knowledge_concept_version_events
BEGIN
    SELECT RAISE(ABORT, 'concept version events are immutable');
END;
CREATE TRIGGER transfer_concept_version_events_no_delete
BEFORE DELETE ON knowledge_concept_version_events
BEGIN
    SELECT RAISE(ABORT, 'concept version events cannot be deleted');
END;
CREATE TRIGGER transfer_principle_versions_no_update
BEFORE UPDATE ON knowledge_principle_versions
BEGIN
    SELECT RAISE(ABORT, 'principle versions are immutable');
END;
CREATE TRIGGER transfer_principle_versions_no_delete
BEFORE DELETE ON knowledge_principle_versions
BEGIN
    SELECT RAISE(ABORT, 'principle versions cannot be deleted');
END;
CREATE TRIGGER transfer_principle_version_events_no_update
BEFORE UPDATE ON knowledge_principle_version_events
BEGIN
    SELECT RAISE(ABORT, 'principle version events are immutable');
END;
CREATE TRIGGER transfer_principle_version_events_no_delete
BEFORE DELETE ON knowledge_principle_version_events
BEGIN
    SELECT RAISE(ABORT, 'principle version events cannot be deleted');
END;

CREATE TRIGGER transfer_origins_no_update
BEFORE UPDATE ON knowledge_transfer_origins
BEGIN
    SELECT RAISE(ABORT, 'transfer origin assertions are immutable');
END;
CREATE TRIGGER transfer_origins_no_delete
BEFORE DELETE ON knowledge_transfer_origins
BEGIN
    SELECT RAISE(ABORT, 'transfer origin assertions cannot be deleted');
END;
CREATE TRIGGER transfer_eligibilities_no_update
BEFORE UPDATE ON transfer_eligibilities
BEGIN
    SELECT RAISE(ABORT, 'transfer eligibility snapshots are immutable');
END;
CREATE TRIGGER transfer_eligibilities_no_delete
BEFORE DELETE ON transfer_eligibilities
BEGIN
    SELECT RAISE(ABORT, 'transfer eligibility snapshots cannot be deleted');
END;
CREATE TRIGGER transfer_eligibility_projects_no_update
BEFORE UPDATE ON transfer_eligibility_projects
BEGIN
    SELECT RAISE(ABORT, 'eligibility recipient allowlists are immutable');
END;
CREATE TRIGGER transfer_eligibility_projects_no_delete
BEFORE DELETE ON transfer_eligibility_projects
BEGIN
    SELECT RAISE(ABORT, 'eligibility recipient allowlists cannot be deleted');
END;
CREATE TRIGGER transfer_restrictions_no_update
BEFORE UPDATE ON transfer_restrictions
BEGIN
    SELECT RAISE(ABORT, 'transfer restriction history is immutable');
END;
CREATE TRIGGER transfer_restrictions_no_delete
BEFORE DELETE ON transfer_restrictions
BEGIN
    SELECT RAISE(ABORT, 'transfer restriction history cannot be deleted');
END;
CREATE TRIGGER knowledge_transfers_no_update
BEFORE UPDATE ON knowledge_transfers
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfers are immutable');
END;
CREATE TRIGGER knowledge_transfers_no_delete
BEFORE DELETE ON knowledge_transfers
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfers cannot be deleted');
END;
CREATE TRIGGER transfer_attributions_no_update
BEFORE UPDATE ON transfer_attributions
BEGIN
    SELECT RAISE(ABORT, 'transfer attributions are immutable');
END;
CREATE TRIGGER transfer_attributions_no_delete
BEFORE DELETE ON transfer_attributions
BEGIN
    SELECT RAISE(ABORT, 'transfer attributions cannot be deleted');
END;
CREATE TRIGGER transfer_contexts_no_update
BEFORE UPDATE ON transfer_contexts
BEGIN
    SELECT RAISE(ABORT, 'transfer contexts are immutable');
END;
CREATE TRIGGER transfer_contexts_no_delete
BEFORE DELETE ON transfer_contexts
BEGIN
    SELECT RAISE(ABORT, 'transfer contexts cannot be deleted');
END;
CREATE TRIGGER knowledge_transfer_events_no_update
BEFORE UPDATE ON knowledge_transfer_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer events are immutable');
END;
CREATE TRIGGER knowledge_transfer_events_no_delete
BEFORE DELETE ON knowledge_transfer_events
BEGIN
    SELECT RAISE(ABORT, 'knowledge transfer events cannot be deleted');
END;
CREATE TRIGGER transfer_adaptations_no_update
BEFORE UPDATE ON transfer_adaptations
BEGIN
    SELECT RAISE(ABORT, 'transfer adaptations are immutable');
END;
CREATE TRIGGER transfer_adaptations_no_delete
BEFORE DELETE ON transfer_adaptations
BEGIN
    SELECT RAISE(ABORT, 'transfer adaptations cannot be deleted');
END;
CREATE TRIGGER research_lineage_no_update
BEFORE UPDATE ON research_lineage
BEGIN
    SELECT RAISE(ABORT, 'research lineage is immutable');
END;
CREATE TRIGGER research_lineage_no_delete
BEFORE DELETE ON research_lineage
BEGIN
    SELECT RAISE(ABORT, 'research lineage cannot be deleted');
END;
CREATE TRIGGER downstream_usage_no_update
BEFORE UPDATE ON downstream_usage
BEGIN
    SELECT RAISE(ABORT, 'downstream usage is immutable');
END;
CREATE TRIGGER downstream_usage_no_delete
BEFORE DELETE ON downstream_usage
BEGIN
    SELECT RAISE(ABORT, 'downstream usage cannot be deleted');
END;
