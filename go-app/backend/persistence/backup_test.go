package persistence

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

type failingEvalFilesystem struct {
	backupFilesystem
	err error
}

func (f failingEvalFilesystem) EvalSymlinks(path string) (string, error) {
	return "", f.err
}

func requireTestIsolation(t *testing.T, root string, paths ...string) {
	t.Helper()
	absoluteRoot, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("could not resolve test root: %v", err)
	}
	absoluteRoot = filepath.Clean(absoluteRoot)
	protectedRoots := []string{}
	if configDir, err := os.UserConfigDir(); err == nil {
		protectedRoots = append(protectedRoots, filepath.Join(configDir, "Vora Mind"))
	}
	protectedRoots = append(protectedRoots,
		`C:\Users\HELIX\AppData\Local\Temp\vora-phase3122-acceptance-ecd291a08fcf444da2fbe2503ebbf09d\profile`,
		`D:\workspace\vora_mind\acceptance-mcp-data`,
	)
	for _, protected := range protectedRoots {
		absoluteProtected, err := filepath.Abs(protected)
		if err != nil {
			t.Fatalf("could not resolve protected test path: %v", err)
		}
		if pathWithinTestTree(absoluteRoot, absoluteProtected) || pathWithinTestTree(absoluteProtected, absoluteRoot) {
			t.Fatalf("test isolation guard rejected root %q overlapping protected path %q", absoluteRoot, absoluteProtected)
		}
	}
	for _, path := range paths {
		absolutePath, err := filepath.Abs(path)
		if err != nil {
			t.Fatalf("could not resolve test path %q: %v", path, err)
		}
		for _, protected := range protectedRoots {
			absoluteProtected, err := filepath.Abs(protected)
			if err != nil {
				t.Fatalf("could not resolve protected test path: %v", err)
			}
			if stringsEqualTestPath(absolutePath, absoluteProtected) || pathWithinTestTree(absoluteProtected, absolutePath) {
				t.Fatalf("test path %q overlaps protected path %q", absolutePath, absoluteProtected)
			}
		}
	}
}

func pathWithinTestTree(root, target string) bool {
	relative, err := filepath.Rel(filepath.Clean(root), filepath.Clean(target))
	if err != nil || filepath.IsAbs(relative) {
		return false
	}
	return relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator))
}

func stringsEqualTestPath(left, right string) bool {
	left = filepath.Clean(left)
	right = filepath.Clean(right)
	if runtime.GOOS == "windows" {
		return strings.EqualFold(left, right)
	}
	return left == right
}

func isolatedTempDir(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	requireTestIsolation(t, root)
	return root
}

func TestBackupCreateValidateAndPreflightRestore(t *testing.T) {
	root := isolatedTempDir(t)
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
	path := filepath.Join(isolatedTempDir(t), "corrupt.db")
	if err := os.WriteFile(path, []byte("not sqlite"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ValidateDatabase(path); err == nil {
		t.Fatal("corrupt database should fail validation")
	}
}

func TestRestorePreflightRejectsOutsideScope(t *testing.T) {
	root := isolatedTempDir(t)
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
	outside := filepath.Join(isolatedTempDir(t), "outside.db")
	if err := os.WriteFile(outside, []byte("x"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := manager.PreflightRestore(outside); err == nil {
		t.Fatal("restore outside backup scope should fail")
	}
}

func TestValidBackupRestoreReopensDatabase(t *testing.T) {
	root := isolatedTempDir(t)
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
	root := isolatedTempDir(t)
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

func TestRestorePreservesProjectAndWorkspaceRoot(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	workspaceRoot := filepath.Join(root, "projects", "project_restore_test")
	if err := os.MkdirAll(workspaceRoot, 0o700); err != nil {
		t.Fatal(err)
	}
	now := "2026-11-22T19:00:00Z"
	if _, err := store.DB.Exec(`INSERT INTO projects(id,name,description,created_at,updated_at) VALUES(?,?,?,?,?)`,
		"project_restore_test", "Restore Test", "", now, now); err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`INSERT INTO project_filesystem_roots(project_id,workspace_root,created_at,updated_at) VALUES(?,?,?,?)`,
		"project_restore_test", workspaceRoot, now, now); err != nil {
		t.Fatal(err)
	}
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	record, err := manager.Create()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`DELETE FROM project_filesystem_roots WHERE project_id=?`, "project_restore_test"); err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`DELETE FROM projects WHERE id=?`, "project_restore_test"); err != nil {
		t.Fatal(err)
	}
	restored, err := manager.Restore(record.Path)
	if err != nil {
		t.Fatal(err)
	}
	if !restored {
		t.Fatal("isolated project restore should succeed")
	}
	var count int
	if err := store.DB.QueryRow(`SELECT COUNT(*) FROM projects WHERE id=?`, "project_restore_test").Scan(&count); err != nil {
		t.Fatal(err)
	}
	if count != 1 {
		t.Fatalf("restored project count=%d", count)
	}
	var restoredRoot string
	if err := store.DB.QueryRow(`SELECT workspace_root FROM project_filesystem_roots WHERE project_id=?`, "project_restore_test").Scan(&restoredRoot); err != nil {
		t.Fatal(err)
	}
	if restoredRoot != workspaceRoot {
		t.Fatalf("restored workspace root=%q want=%q", restoredRoot, workspaceRoot)
	}
	if err := store.DB.Close(); err != nil {
		t.Fatal(err)
	}
}

func TestRestorePreflightRejectsSameFile(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := manager.PreflightRestore(database); err == nil {
		t.Fatal("active database and backup source must not resolve to the same file")
	}
}

func TestRestorePreflightRejectsTraversal(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	outsideBackup := filepath.Join(root, "outside.db")
	requireTestIsolation(t, root, database, outsideBackup)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	if err := os.WriteFile(outsideBackup, []byte("not sqlite"), 0o600); err != nil {
		t.Fatal(err)
	}
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	traversalPath := filepath.Join(manager.root, "..", "outside.db")
	if _, err := manager.PreflightRestore(traversalPath); err == nil {
		t.Fatal("traversal outside the backup root must be rejected")
	}
}

func TestRestorePreflightRejectsSymlinkEscape(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	outsideBackup := filepath.Join(root, "outside.db")
	requireTestIsolation(t, root, database, outsideBackup)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	if err := os.WriteFile(outsideBackup, []byte("not sqlite"), 0o600); err != nil {
		t.Fatal(err)
	}
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	symlinkPath := filepath.Join(manager.root, "escape.db")
	if err := os.Symlink(outsideBackup, symlinkPath); err != nil {
		t.Skipf("filesystem does not allow test symlinks: %v", err)
	}
	if _, err := manager.PreflightRestore(symlinkPath); err == nil {
		t.Fatal("symlink resolving outside the backup root must be rejected")
	}
}

func TestRestorePreflightAcceptsValidInRootBackup(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
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
	valid, err := manager.PreflightRestore(record.Path)
	if err != nil || !valid {
		t.Fatalf("valid in-root backup should pass preflight: valid=%t err=%v", valid, err)
	}
}

func TestRestorePreflightRejectsHardLinkToActiveDatabase(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	hardLink := filepath.Join(manager.root, "active-hard-link.db")
	if err := os.Link(database, hardLink); err != nil {
		t.Skipf("platform cannot create a hard link for this test: %v", err)
	}
	if _, err := manager.PreflightRestore(hardLink); err == nil {
		t.Fatal("hard link to the active database must be rejected")
	}
}

func TestRestorePreflightRejectsCrossVolumePath(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("cross-volume path semantics are Windows-specific")
	}
	root := isolatedTempDir(t)
	rootVolume := filepath.VolumeName(root)
	candidateVolume := "Z:"
	if strings.EqualFold(rootVolume, candidateVolume) {
		candidateVolume = "Y:"
	}
	inside := filepath.Join(root, "backups")
	outside := filepath.Join(candidateVolume+"\\", "outside.db")
	inside, err := filepath.Abs(inside)
	if err != nil {
		t.Fatal(err)
	}
	insideCanonical := filepath.Clean(inside)
	outsideCanonical := filepath.Clean(outside)
	relative, err := filepath.Rel(insideCanonical, outsideCanonical)
	if err == nil {
		if !filepath.IsAbs(relative) {
			t.Fatalf("cross-volume synthetic path unexpectedly produced a rooted relative path: %q", relative)
		}
	}
}

func TestRestorePreflightRejectsWindowsJunctionEscape(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Windows junction escape testing is Windows-specific")
	}
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager, err := NewBackupManager(store, filepath.Join(root, "backups"))
	if err != nil {
		t.Fatal(err)
	}
	outsideDir := filepath.Join(root, "outside")
	if err := os.MkdirAll(outsideDir, 0o700); err != nil {
		t.Fatal(err)
	}
	outsideDB := filepath.Join(outsideDir, "outside.db")
	if err := os.WriteFile(outsideDB, []byte("not sqlite"), 0o600); err != nil {
		t.Fatal(err)
	}
	junction := filepath.Join(manager.root, "escape")
	command := exec.Command("cmd.exe", "/c", "mklink", "/J", junction, outsideDir)
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("genuine Windows junction could not be created for escape testing: %v: %s", err, output)
	}
	junctionSource := filepath.Join(junction, "outside.db")
	if _, err := manager.PreflightRestore(junctionSource); err == nil {
		t.Fatal("junction resolving outside the backup root must be rejected")
	}
}

func TestRestoreUsesCanonicalActiveDatabaseTarget(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
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
		"post_backup_project", "Post Backup", "", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"); err != nil {
		t.Fatal(err)
	}

	aliasDir := filepath.Join(root, "alias")
	if err := os.MkdirAll(aliasDir, 0o700); err != nil {
		t.Fatal(err)
	}
	alias := filepath.Join(aliasDir, "active.db")
	if err := os.Symlink(database, alias); err != nil {
		t.Skipf("platform cannot create the canonical-target symlink: %v", err)
	}
	store.Database = alias

	restored, err := manager.Restore(record.Path)
	if err != nil {
		t.Fatal(err)
	}
	if !restored {
		t.Fatal("restore should succeed")
	}
	canonicalDatabase, err := filepath.EvalSymlinks(alias)
	if err != nil {
		t.Fatal(err)
	}
	canonicalDatabase, err = filepath.Abs(canonicalDatabase)
	if err != nil {
		t.Fatal(err)
	}
	if !stringsEqualTestPath(store.Database, canonicalDatabase) {
		t.Fatalf("store database target = %q, want canonical target %q", store.Database, canonicalDatabase)
	}
	var count int
	if err := store.DB.QueryRow(`SELECT COUNT(*) FROM projects WHERE id=?`, "post_backup_project").Scan(&count); err != nil {
		t.Fatal(err)
	}
	if count != 0 {
		t.Fatal("canonical-target restore did not restore the backup state")
	}
}

func TestRestorePreflightFailsClosedWhenCanonicalizationFails(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
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

	failing := failingEvalFilesystem{
		backupFilesystem: osBackupFilesystem{},
		err:              errors.New("canonicalization unavailable"),
	}
	injected, err := newBackupManager(store, manager.root, failing)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := injected.PreflightRestore(record.Path); err == nil {
		t.Fatal("canonicalization failure must fail closed")
	}
}

func TestNewBackupManagerRequiresFilesystem(t *testing.T) {
	root := isolatedTempDir(t)
	database := filepath.Join(root, "active.db")
	requireTestIsolation(t, root, database)
	store, err := Open(database)
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	if _, err := newBackupManager(store, filepath.Join(root, "backups"), nil); err == nil {
		t.Fatal("nil filesystem must be rejected")
	}
}
