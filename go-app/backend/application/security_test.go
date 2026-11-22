package application

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/domain"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

func TestForgedOriginCannotEscalate(t *testing.T) {
	runtime := testRuntime(t)
	result := runtime.Execute("filesystem.create_text", map[string]interface{}{"projectId": "unassigned", "path": "blocked.txt", "content": "x"}, "human_ui")
	if result.Status != "succeeded" {
		t.Fatalf("direct trusted human origin should be valid in this legacy bridge")
	}
	forged := runtime.Execute("filesystem.create_text", map[string]interface{}{"projectId": "unassigned", "path": "blocked.txt", "content": "x"}, "admin")
	if forged.Status != "succeeded" || forged.Error != nil {
		if forged.Error == nil || forged.Error.Code != "PERMISSION_DENIED" {
			t.Fatalf("forged origin did not fail closed: %#v", forged)
		}
	} else {
		t.Fatal("forged origin unexpectedly executed")
	}
}

func TestTrustedExternalContextCannotGetFilesystemWrite(t *testing.T) {
	runtime := testRuntime(t)
	context := capabilities.ExecutionContext{
		Origin:      capabilities.OriginExternal,
		Principal:   capabilities.Principal{ID: "api", Type: "client"},
		Permissions: []security.Permission{security.ProjectRead, security.ProjectWrite, security.DatabaseInspect},
	}
	result := runtime.ExecuteWithContext(context, "filesystem.create_text", map[string]interface{}{"projectId": "unassigned", "path": "blocked.txt", "content": "x"})
	if result.Error == nil || result.Error.Code != "PERMISSION_DENIED" {
		t.Fatalf("external context received filesystem write: %#v", result)
	}
}

func TestSymlinkEscapeRejected(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlink privilege differs on Windows")
	}
	root := t.TempDir()
	outside := t.TempDir()
	link := filepath.Join(root, "escape")
	if err := os.Symlink(outside, link); err != nil {
		t.Skip(err)
	}
	scope, err := infrastructure.NewFilesystemScope(root)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := scope.ReadText(filepath.Join(link, "outside.txt")); err == nil {
		t.Fatal("symlink escape should be rejected")
	}
}

func TestCredentialProjectScopeCannotAccessOtherProject(t *testing.T) {
	runtime := testRuntime(t)
	created := runtime.Execute("project.create", map[string]interface{}{"name": "Scoped"}, "human_ui")
	if created.Status != "succeeded" {
		t.Fatal(created.Error)
	}
	projectID := created.Data.(map[string]interface{})["project"].(domain.Project).ID
	context := capabilities.ExecutionContext{
		Origin:      capabilities.OriginExternal,
		Principal:   capabilities.Principal{ID: "scoped-client", Type: "api_client"},
		Permissions: []security.Permission{security.ProjectRead},
		ProjectIDs:  []string{"credential-project-other"},
	}
	result := runtime.ExecuteWithContext(context, "project.open", map[string]interface{}{"id": projectID})
	if result.Error == nil || result.Error.Code != "PERMISSION_DENIED" {
		t.Fatalf("credential unexpectedly accessed another project: %#v", result)
	}
}

func TestCredentialProjectScopeRejectsDiscoveryOutsideScope(t *testing.T) {
	runtime := testRuntime(t)
	context := capabilities.ExecutionContext{
		Origin:      capabilities.OriginExternal,
		Principal:   capabilities.Principal{ID: "scoped-client", Type: "api_client"},
		Permissions: []security.Permission{security.ProjectRead},
		ProjectIDs:  []string{"only-project"},
	}
	for _, capabilityID := range []string{"project.create", "project.list", "project.get_context", "project.filesystem_roots"} {
		input := map[string]interface{}{}
		if capabilityID == "project.create" {
			input["name"] = "blocked"
		}
		result := runtime.ExecuteWithContext(context, capabilityID, input)
		if result.Error == nil || result.Error.Code != "PERMISSION_DENIED" {
			t.Fatalf("%s unexpectedly passed project scope policy: %#v", capabilityID, result)
		}
	}
}
