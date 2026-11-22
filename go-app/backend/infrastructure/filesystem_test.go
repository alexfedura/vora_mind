package infrastructure

import (
	"os"
	"os/exec"
	"path/filepath"
	goruntime "runtime"
	"testing"
)

func TestFilesystemScopePreventsEscape(t *testing.T) {
	root := t.TempDir()
	scope, err := NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	outside := filepath.Join(t.TempDir(), "outside.txt")
	if _, err := scope.CreateText(outside, "blocked"); err == nil {
		t.Fatal("absolute path outside scope should be rejected")
	}
	if _, err := os.Stat(outside); !os.IsNotExist(err) {
		t.Fatal("outside file should not exist")
	}
}

func TestFilesystemTextOperationsInsideScope(t *testing.T) {
	root := t.TempDir()
	scope, err := NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join("notes", "sample.txt")
	created, err := scope.CreateText(path, "hello")
	if err != nil {
		t.Fatal(err)
	}
	if created["size"] != int64(5) {
		t.Fatalf("unexpected created size: %#v", created)
	}
	read, err := scope.ReadText(path)
	if err != nil {
		t.Fatal(err)
	}
	if read["content"] != "hello" {
		t.Fatalf("unexpected content: %#v", read)
	}
}

func TestProjectResolutionRejectsUnsafePaths(t *testing.T) {
	root := t.TempDir()
	scope, err := NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	projectRoot := filepath.Join(root, "project-a")
	if err := scope.SetProjectRoot("project-a", projectRoot); err != nil {
		t.Fatal(err)
	}
	for _, candidate := range []string{
		"..\\outside.txt",
		"%2e%2e/outside.txt",
		filepath.Join(projectRoot, "..", "outside.txt"),
		`\\server\share\outside.txt`,
	} {
		if _, err := scope.ResolveProject("project-a", candidate); err == nil {
			t.Fatalf("unsafe project path should be rejected: %q", candidate)
		}
	}
	resolved, err := scope.ResolveProject("project-a", "nested/file.txt")
	if err != nil {
		t.Fatal(err)
	}
	if _, err := filepath.Rel(projectRoot, resolved); err != nil {
		t.Fatal(err)
	}
}

func TestProjectRootMustBeInsideAllowedScope(t *testing.T) {
	allowed := t.TempDir()
	scope, err := NewFilesystemScope(allowed)
	if err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	if _, err := scope.ValidateProjectRoot(outside); err == nil {
		t.Fatal("project root outside allowed scope should be rejected")
	}
	inside := filepath.Join(allowed, "project-a")
	if _, err := scope.ValidateProjectRoot(inside); err != nil {
		t.Fatal(err)
	}
}

func TestWindowsJunctionEscapeRejected(t *testing.T) {
	if goruntime.GOOS != "windows" {
		t.Skip("Windows junction test")
	}
	allowed := t.TempDir()
	scope, err := NewFilesystemScope(allowed)
	if err != nil {
		t.Fatal(err)
	}
	projectRoot := filepath.Join(allowed, "project-a")
	if err := os.MkdirAll(projectRoot, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := scope.SetProjectRoot("project-a", projectRoot); err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	junction := filepath.Join(projectRoot, "outside-junction")
	cmd := exec.Command("cmd.exe", "/d", "/c", "mklink", "/J", junction, outside)
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Skipf("junction privilege unavailable: %v: %s", err, string(output))
	}
	defer os.RemoveAll(junction)
	resolved, err := scope.ResolveProject("project-a", filepath.Join("outside-junction", "secret.txt"))
	canonicalJunction, canonicalErr := canonicalExistingPath(junction)
	if err == nil {
		t.Fatalf("junction escape should be rejected; resolved=%q junction=%q canonicalErr=%v", resolved, canonicalJunction, canonicalErr)
	}
}

func TestWindowsDirectorySymlinkEscapeRejected(t *testing.T) {
	if goruntime.GOOS != "windows" {
		t.Skip("Windows symlink test")
	}
	allowed := t.TempDir()
	scope, err := NewFilesystemScope(allowed)
	if err != nil {
		t.Fatal(err)
	}
	projectRoot := filepath.Join(allowed, "project-a")
	if err := os.MkdirAll(projectRoot, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := scope.SetProjectRoot("project-a", projectRoot); err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	link := filepath.Join(projectRoot, "outside-link")
	if err := os.Symlink(outside, link); err != nil {
		t.Skipf("directory symlink privilege unavailable: %v", err)
	}
	defer os.Remove(link)
	if _, err := scope.ResolveProject("project-a", filepath.Join("outside-link", "secret.txt")); err == nil {
		t.Fatal("directory symlink escape should be rejected")
	}
}

func TestBrokenSymlinkRejected(t *testing.T) {
	root := t.TempDir()
	scope, err := NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	projectRoot := filepath.Join(root, "project-a")
	if err := os.MkdirAll(projectRoot, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := scope.SetProjectRoot("project-a", projectRoot); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(projectRoot, "broken-link")
	if err := os.Symlink(filepath.Join(root, "missing-target"), link); err != nil {
		t.Skipf("symlink privilege unavailable: %v", err)
	}
	defer os.Remove(link)
	if _, err := scope.ResolveProject("project-a", "broken-link"); err == nil {
		t.Fatal("broken symlink should be rejected")
	}
}
