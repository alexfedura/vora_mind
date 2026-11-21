package infrastructure

import (
	"os"
	"path/filepath"
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
