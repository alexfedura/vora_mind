CREATE TABLE research_project_sources (
    project_id TEXT NOT NULL REFERENCES research_projects(id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
    use_context TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    created_at_us INTEGER NOT NULL,
    updated_at_us INTEGER NOT NULL,
    PRIMARY KEY (project_id, source_id)
);

CREATE INDEX idx_research_project_sources_source
    ON research_project_sources(source_id);
