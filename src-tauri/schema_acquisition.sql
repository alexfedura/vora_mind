CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at_us INTEGER NOT NULL,
    app_version TEXT NOT NULL
);

CREATE TABLE source_versions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
    capture_kind TEXT NOT NULL CHECK (capture_kind IN ('edition', 'retrieval')),
    edition_label TEXT,
    canonical_uri TEXT,
    captured_at_us INTEGER NOT NULL,
    content_sha256 TEXT CHECK (
        content_sha256 IS NULL OR
        (length(content_sha256) = 64 AND content_sha256 NOT GLOB '*[^0-9a-f]*')
    ),
    external_asset_ref TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_by TEXT NOT NULL,
    created_at_us INTEGER NOT NULL,
    UNIQUE (id, source_id)
);
CREATE INDEX idx_source_versions_source
    ON source_versions(source_id, captured_at_us DESC, id);

CREATE TABLE source_version_segments (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
    source_version_id TEXT NOT NULL,
    legacy_segment_id TEXT REFERENCES segments(id) ON DELETE RESTRICT,
    label TEXT NOT NULL,
    location_kind TEXT NOT NULL CHECK (location_kind IN (
        'page', 'paragraph', 'line', 'timestamp', 'time_range', 'text_anchor', 'unknown'
    )),
    page_start INTEGER,
    page_end INTEGER,
    paragraph_start INTEGER,
    paragraph_end INTEGER,
    line_start INTEGER,
    line_end INTEGER,
    time_start_ms INTEGER,
    time_end_ms INTEGER,
    stable_selector TEXT,
    locator_text TEXT,
    notes TEXT NOT NULL DEFAULT '',
    parent_segment_id TEXT REFERENCES source_version_segments(id) ON DELETE RESTRICT,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (source_version_id, source_id)
        REFERENCES source_versions(id, source_id) ON DELETE RESTRICT,
    UNIQUE (id, source_id, source_version_id),
    CHECK (page_start IS NULL OR page_start > 0),
    CHECK (page_end IS NULL OR page_end >= page_start),
    CHECK (paragraph_start IS NULL OR paragraph_start > 0),
    CHECK (paragraph_end IS NULL OR paragraph_end >= paragraph_start),
    CHECK (line_start IS NULL OR line_start > 0),
    CHECK (line_end IS NULL OR line_end >= line_start),
    CHECK (time_start_ms IS NULL OR time_start_ms >= 0),
    CHECK (time_end_ms IS NULL OR time_end_ms >= time_start_ms)
);
CREATE INDEX idx_source_version_segments_version
    ON source_version_segments(source_version_id, order_index, id);
CREATE INDEX idx_source_version_segments_legacy
    ON source_version_segments(legacy_segment_id);

CREATE TABLE knowledge_extractions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
    source_version_id TEXT NOT NULL,
    source_segment_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN (
        'quote', 'paraphrase', 'summary', 'learner_interpretation', 'source_claim'
    )),
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    language TEXT,
    parent_extraction_id TEXT REFERENCES knowledge_extractions(id) ON DELETE RESTRICT,
    tool_identity TEXT,
    tool_version TEXT,
    prompt_config_identity TEXT,
    confidence_decimal TEXT,
    uncertainty TEXT,
    review_status TEXT NOT NULL DEFAULT 'unreviewed'
        CHECK (review_status IN ('unreviewed', 'accepted', 'rejected', 'needs_correction')),
    created_by TEXT NOT NULL,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (source_segment_id, source_id, source_version_id)
        REFERENCES source_version_segments(id, source_id, source_version_id) ON DELETE RESTRICT,
    UNIQUE (id, source_id, source_version_id),
    CHECK (
        confidence_decimal IS NULL OR
        (length(confidence_decimal) > 0 AND confidence_decimal NOT GLOB '*[^0-9.\\-]*')
    )
);
CREATE INDEX idx_knowledge_extractions_location
    ON knowledge_extractions(source_id, source_version_id, source_segment_id, created_at_us);
CREATE INDEX idx_knowledge_extractions_parent
    ON knowledge_extractions(parent_extraction_id);
CREATE INDEX idx_knowledge_extractions_review
    ON knowledge_extractions(review_status, created_at_us);

CREATE TABLE extraction_reviews (
    id TEXT PRIMARY KEY,
    extraction_id TEXT NOT NULL REFERENCES knowledge_extractions(id) ON DELETE RESTRICT,
    review_status TEXT NOT NULL CHECK (
        review_status IN ('accepted', 'rejected', 'needs_correction')
    ),
    reviewer_id TEXT,
    reviewed_at_us INTEGER NOT NULL,
    rationale TEXT NOT NULL DEFAULT '',
    UNIQUE (id, extraction_id)
);
CREATE INDEX idx_extraction_reviews_extraction
    ON extraction_reviews(extraction_id, reviewed_at_us, id);

CREATE TRIGGER extraction_reviews_apply_state
AFTER INSERT ON extraction_reviews
BEGIN
    UPDATE knowledge_extractions
    SET review_status = NEW.review_status
    WHERE id = NEW.extraction_id;
END;
CREATE TRIGGER extraction_reviews_no_update
BEFORE UPDATE ON extraction_reviews
BEGIN
    SELECT RAISE(ABORT, 'extraction_reviews is append-only');
END;
CREATE TRIGGER extraction_reviews_no_delete
BEFORE DELETE ON extraction_reviews
BEGIN
    SELECT RAISE(ABORT, 'extraction_reviews is append-only');
END;

CREATE TABLE extraction_supersessions (
    id TEXT PRIMARY KEY,
    prior_extraction_id TEXT NOT NULL UNIQUE
        REFERENCES knowledge_extractions(id) ON DELETE RESTRICT,
    replacement_extraction_id TEXT NOT NULL UNIQUE
        REFERENCES knowledge_extractions(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    actor_id TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (prior_extraction_id <> replacement_extraction_id)
);
CREATE TRIGGER extraction_supersessions_no_update
BEFORE UPDATE ON extraction_supersessions
BEGIN
    SELECT RAISE(ABORT, 'extraction_supersessions is append-only');
END;
CREATE TRIGGER extraction_supersessions_no_delete
BEFORE DELETE ON extraction_supersessions
BEGIN
    SELECT RAISE(ABORT, 'extraction_supersessions is append-only');
END;

CREATE TABLE source_claims (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
    source_version_id TEXT NOT NULL,
    extraction_id TEXT NOT NULL UNIQUE,
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    asserted_by TEXT,
    scope TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (source_version_id, source_id)
        REFERENCES source_versions(id, source_id) ON DELETE RESTRICT,
    FOREIGN KEY (extraction_id, source_id, source_version_id)
        REFERENCES knowledge_extractions(id, source_id, source_version_id) ON DELETE RESTRICT
);
CREATE INDEX idx_source_claims_source
    ON source_claims(source_id, source_version_id, created_at_us);

CREATE TRIGGER source_claims_require_reviewed_extraction
BEFORE INSERT ON source_claims
WHEN NOT EXISTS (
    SELECT 1 FROM knowledge_extractions e
    WHERE e.id = NEW.extraction_id
      AND e.source_id = NEW.source_id
      AND e.source_version_id = NEW.source_version_id
      AND e.kind = 'source_claim'
      AND e.review_status = 'accepted'
)
BEGIN
    SELECT RAISE(ABORT, 'source claim requires an accepted source-claim extraction');
END;

CREATE TABLE acquisition_provenance_events (
    id TEXT PRIMARY KEY,
    source_id TEXT REFERENCES sources(id) ON DELETE RESTRICT,
    source_version_id TEXT REFERENCES source_versions(id) ON DELETE RESTRICT,
    source_segment_id TEXT REFERENCES source_version_segments(id) ON DELETE RESTRICT,
    extraction_id TEXT REFERENCES knowledge_extractions(id) ON DELETE RESTRICT,
    source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    concept_id TEXT REFERENCES knowledge_concepts(id) ON DELETE RESTRICT,
    principle_id TEXT REFERENCES knowledge_principles(id) ON DELETE RESTRICT,
    procedure_id TEXT REFERENCES knowledge_procedures(id) ON DELETE RESTRICT,
    framework_id TEXT REFERENCES knowledge_frameworks(id) ON DELETE RESTRICT,
    framework_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL,
    actor_id TEXT,
    occurred_at_us INTEGER NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    before_json TEXT,
    after_json TEXT,
    CHECK (
        (source_id IS NOT NULL) + (source_version_id IS NOT NULL) +
        (source_segment_id IS NOT NULL) + (extraction_id IS NOT NULL) +
        (source_claim_id IS NOT NULL) + (concept_id IS NOT NULL) +
        (principle_id IS NOT NULL) + (procedure_id IS NOT NULL) +
        (framework_id IS NOT NULL) + (framework_version_id IS NOT NULL) = 1
    )
);
CREATE INDEX idx_acquisition_provenance_source
    ON acquisition_provenance_events(source_id, occurred_at_us);
CREATE INDEX idx_acquisition_provenance_extraction
    ON acquisition_provenance_events(extraction_id, occurred_at_us);
CREATE INDEX idx_acquisition_provenance_claim
    ON acquisition_provenance_events(source_claim_id, occurred_at_us);
CREATE INDEX idx_acquisition_provenance_framework
    ON acquisition_provenance_events(framework_id, occurred_at_us);
CREATE TRIGGER acquisition_provenance_no_update
BEFORE UPDATE ON acquisition_provenance_events
BEGIN
    SELECT RAISE(ABORT, 'acquisition_provenance_events is append-only');
END;
CREATE TRIGGER acquisition_provenance_no_delete
BEFORE DELETE ON acquisition_provenance_events
BEGIN
    SELECT RAISE(ABORT, 'acquisition_provenance_events is append-only');
END;

CREATE TABLE knowledge_concepts (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    definition TEXT NOT NULL,
    lifecycle TEXT NOT NULL DEFAULT 'proposed'
        CHECK (lifecycle IN ('proposed', 'reviewed', 'accepted', 'deprecated')),
    created_at_us INTEGER NOT NULL
);
CREATE TABLE knowledge_principles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
    lifecycle TEXT NOT NULL DEFAULT 'proposed'
        CHECK (lifecycle IN ('proposed', 'reviewed', 'supported', 'contested', 'deprecated')),
    created_at_us INTEGER NOT NULL
);
CREATE TABLE knowledge_procedures (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL,
    lifecycle TEXT NOT NULL DEFAULT 'proposed'
        CHECK (lifecycle IN ('proposed', 'reviewed', 'active', 'deprecated')),
    created_at_us INTEGER NOT NULL
);

CREATE TABLE source_claim_relations (
    id TEXT PRIMARY KEY,
    subject_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    object_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    CHECK (subject_claim_id <> object_claim_id),
    UNIQUE (subject_claim_id, object_claim_id, relation_kind)
);
CREATE TABLE source_claim_concept_relations (
    id TEXT PRIMARY KEY,
    source_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    concept_id TEXT NOT NULL REFERENCES knowledge_concepts(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('defines', 'supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (source_claim_id, concept_id, relation_kind)
);
CREATE TABLE source_claim_principle_relations (
    id TEXT PRIMARY KEY,
    source_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    principle_id TEXT NOT NULL REFERENCES knowledge_principles(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (source_claim_id, principle_id, relation_kind)
);
CREATE TABLE source_claim_procedure_relations (
    id TEXT PRIMARY KEY,
    source_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    procedure_id TEXT NOT NULL REFERENCES knowledge_procedures(id) ON DELETE RESTRICT,
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('describes', 'supports', 'contradicts', 'qualifies')),
    rationale TEXT NOT NULL DEFAULT '',
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    UNIQUE (source_claim_id, procedure_id, relation_kind)
);

CREATE TABLE knowledge_frameworks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    lifecycle TEXT NOT NULL DEFAULT 'draft'
        CHECK (lifecycle IN ('draft', 'review', 'published', 'withdrawn', 'retired')),
    current_version_id TEXT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    FOREIGN KEY (current_version_id, id)
        REFERENCES knowledge_framework_versions(id, framework_id) ON DELETE RESTRICT
);
CREATE TABLE knowledge_framework_versions (
    id TEXT PRIMARY KEY,
    framework_id TEXT NOT NULL REFERENCES knowledge_frameworks(id) ON DELETE RESTRICT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    parent_version_id TEXT REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'review', 'published', 'withdrawn')),
    change_class TEXT CHECK (
        change_class IS NULL OR
        change_class IN ('refinement', 'extension', 'revision', 'replacement')
    ),
    change_rationale TEXT,
    created_by TEXT,
    created_at_us INTEGER NOT NULL,
    published_at_us INTEGER,
    UNIQUE (framework_id, version_no),
    UNIQUE (id, framework_id),
    FOREIGN KEY (parent_version_id, framework_id)
        REFERENCES knowledge_framework_versions(id, framework_id) ON DELETE RESTRICT,
    CHECK (
        (version_no = 1 AND parent_version_id IS NULL AND change_class IS NULL) OR
        (version_no > 1 AND parent_version_id IS NOT NULL AND
         change_class IS NOT NULL AND length(trim(change_rationale)) > 0)
    ),
    CHECK ((status = 'published' AND published_at_us IS NOT NULL) OR
           (status <> 'published' AND published_at_us IS NULL))
);
CREATE INDEX idx_knowledge_framework_versions
    ON knowledge_framework_versions(framework_id, version_no DESC);
CREATE TRIGGER framework_versions_published_no_update
BEFORE UPDATE ON knowledge_framework_versions
WHEN OLD.status = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published framework versions are immutable');
END;
CREATE TRIGGER framework_versions_published_no_delete
BEFORE DELETE ON knowledge_framework_versions
WHEN OLD.status = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published framework versions are immutable');
END;

CREATE TABLE framework_version_items (
    id TEXT PRIMARY KEY,
    framework_version_id TEXT NOT NULL REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    item_kind TEXT NOT NULL CHECK (item_kind IN ('source_claim', 'concept', 'principle', 'procedure')),
    source_claim_id TEXT REFERENCES source_claims(id) ON DELETE RESTRICT,
    concept_id TEXT REFERENCES knowledge_concepts(id) ON DELETE RESTRICT,
    principle_id TEXT REFERENCES knowledge_principles(id) ON DELETE RESTRICT,
    procedure_id TEXT REFERENCES knowledge_procedures(id) ON DELETE RESTRICT,
    membership_role TEXT NOT NULL CHECK (
        membership_role IN ('core', 'supporting', 'counterpoint', 'method', 'example', 'context')
    ),
    item_order INTEGER NOT NULL CHECK (item_order >= 0),
    rationale TEXT NOT NULL DEFAULT '',
    created_at_us INTEGER NOT NULL,
    CHECK (
        (item_kind = 'source_claim' AND source_claim_id IS NOT NULL AND concept_id IS NULL AND principle_id IS NULL AND procedure_id IS NULL) OR
        (item_kind = 'concept' AND source_claim_id IS NULL AND concept_id IS NOT NULL AND principle_id IS NULL AND procedure_id IS NULL) OR
        (item_kind = 'principle' AND source_claim_id IS NULL AND concept_id IS NULL AND principle_id IS NOT NULL AND procedure_id IS NULL) OR
        (item_kind = 'procedure' AND source_claim_id IS NULL AND concept_id IS NULL AND principle_id IS NULL AND procedure_id IS NOT NULL)
    ),
    UNIQUE (framework_version_id, item_order),
    UNIQUE (framework_version_id, source_claim_id, membership_role),
    UNIQUE (framework_version_id, concept_id, membership_role),
    UNIQUE (framework_version_id, principle_id, membership_role),
    UNIQUE (framework_version_id, procedure_id, membership_role)
);
CREATE INDEX idx_framework_items_version
    ON framework_version_items(framework_version_id, item_order);
CREATE TRIGGER framework_items_published_no_insert
BEFORE INSERT ON framework_version_items
WHEN (SELECT status FROM knowledge_framework_versions WHERE id = NEW.framework_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published framework versions are immutable');
END;
CREATE TRIGGER framework_items_published_no_update
BEFORE UPDATE ON framework_version_items
WHEN (SELECT status FROM knowledge_framework_versions WHERE id = OLD.framework_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published framework versions are immutable');
END;
CREATE TRIGGER framework_items_published_no_delete
BEFORE DELETE ON framework_version_items
WHEN (SELECT status FROM knowledge_framework_versions WHERE id = OLD.framework_version_id) = 'published'
BEGIN
    SELECT RAISE(ABORT, 'published framework versions are immutable');
END;

CREATE TABLE framework_version_change_links (
    id TEXT PRIMARY KEY,
    framework_version_id TEXT NOT NULL REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    source_claim_id TEXT NOT NULL REFERENCES source_claims(id) ON DELETE RESTRICT,
    link_kind TEXT NOT NULL CHECK (link_kind IN ('affected_claim', 'assumption')),
    rationale TEXT NOT NULL DEFAULT '',
    UNIQUE (framework_version_id, source_claim_id, link_kind)
);

CREATE TABLE framework_version_reviews (
    id TEXT PRIMARY KEY,
    framework_version_id TEXT NOT NULL REFERENCES knowledge_framework_versions(id) ON DELETE RESTRICT,
    review_status TEXT NOT NULL CHECK (review_status IN ('approved', 'changes_requested', 'rejected')),
    reviewer_id TEXT,
    rationale TEXT NOT NULL DEFAULT '',
    reviewed_at_us INTEGER NOT NULL
);
CREATE INDEX idx_framework_reviews_version
    ON framework_version_reviews(framework_version_id, reviewed_at_us, id);
CREATE TRIGGER framework_reviews_no_update
BEFORE UPDATE ON framework_version_reviews
BEGIN
    SELECT RAISE(ABORT, 'framework_version_reviews is append-only');
END;
CREATE TRIGGER framework_reviews_no_delete
BEFORE DELETE ON framework_version_reviews
BEGIN
    SELECT RAISE(ABORT, 'framework_version_reviews is append-only');
END;
CREATE TRIGGER framework_reviews_set_status
AFTER INSERT ON framework_version_reviews
BEGIN
    UPDATE knowledge_framework_versions
    SET status = CASE NEW.review_status
        WHEN 'approved' THEN 'review'
        WHEN 'changes_requested' THEN 'draft'
        ELSE 'draft'
    END
    WHERE id = NEW.framework_version_id AND status <> 'published';
END;

CREATE TRIGGER framework_published_version_pointer
AFTER UPDATE OF status ON knowledge_framework_versions
WHEN NEW.status = 'published' AND OLD.status <> 'published'
BEGIN
    UPDATE knowledge_frameworks
    SET current_version_id = NEW.id,
        lifecycle = 'published'
    WHERE id = NEW.framework_id;
END;

