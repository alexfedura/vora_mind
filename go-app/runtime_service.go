package main

import (
	"context"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	wailsapp "github.com/wailsapp/wails/v3/pkg/application"
)

type RuntimeService struct {
	runtime *application.Runtime
	store   *persistence.Store
}

func NewRuntimeService(runtime *application.Runtime, store *persistence.Store) *RuntimeService {
	return &RuntimeService{runtime: runtime, store: store}
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
