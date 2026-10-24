PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 30000000000;

-- ۱. هویت اشیاء
CREATE TABLE IF NOT EXISTS objects (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    deleted_at INTEGER NULL,
    metadata JSON
);

-- ۲. نسخه‌های تغییرناپذیر محتوا
CREATE TABLE IF NOT EXISTS object_versions (
    id TEXT PRIMARY KEY,
    object_id TEXT NOT NULL,
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    parent_version_id TEXT,
    reason TEXT,
    summary TEXT,
    content JSON NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(object_id) REFERENCES objects(id),
    FOREIGN KEY(parent_version_id) REFERENCES object_versions(id)
);

-- ۳. اسنپ‌شات‌ها و متادیتای هزینه
CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_snapshot_id TEXT,
    depth INTEGER NOT NULL,
    replay_cost INTEGER NOT NULL DEFAULT 0,
    is_checkpoint INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(parent_snapshot_id) REFERENCES snapshots(id)
);

-- ۴. دلتاهای آبجکت
CREATE TABLE IF NOT EXISTS snapshot_object_changes (
    snapshot_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    operation TEXT NOT NULL, -- 'ADD', 'REMOVE', 'UPDATE_VERSION'
    object_version_id TEXT,
    PRIMARY KEY (snapshot_id, object_id),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(id),
    FOREIGN KEY(object_id) REFERENCES objects(id),
    FOREIGN KEY(object_version_id) REFERENCES object_versions(id)
) WITHOUT ROWID;

-- ۵. دلتاهای چیدمان (Placement)
CREATE TABLE IF NOT EXISTS snapshot_placement_changes (
    snapshot_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    operation TEXT NOT NULL, -- 'ADD', 'MOVE', 'RESIZE', 'STYLE', 'REMOVE'
    x REAL,
    y REAL,
    width REAL,
    height REAL,
    rotation REAL DEFAULT 0,
    z_index INTEGER DEFAULT 0,
    style JSON,
    PRIMARY KEY (snapshot_id, object_id),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(id),
    FOREIGN KEY(object_id) REFERENCES objects(id)
) WITHOUT ROWID;

-- ۶. دلتاهای اتصالات گراف (Links)
CREATE TABLE IF NOT EXISTS snapshot_link_changes (
    snapshot_id TEXT NOT NULL,
    link_id TEXT NOT NULL,
    operation TEXT NOT NULL, -- 'ADD', 'REMOVE', 'UPDATE_TYPE'
    source_object_id TEXT,
    target_object_id TEXT,
    link_type TEXT,
    PRIMARY KEY (snapshot_id, link_id),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(id)
) WITHOUT ROWID;

-- ۷. وضعیت مادی‌شده چک‌پوینت‌ها (Materialized Checkpoints)
CREATE TABLE IF NOT EXISTS checkpoint_objects (
    checkpoint_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_version_id TEXT NOT NULL,
    PRIMARY KEY (checkpoint_id, object_id),
    FOREIGN KEY(checkpoint_id) REFERENCES snapshots(id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS checkpoint_placements (
    checkpoint_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    width REAL NOT NULL,
    height REAL NOT NULL,
    rotation REAL DEFAULT 0,
    z_index INTEGER DEFAULT 0,
    style JSON,
    PRIMARY KEY (checkpoint_id, object_id),
    FOREIGN KEY(checkpoint_id) REFERENCES snapshots(id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS checkpoint_links (
    checkpoint_id TEXT NOT NULL,
    link_id TEXT NOT NULL,
    source_object_id TEXT NOT NULL,
    target_object_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    PRIMARY KEY (checkpoint_id, link_id),
    FOREIGN KEY(checkpoint_id) REFERENCES snapshots(id)
) WITHOUT ROWID;

-- ایندکس‌های حیاتی
CREATE INDEX IF NOT EXISTS idx_snapshots_parent ON snapshots(parent_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_checkpoint ON snapshots(is_checkpoint);
CREATE INDEX IF NOT EXISTS idx_snapshots_depth ON snapshots(depth);
CREATE INDEX IF NOT EXISTS idx_versions_obj ON object_versions(object_id);

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
    updated_at      INTEGER NOT NULL
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

-- ═══════════════════════════════════════════════════════════
-- VORA SOURCE -- provenance / data lineage
-- Design principle: APPEND-ONLY. Nothing is ever deleted.
-- Every mutation writes a row in source_revisions.
-- ═══════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS sources (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,
    path            TEXT,
    title           TEXT NOT NULL,
    author          TEXT,
    language        TEXT,
    duration_sec    INTEGER,
    page_count      INTEGER,
    metadata_json   TEXT NOT NULL DEFAULT '{}',
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS segments (
    id              TEXT PRIMARY KEY,
    source_id       TEXT NOT NULL REFERENCES sources(id),
    label           TEXT NOT NULL,
    start_sec       INTEGER,
    end_sec         INTEGER,
    page_start      INTEGER,
    page_end        INTEGER,
    line_start      INTEGER,
    line_end        INTEGER,
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS extracts (
    id              TEXT PRIMARY KEY,
    segment_id      TEXT NOT NULL REFERENCES segments(id),
    transform_kind  TEXT NOT NULL,
    content         TEXT NOT NULL,
    language        TEXT,
    confidence      TEXT NOT NULL DEFAULT 'certain',
    created_by      TEXT NOT NULL DEFAULT 'user',
    reviewed_by_user INTEGER NOT NULL DEFAULT 1,
    parent_extract_id TEXT REFERENCES extracts(id),
    superseded_by   TEXT REFERENCES extracts(id),
    notes           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS links (
    id              TEXT PRIMARY KEY,
    extract_id      TEXT NOT NULL REFERENCES extracts(id),
    target_kind     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    link_kind       TEXT NOT NULL DEFAULT 'reference',
    note            TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS source_revisions (
    id              TEXT PRIMARY KEY,
    entity_kind     TEXT NOT NULL,
    entity_id       TEXT NOT NULL,
    revision_type   TEXT NOT NULL,
    before_json     TEXT,
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
