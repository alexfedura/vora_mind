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
	return nil
}
