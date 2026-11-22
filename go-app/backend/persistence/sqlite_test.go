package persistence

import (
	"path/filepath"
	"testing"
)

func TestFreshDatabaseInitializationIsRepeatable(t *testing.T) {
	path := filepath.Join(t.TempDir(), "vora.db")
	first, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := first.DB.Close(); err != nil {
		t.Fatal(err)
	}
	second, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer second.DB.Close()

	var foreignKeys int
	if err := second.DB.QueryRow("PRAGMA foreign_keys").Scan(&foreignKeys); err != nil {
		t.Fatal(err)
	}
	var migrations int
	if err := second.DB.QueryRow("SELECT COUNT(*) FROM schema_migrations").Scan(&migrations); err != nil {
		t.Fatal(err)
	}
	if foreignKeys != 1 {
		t.Fatal("foreign keys were not enabled")
	}
	if migrations != 6 {
		t.Fatalf("expected six migrations, got %d", migrations)
	}
}
