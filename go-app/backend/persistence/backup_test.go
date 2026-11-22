package persistence

import (
	"os"
	"path/filepath"
	"testing"
)

func TestBackupCreateValidateAndPreflightRestore(t *testing.T) {
	root := t.TempDir()
	database := filepath.Join(root, "active.db")
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	record, err := manager.Create()
	if err != nil {
		t.Fatal(err)
	}
	if !record.Valid {
		t.Fatal("backup should be valid")
	}
	valid, err := ValidateDatabase(record.Path)
	if err != nil || !valid {
		t.Fatalf("validation failed: valid=%v err=%v", valid, err)
	}
	preflight, err := manager.PreflightRestore(record.Path)
	if err != nil || !preflight {
		t.Fatalf("preflight failed: valid=%v err=%v", preflight, err)
	}
}

func TestBackupRejectsCorruptDatabase(t *testing.T) {
	path := filepath.Join(t.TempDir(), "corrupt.db")
	if err := os.WriteFile(path, []byte("not sqlite"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ValidateDatabase(path); err == nil {
		t.Fatal("corrupt database should fail validation")
	}
}

func TestRestorePreflightRejectsOutsideScope(t *testing.T) {
	root := t.TempDir()
	database := filepath.Join(root, "active.db")
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	outside := filepath.Join(t.TempDir(), "outside.db")
	if err := os.WriteFile(outside, []byte("x"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := manager.PreflightRestore(outside); err == nil {
		t.Fatal("restore outside backup scope should fail")
	}
}
