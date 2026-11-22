package application

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/domain"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

func testRuntime(t *testing.T) *Runtime {
	t.Helper()
	store, err := persistence.Open(filepath.Join(t.TempDir(), "test.db"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { store.DB.Close() })
	filesystem, err := infrastructure.NewFilesystemScope(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	return NewRuntime(store, filesystem, "Vora Test", "0.26.11", "v3.1.0")
}

func TestProjectCRUDAndRepeatedInitialization(t *testing.T) {
	runtime := testRuntime(t)
	created := runtime.Execute("project.create", map[string]interface{}{"name": "Research", "description": "Test project"}, string(capabilities.OriginHuman))
	if created.Status != "succeeded" {
		t.Fatalf("project create failed: %#v", created.Error)
	}
	data := created.Data.(map[string]interface{})
	project := data["project"]
	projects := runtime.Execute("project.list", map[string]interface{}{}, string(capabilities.OriginHuman))
	if projects.Status != "succeeded" {
		t.Fatal(projects.Error)
	}
	if project == nil {
		t.Fatal("created project missing")
	}
}

func TestProjectDeleteRequiresExplicitApproval(t *testing.T) {
	runtime := testRuntime(t)
	created := runtime.Execute("project.create", map[string]interface{}{"name": "Disposable", "description": "Delete test"}, string(capabilities.OriginHuman))
	if created.Status != "succeeded" {
		t.Fatal(created.Error)
	}
	project := created.Data.(map[string]interface{})["project"].(domain.Project)

	withoutApproval := runtime.Execute("project.delete", map[string]interface{}{"id": project.ID}, string(capabilities.OriginHuman))
	if withoutApproval.Status != "failed" || withoutApproval.Error == nil || withoutApproval.Error.Code != "APPROVAL_REQUIRED" {
		t.Fatalf("delete should require approval, got %#v", withoutApproval)
	}

	approvedContext := capabilities.ExecutionContext{
		Origin:        capabilities.OriginHuman,
		Principal:     capabilities.Principal{ID: "wails_human", Type: "human"},
		Permissions:   []security.Permission{security.ProjectWrite, security.ProjectRead},
		ProjectScope:  project.ID,
		CorrelationID: "wails_project_delete_" + project.ID,
		Approved:      true,
	}
	approved := runtime.ExecuteWithContext(approvedContext, "project.delete", map[string]interface{}{"id": project.ID})
	if approved.Status != "succeeded" {
		t.Fatalf("approved delete failed: %#v", approved.Error)
	}
}

func TestUnauthorizedFilesystemWriteCannotExecute(t *testing.T) {
	runtime := testRuntime(t)
	result := runtime.Execute("filesystem.create_text", map[string]interface{}{"projectId": "unassigned", "path": "forbidden.txt", "content": "no"}, "ai_agent")
	if result.Status != "succeeded" {
		if result.Error == nil || result.Error.Code != "PERMISSION_DENIED" {
			t.Fatalf("expected permission denied, got %#v", result)
		}
		return
	}
	t.Fatal("AI origin unexpectedly had filesystem write permission")
}

func TestInvalidInputAndDiscovery(t *testing.T) {
	runtime := testRuntime(t)
	invalid := runtime.Execute("project.create", map[string]interface{}{"name": ""}, string(capabilities.OriginHuman))
	if invalid.Status != "succeeded" || invalid.Error != nil {
		if invalid.Error == nil || invalid.Error.Code != "INVALID_INPUT" {
			t.Fatalf("expected invalid input, got %#v", invalid)
		}
	} else {
		t.Fatal("empty project name should fail")
	}
	discovery := runtime.Execute("capability.list", map[string]interface{}{}, string(capabilities.OriginHuman))
	if discovery.Status != "succeeded" {
		t.Fatal(discovery.Error)
	}
}

func TestAuditEventIsPersisted(t *testing.T) {
	runtime := testRuntime(t)
	runtime.Execute("project.list", map[string]interface{}{}, string(capabilities.OriginHuman))
	var count int
	if err := runtime.DB.QueryRow("SELECT COUNT(*) FROM audit_events").Scan(&count); err != nil {
		t.Fatal(err)
	}
	if count < 1 {
		t.Fatal("expected audit event")
	}
}

func TestRestoreRebindKeepsRuntimeServicesUsable(t *testing.T) {
	root := t.TempDir()
	store, err := persistence.Open(filepath.Join(root, "active.db"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { store.DB.Close() })
	filesystem, err := infrastructure.NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	runtime := NewRuntime(store, filesystem, "Vora Test", "0.26.11", "v3.1.0")
	backupRoot := filepath.Join(root, "backups")
	manager, err := persistence.NewBackupManager(store, backupRoot)
	if err != nil {
		t.Fatal(err)
	}
	record, err := manager.Create()
	if err != nil {
		t.Fatal(err)
	}
	created := runtime.Execute("project.create", map[string]interface{}{"name": "after-backup"}, string(capabilities.OriginHuman))
	if created.Status != "succeeded" {
		t.Fatal(created.Error)
	}
	restored, err := manager.Restore(record.Path)
	if err != nil {
		t.Fatal(err)
	}
	if !restored {
		t.Fatal("restore should succeed")
	}
	projects := runtime.Execute("project.list", map[string]interface{}{}, string(capabilities.OriginHuman))
	if projects.Status != "succeeded" {
		t.Fatalf("runtime should remain usable after restore: %#v", projects.Error)
	}
}

func TestProjectFilesystemIsolationAndReopen(t *testing.T) {
	root := t.TempDir()
	store, err := persistence.Open(filepath.Join(root, "active.db"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { store.DB.Close() })
	filesystem, err := infrastructure.NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	runtime := NewRuntime(store, filesystem, "Vora Test", "0.26.11", "v3.1.0")

	projectA := runtime.Execute("project.create", map[string]interface{}{"name": "Project A"}, string(capabilities.OriginHuman))
	projectB := runtime.Execute("project.create", map[string]interface{}{"name": "Project B"}, string(capabilities.OriginHuman))
	if projectA.Status != "succeeded" || projectB.Status != "succeeded" {
		t.Fatalf("project creation failed: A=%#v B=%#v", projectA.Error, projectB.Error)
	}
	idA := projectA.Data.(map[string]interface{})["project"].(domain.Project).ID
	idB := projectB.Data.(map[string]interface{})["project"].(domain.Project).ID
	rootA, err := runtime.ProjectRepo.GetWorkspaceRoot(idA)
	if err != nil {
		t.Fatal(err)
	}
	rootB, err := runtime.ProjectRepo.GetWorkspaceRoot(idB)
	if err != nil {
		t.Fatal(err)
	}
	if rootA == rootB {
		t.Fatal("projects must have distinct workspace roots")
	}
	if err := os.WriteFile(filepath.Join(rootA, "a.txt"), []byte("a"), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := filesystem.ResolveProject(idB, filepath.Join(rootA, "a.txt")); err == nil {
		t.Fatal("project B must not resolve project A's workspace")
	}
	reopened := NewRuntime(store, filesystem, "Vora Test", "0.26.11", "v3.1.0")
	reopenedRoot, err := reopened.ProjectRepo.GetWorkspaceRoot(idA)
	if err != nil || reopenedRoot != rootA {
		t.Fatalf("project root did not reopen: root=%q err=%v", reopenedRoot, err)
	}
}
