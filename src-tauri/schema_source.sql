-- ═══════════════════════════════════════════════════════════
-- VORA SOURCE -- provenance / data lineage
-- Design principle: APPEND-ONLY. Nothing is ever deleted.
-- Every mutation writes a row in source_revisions.
-- ═══════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS sources (
    id              TEXT PRIMARY KEY,               -- SRC-0001
    kind            TEXT NOT NULL,                  -- video|audio|doc|url|book|image
    path            TEXT,                           -- local path or URL
    title           TEXT NOT NULL,
    author          TEXT,
    language        TEXT,
    duration_sec    INTEGER,                        -- for video/audio
    page_count      INTEGER,                        -- for book/doc
    metadata_json   TEXT NOT NULL DEFAULT '{}',
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active', -- active|archived
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS segments (
    id              TEXT PRIMARY KEY,               -- SEG-0001
    source_id       TEXT NOT NULL REFERENCES sources(id),
    label           TEXT NOT NULL,
    start_sec       INTEGER,                        -- for time-based
    end_sec         INTEGER,
    page_start      INTEGER,                        -- for page-based
    page_end        INTEGER,
    line_start      INTEGER,                        -- for text-based
    line_end        INTEGER,
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    parent_segment_id TEXT REFERENCES segments(id),
    order_index     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS extracts (
    id              TEXT PRIMARY KEY,               -- EXT-0001
    segment_id      TEXT NOT NULL REFERENCES segments(id),
    transform_kind  TEXT NOT NULL,                  -- transcribed|summarized|translated|quoted|paraphrased|extracted_idea|keyword|question|answer
    content         TEXT NOT NULL,
    language        TEXT,
    confidence      TEXT NOT NULL DEFAULT 'certain',-- certain|probable|uncertain
    created_by      TEXT NOT NULL DEFAULT 'user',   -- user|whisper|llm|import
    reviewed_by_user INTEGER NOT NULL DEFAULT 1,    -- 0 = pending review
    parent_extract_id TEXT REFERENCES extracts(id), -- chain of derivations
    superseded_by   TEXT REFERENCES extracts(id),   -- replaced by newer extract
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS links (
    id              TEXT PRIMARY KEY,               -- LNK-0001
    extract_id      TEXT NOT NULL REFERENCES extracts(id),
    target_kind     TEXT NOT NULL,                  -- mind_node|chart_annotation|source|external
    target_id       TEXT NOT NULL,
    link_kind       TEXT NOT NULL DEFAULT 'reference', -- evidence|reference|inspiration|contradicts|supports
    note            TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS source_revisions (
    id              TEXT PRIMARY KEY,               -- REV-0001
    entity_kind     TEXT NOT NULL,                  -- source|segment|extract|link
    entity_id       TEXT NOT NULL,
    revision_type   TEXT NOT NULL,                  -- create|update|correct|retract|review
    before_json     TEXT,                           -- NULL only for 'create'
    after_json      TEXT NOT NULL,
    reason          TEXT NOT NULL DEFAULT '',
    created_by      TEXT NOT NULL DEFAULT 'user',
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_segments_source
    ON segments(source_id);
CREATE INDEX IF NOT EXISTS idx_extracts_segment
    ON extracts(segment_id);
CREATE INDEX IF NOT EXISTS idx_extracts_parent
    ON extracts(parent_extract_id);
CREATE INDEX IF NOT EXISTS idx_extracts_superseded
    ON extracts(superseded_by);
CREATE INDEX IF NOT EXISTS idx_links_extract
    ON links(extract_id);
CREATE INDEX IF NOT EXISTS idx_links_target
    ON links(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_revisions_entity
    ON source_revisions(entity_kind, entity_id, created_at);

-- Append-only enforcement: source_revisions can NEVER be
-- modified or deleted, at the SQLite level.
CREATE TRIGGER IF NOT EXISTS prevent_revision_update
BEFORE UPDATE ON source_revisions
BEGIN
    SELECT RAISE(ABORT, 'source_revisions is append-only');
END;

CREATE TRIGGER IF NOT EXISTS prevent_revision_delete
BEFORE DELETE ON source_revisions
BEGIN
    SELECT RAISE(ABORT, 'source_revisions is append-only');
END;
