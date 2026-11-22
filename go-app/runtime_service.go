package main

import (
	"context"
	"encoding/json"
	"path/filepath"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
	wailsapp "github.com/wailsapp/wails/v3/pkg/application"
)

type RuntimeService struct {
	runtime   *application.Runtime
	store     *persistence.Store
	backups   *persistence.BackupManager
	apiServer interface {
		Address() string
		Running() bool
		AuthenticationStatus() string
		Start(context.Context) error
		Stop() error
	}
	credentials *security.CredentialManager
}

func NewRuntimeService(runtime *application.Runtime, store *persistence.Store) *RuntimeService {
	service := &RuntimeService{runtime: runtime, store: store, credentials: security.NewCredentialManager(store.DB)}
	backupRoot := filepath.Join(filepath.Dir(store.Database), "backups")
	if manager, err := persistence.NewBackupManager(store, backupRoot); err == nil {
		service.backups = manager
	}
	return service
}

func (s *RuntimeService) ServiceStartup(ctx context.Context, options wailsapp.ServiceOptions) error {
	return nil
}

func (s *RuntimeService) ServiceShutdown() error {
	if s.store == nil || s.store.DB == nil {
		return nil
	}
	return s.store.DB.Close()
}

func (s *RuntimeService) Execute(capabilityID string, input map[string]interface{}, origin string) capabilities.Result {
	return s.runtime.Execute(capabilityID, input, origin)
}

func (s *RuntimeService) GetApplicationInfo() capabilities.Result {
	return s.Execute("system.get_info", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) GetDatabaseStatus() capabilities.Result {
	return s.Execute("database.get_status", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ListProjects() capabilities.Result {
	return s.Execute("project.list", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) CreateProject(name, description string) capabilities.Result {
	return s.Execute("project.create", map[string]interface{}{"name": name, "description": description}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ListCapabilities() capabilities.Result {
	return s.Execute("capability.list", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) GetProject(id string) capabilities.Result {
	return s.Execute("project.get", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) OpenProject(id string) capabilities.Result {
	return s.Execute("project.open", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) GetProjectContext() capabilities.Result {
	return s.Execute("project.get_context", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ListProjectFilesystemRoots() capabilities.Result {
	return s.Execute("project.filesystem_roots", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) UpdateProjectFilesystemRoot(id, root string) capabilities.Result {
	return s.Execute("project.filesystem_root_update", map[string]interface{}{"id": id, "root": root}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) RemoveProjectFilesystemRoot(id string) capabilities.Result {
	input := map[string]interface{}{"id": id}
	context := capabilities.ExecutionContext{
		Origin:        capabilities.OriginHuman,
		Principal:     capabilities.Principal{ID: "wails_human", Type: "human"},
		Permissions:   []security.Permission{security.ProjectWrite},
		ProjectScope:  id,
		CorrelationID: "wails_project_root_remove_" + id,
		Approved:      true,
	}
	return s.runtime.ExecuteWithContext(context, "project.filesystem_root_remove", input)
}

func (s *RuntimeService) ResolveProjectFilesystemPath(id, path string) capabilities.Result {
	return s.Execute("project.filesystem_root_resolve", map[string]interface{}{"id": id, "path": path}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) UpdateProject(id, name, description string) capabilities.Result {
	return s.Execute("project.update", map[string]interface{}{"id": id, "name": name, "description": description}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) DeleteProject(id string) capabilities.Result {
	input := map[string]interface{}{"id": id}
	context := capabilities.ExecutionContext{
		Origin:        capabilities.OriginHuman,
		Principal:     capabilities.Principal{ID: "wails_human", Type: "human"},
		Permissions:   []security.Permission{security.ProjectWrite, security.ProjectRead},
		ProjectScope:  id,
		CorrelationID: "wails_project_delete_" + id,
		Approved:      true,
	}
	return s.runtime.ExecuteWithContext(context, "project.delete", input)
}

func (s *RuntimeService) GetAPIStatus() map[string]interface{} {
	if s.apiServer == nil {
		return map[string]interface{}{"running": false, "address": "", "authentication": "not configured"}
	}
	return map[string]interface{}{
		"running":        s.apiServer.Running(),
		"address":        s.apiServer.Address(),
		"authentication": s.apiServer.AuthenticationStatus(),
	}
}

func (s *RuntimeService) StartAPI() map[string]interface{} {
	if s.apiServer == nil {
		return map[string]interface{}{"status": "failed", "error": "API is not configured"}
	}
	if s.apiServer.Running() {
		return s.GetAPIStatus()
	}
	if err := s.apiServer.Start(context.Background()); err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	status := s.GetAPIStatus()
	status["status"] = "succeeded"
	return status
}

func (s *RuntimeService) StopAPI() map[string]interface{} {
	if s.apiServer == nil {
		return map[string]interface{}{"status": "succeeded", "running": false}
	}
	if err := s.apiServer.Stop(); err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "running": false}
}

func (s *RuntimeService) ListCredentials() map[string]interface{} {
	if s.credentials == nil {
		return map[string]interface{}{"status": "failed", "error": "credential manager is unavailable"}
	}
	items, err := s.credentials.List()
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": "could not list credentials"}
	}
	return map[string]interface{}{"status": "succeeded", "credentials": items}
}

func (s *RuntimeService) CreateCredential(name string, permissionsJSON string, projectIDsJSON string) map[string]interface{} {
	if s.credentials == nil {
		return map[string]interface{}{"status": "failed", "error": "credential manager is unavailable"}
	}
	var permissions []security.Permission
	if err := json.Unmarshal([]byte(permissionsJSON), &permissions); err != nil {
		return map[string]interface{}{"status": "failed", "error": "permissions must be a JSON array"}
	}
	var projectIDs []string
	if err := json.Unmarshal([]byte(projectIDsJSON), &projectIDs); err != nil {
		return map[string]interface{}{"status": "failed", "error": "project IDs must be a JSON array"}
	}
	allowed := map[security.Permission]bool{
		security.ProjectRead: true, security.ProjectWrite: true,
		security.DatabaseInspect: true, security.FilesystemRead: true,
		security.CapabilitiesDiscover: true, security.WorkflowsRead: true,
	}
	for _, permission := range permissions {
		if !allowed[permission] {
			return map[string]interface{}{"status": "failed", "error": "requested permission is not allowed"}
		}
	}
	credential, secret, err := s.credentials.GenerateNamed(name, permissions, projectIDs, nil)
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "credential": credential, "secret": secret}
}

func (s *RuntimeService) RevokeCredential(id string) map[string]interface{} {
	if s.credentials == nil {
		return map[string]interface{}{"status": "failed", "error": "credential manager is unavailable"}
	}
	if err := s.credentials.Revoke(id); err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded"}
}

func (s *RuntimeService) RotateCredential(id string) map[string]interface{} {
	if s.credentials == nil {
		return map[string]interface{}{"status": "failed", "error": "credential manager is unavailable"}
	}
	credential, secret, err := s.credentials.Rotate(id)
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "credential": credential, "secret": secret}
}

func (s *RuntimeService) ListBackups() map[string]interface{} {
	if s.backups == nil {
		return map[string]interface{}{"status": "failed", "error": "backup manager is unavailable"}
	}
	backups, err := s.backups.List()
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": "could not list backups"}
	}
	return map[string]interface{}{"status": "succeeded", "backups": backups}
}

func (s *RuntimeService) CreateBackup() map[string]interface{} {
	if s.backups == nil {
		return map[string]interface{}{"status": "failed", "error": "backup manager is unavailable"}
	}
	record, err := s.backups.Create()
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "backup": record}
}

func (s *RuntimeService) ValidateBackup(path string) map[string]interface{} {
	if s.backups == nil {
		return map[string]interface{}{"status": "failed", "error": "backup manager is unavailable"}
	}
	valid, err := s.backups.PreflightRestore(path)
	if err != nil {
		return map[string]interface{}{"status": "failed", "valid": false, "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "valid": valid}
}

func (s *RuntimeService) RestoreBackup(path string) map[string]interface{} {
	if s.backups == nil {
		return map[string]interface{}{"status": "failed", "error": "backup manager is unavailable"}
	}
	restored, err := s.backups.Restore(path)
	if err != nil {
		return map[string]interface{}{"status": "failed", "error": err.Error()}
	}
	return map[string]interface{}{"status": "succeeded", "restored": restored}
}

func (s *RuntimeService) CreateWorkflow(name, description, projectScope string, stepsJSON string) capabilities.Result {
	input := map[string]interface{}{"name": name, "description": description, "projectScope": projectScope}
	var steps []interface{}
	if err := json.Unmarshal([]byte(stepsJSON), &steps); err == nil {
		input["steps"] = steps
	} else {
		input["steps"] = []interface{}{}
	}
	return s.Execute("workflow.create", input, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ListWorkflows() capabilities.Result {
	return s.Execute("workflow.list", map[string]interface{}{}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) GetWorkflow(id string) capabilities.Result {
	return s.Execute("workflow.get", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) StartWorkflow(id string) capabilities.Result {
	return s.Execute("workflow.start", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) CancelWorkflow(id string) capabilities.Result {
	return s.Execute("workflow.cancel", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ResumeWorkflow(id string) capabilities.Result {
	return s.Execute("workflow.resume", map[string]interface{}{"id": id}, string(capabilities.OriginHuman))
}

func (s *RuntimeService) ApproveWorkflowStep(id string, approved bool) capabilities.Result {
	return s.Execute("workflow.approve_step", map[string]interface{}{"id": id, "approved": approved}, string(capabilities.OriginHuman))
}
