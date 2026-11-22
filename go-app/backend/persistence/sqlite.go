package persistence

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	_ "modernc.org/sqlite"
)

type Store struct {
	DB       *sql.DB
	Database string
}

var migrations = []struct {
	Version int
	SQL     string
}{
	{1, `
CREATE TABLE IF NOT EXISTS schema_migrations (
 version INTEGER PRIMARY KEY,
 applied_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS projects (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL,
 description TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 CONSTRAINT projects_name_length CHECK (length(trim(name)) BETWEEN 1 AND 120)
);
CREATE INDEX IF NOT EXISTS idx_projects_updated_at ON projects(updated_at);
CREATE TABLE IF NOT EXISTS audit_events (
 execution_id TEXT PRIMARY KEY,
 capability_id TEXT NOT NULL,
 timestamp TEXT NOT NULL,
 origin TEXT NOT NULL,
 project_scope TEXT NOT NULL DEFAULT '',
 status TEXT NOT NULL,
 duration_ms INTEGER NOT NULL,
 error_json TEXT NOT NULL DEFAULT '',
 permissions_json TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_events_capability ON audit_events(capability_id);
`},
	{2, `
CREATE TABLE IF NOT EXISTS execution_history (
 execution_id TEXT PRIMARY KEY,
 capability_id TEXT NOT NULL,
 origin TEXT NOT NULL,
 principal_id TEXT NOT NULL DEFAULT '',
 project_scope TEXT NOT NULL DEFAULT '',
 workflow_id TEXT NOT NULL DEFAULT '',
 correlation_id TEXT NOT NULL DEFAULT '',
 started_at TEXT NOT NULL,
 ended_at TEXT NOT NULL,
 duration_ms INTEGER NOT NULL,
 status TEXT NOT NULL,
 result_summary TEXT NOT NULL DEFAULT '',
 error_json TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS workflows (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL,
 description TEXT NOT NULL DEFAULT '',
 project_scope TEXT NOT NULL DEFAULT '',
 creator TEXT NOT NULL DEFAULT '',
 origin TEXT NOT NULL,
 status TEXT NOT NULL,
 current_step INTEGER NOT NULL DEFAULT 0,
 cancellation_requested INTEGER NOT NULL DEFAULT 0,
 approval_state TEXT NOT NULL DEFAULT 'none',
 error_json TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflow_steps (
 id TEXT PRIMARY KEY,
 workflow_id TEXT NOT NULL,
 step_index INTEGER NOT NULL,
 capability_id TEXT NOT NULL,
 input_json TEXT NOT NULL,
 status TEXT NOT NULL,
 execution_id TEXT NOT NULL DEFAULT '',
 result_json TEXT NOT NULL DEFAULT '',
 error_json TEXT NOT NULL DEFAULT '',
 retry_count INTEGER NOT NULL DEFAULT 0,
 max_retries INTEGER NOT NULL DEFAULT 0,
 approval_required INTEGER NOT NULL DEFAULT 0,
 approval_status TEXT NOT NULL DEFAULT 'not_required',
 started_at TEXT,
 ended_at TEXT,
 UNIQUE(workflow_id, step_index),
 FOREIGN KEY(workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_workflow_steps_workflow ON workflow_steps(workflow_id);
CREATE INDEX IF NOT EXISTS idx_execution_history_started ON execution_history(started_at);
`},
}

func Open(databasePath string) (*Store, error) {
	if strings.TrimSpace(databasePath) == "" {
		return nil, fmt.Errorf("database path is required")
	}
	if err := os.MkdirAll(filepath.Dir(databasePath), 0o700); err != nil {
		return nil, err
	}
	db, err := sql.Open("sqlite", databasePath)
	if err != nil {
		return nil, err
	}
	db.SetMaxOpenConns(1)
	db.SetMaxIdleConns(1)
	if _, err := db.Exec("PRAGMA foreign_keys = ON"); err != nil {
		db.Close()
		return nil, err
	}
	if _, err := db.Exec("PRAGMA busy_timeout = 5000"); err != nil {
		db.Close()
		return nil, err
	}
	if err := applyMigrations(db); err != nil {
		db.Close()
		return nil, err
	}
	return &Store{DB: db, Database: databasePath}, nil
}

func applyMigrations(db *sql.DB) error {
	if _, err := db.Exec("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)"); err != nil {
		return err
	}
	rows, err := db.Query("SELECT version FROM schema_migrations")
	if err != nil {
		return err
	}
	applied := map[int]bool{}
	for rows.Next() {
		var version int
		if err := rows.Scan(&version); err != nil {
			rows.Close()
			return err
		}
		applied[version] = true
	}
	rows.Close()
	versions := make([]int, 0, len(migrations))
	for _, migration := range migrations {
		versions = append(versions, migration.Version)
	}
	sort.Ints(versions)
	for _, migration := range migrations {
		if applied[migration.Version] {
			continue
		}
		tx, err := db.Begin()
		if err != nil {
			return err
		}
		if _, err := tx.Exec(migration.SQL); err != nil {
			tx.Rollback()
			return err
		}
		if _, err := tx.Exec("INSERT INTO schema_migrations(version, applied_at) VALUES (?, ?)", migration.Version, "1970-01-01T00:00:00Z"); err != nil {
			tx.Rollback()
			return err
		}
		if err := tx.Commit(); err != nil {
			return err
		}
	}
	for _, column := range []string{"principal_id", "correlation_id", "workflow_id"} {
		var count int
		if err := db.QueryRow("SELECT COUNT(*) FROM pragma_table_info('audit_events') WHERE name = ?", column).Scan(&count); err != nil {
			return err
		}
		if count == 0 {
			if _, err := db.Exec("ALTER TABLE audit_events ADD COLUMN " + column + " TEXT NOT NULL DEFAULT ''"); err != nil {
				return err
			}
		}
	}
	return nil
}
