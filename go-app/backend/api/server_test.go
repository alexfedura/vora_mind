package api

import (
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/domain"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

func testServer(t *testing.T) *Server {
	t.Setenv("VORA_CREDENTIAL_TEST_PROVIDER", "1")
	store, err := persistence.Open(t.TempDir() + "/api.db")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { store.DB.Close() })
	filesystem, err := infrastructure.NewFilesystemScope(filepath.Dir(store.Database))
	if err != nil {
		t.Fatal(err)
	}
	runtime := application.NewRuntime(store, filesystem, "Vora Test", "0.26.12", "v3.1.0")
	return NewServer(runtime)
}

func TestHealthDoesNotRequireTokenAndRejectsOtherMethods(t *testing.T) {
	server := testServer(t)
	request := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/health", nil)
	response := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("health status=%d", response.Code)
	}
	post := httptest.NewRequest(http.MethodPost, "http://127.0.0.1/api/v1/health", nil)
	response = httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, post)
	if response.Code != http.StatusMethodNotAllowed {
		t.Fatalf("health POST status=%d", response.Code)
	}
}

func TestProtectedEndpointRejectsMissingAndInvalidToken(t *testing.T) {
	server := testServer(t)
	missing := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/info", nil)
	response := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, missing)
	if response.Code != http.StatusUnauthorized {
		t.Fatalf("missing token status=%d", response.Code)
	}
	invalid := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/info", nil)
	invalid.Header.Set("Authorization", "Bearer wrong")
	response = httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, invalid)
	if response.Code != http.StatusUnauthorized {
		t.Fatalf("invalid token status=%d", response.Code)
	}
}

func TestProtectedEndpointAcceptsValidToken(t *testing.T) {
	server := testServer(t)
	request := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/info", nil)
	request.Header.Set("Authorization", "Bearer "+server.Token())
	response := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("valid token status=%d body=%s", response.Code, response.Body.String())
	}
}

func TestCredentialRevocationAndFilesystemPermission(t *testing.T) {
	server := testServer(t)
	credential, secret, err := server.credentials.Generate([]security.Permission{security.ProjectRead})
	if err != nil {
		t.Fatal(err)
	}
	authorized := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/projects", nil)
	authorized.Header.Set("Authorization", "Bearer "+secret)
	response := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, authorized)
	if response.Code != http.StatusOK {
		t.Fatalf("project read status=%d body=%s", response.Code, response.Body.String())
	}
	forbidden := httptest.NewRequest(http.MethodPost, "http://127.0.0.1/api/v1/projects", strings.NewReader(`{"name":"blocked"}`))
	forbidden.Header.Set("Authorization", "Bearer "+secret)
	forbidden.Header.Set("Content-Type", "application/json")
	response = httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, forbidden)
	if response.Code != http.StatusForbidden {
		t.Fatalf("insufficient scope status=%d", response.Code)
	}
	if err := server.credentials.Revoke(credential.ID); err != nil {
		t.Fatal(err)
	}
	revoked := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/projects", nil)
	revoked.Header.Set("Authorization", "Bearer "+secret)
	response = httptest.NewRecorder()
	server.server.Handler.ServeHTTP(response, revoked)
	if response.Code != http.StatusUnauthorized {
		t.Fatalf("revoked credential status=%d", response.Code)
	}
}

func TestProjectOpenContextAndDeleteApprovalOverHTTP(t *testing.T) {
	server := testServer(t)
	created := server.runtime.Execute("project.create", map[string]interface{}{"name": "HTTP Project"}, string(capabilities.OriginHuman))
	if created.Status != "succeeded" {
		t.Fatal(created.Error)
	}
	project := created.Data.(map[string]interface{})["project"].(domain.Project)
	directOpen := server.runtime.ExecuteWithContext(capabilities.ExecutionContext{
		Origin:      capabilities.OriginExternal,
		Principal:   capabilities.Principal{ID: "test", Type: "api_client"},
		Permissions: []security.Permission{security.ProjectRead, security.ProjectWrite},
	}, "project.open", map[string]interface{}{"id": project.ID})
	if directOpen.Status != "succeeded" {
		t.Fatalf("direct external open failed: %#v", directOpen)
	}
	credential, secret, err := server.credentials.Generate([]security.Permission{security.ProjectRead, security.ProjectWrite})
	if err != nil {
		t.Fatal(err)
	}
	_ = credential

	openRequest := httptest.NewRequest(http.MethodPost, "http://127.0.0.1/api/v1/projects/"+project.ID+"/open", strings.NewReader(`{}`))
	openRequest.Header.Set("Authorization", "Bearer "+secret)
	openRequest.Header.Set("Content-Type", "application/json")
	openResponse := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(openResponse, openRequest)
	if openResponse.Code != http.StatusOK {
		t.Fatalf("project open status=%d body=%s", openResponse.Code, openResponse.Body.String())
	}

	contextRequest := httptest.NewRequest(http.MethodGet, "http://127.0.0.1/api/v1/projects/context", nil)
	contextRequest.Header.Set("Authorization", "Bearer "+secret)
	contextResponse := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(contextResponse, contextRequest)
	if contextResponse.Code != http.StatusOK {
		t.Fatalf("project context status=%d body=%s", contextResponse.Code, contextResponse.Body.String())
	}

	deleteRequest := httptest.NewRequest(http.MethodDelete, "http://127.0.0.1/api/v1/projects/"+project.ID, nil)
	deleteRequest.Header.Set("Authorization", "Bearer "+secret)
	deleteResponse := httptest.NewRecorder()
	server.server.Handler.ServeHTTP(deleteResponse, deleteRequest)
	if deleteResponse.Code != http.StatusBadRequest {
		t.Fatalf("unapproved project delete status=%d body=%s", deleteResponse.Code, deleteResponse.Body.String())
	}
}
