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

func TestValidBackupRestoreReopensDatabase(t *testing.T) {
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
	if _, err := store.DB.Exec(`INSERT INTO projects(id,name,description,created_at,updated_at) VALUES(?,?,?,?,?)`,
		"project_after_backup", "After", "", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"); err != nil {
		t.Fatal(err)
	}
	restored, err := manager.Restore(record.Path)
	if err != nil {
		t.Fatal(err)
	}
	if !restored {
		t.Fatal("restore should succeed")
	}
	var count int
	if err := store.DB.QueryRow(`SELECT COUNT(*) FROM projects WHERE id=?`, "project_after_backup").Scan(&count); err != nil {
		t.Fatal(err)
	}
	if count != 0 {
		t.Fatal("restored database should not contain post-backup project")
	}
	if err := store.DB.Close(); err != nil {
		t.Fatal(err)
	}
}

func TestRestoreRejectsWrongDatabaseAndIncompatibleSchema(t *testing.T) {
	root := t.TempDir()
	store, err := Open(filepath.Join(root, "active.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	wrong := filepath.Join(manager.root, "wrong.db")
	wrongStore, err := Open(wrong)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := wrongStore.DB.Exec(`DROP TABLE application_identity`); err != nil {
		wrongStore.DB.Close()
		t.Fatal(err)
	}
	wrongStore.DB.Close()
	if _, err := manager.PreflightRestore(wrong); err == nil {
		t.Fatal("wrong or incompatible database should be rejected")
	}
}
