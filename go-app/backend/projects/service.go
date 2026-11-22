package projects

import (
	"database/sql"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/domain"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
)

const activeProjectIdentityKey = "active_project_id"

type Service struct {
	DB         *sql.DB
	Repository *persistence.ProjectRepository
	Filesystem *infrastructure.FilesystemScope
}

func NewService(db *sql.DB, repository *persistence.ProjectRepository, filesystem *infrastructure.FilesystemScope) *Service {
	return &Service{DB: db, Repository: repository, Filesystem: filesystem}
}

func (s *Service) Create(project domain.Project, defaultRoot string) (domain.Project, error) {
	if _, err := s.Filesystem.ValidateProjectRoot(defaultRoot); err != nil {
		return domain.Project{}, err
	}
	if err := s.Repository.Create(project); err != nil {
		return domain.Project{}, err
	}
	root, err := s.Repository.RegisterWorkspaceRoot(project.ID, defaultRoot)
	if err != nil {
		_ = s.Repository.Delete(project.ID)
		return domain.Project{}, err
	}
	if err := s.Filesystem.SetProjectRoot(project.ID, root); err != nil {
		_ = s.Repository.Delete(project.ID)
		return domain.Project{}, err
	}
	return project, nil
}

func (s *Service) List() ([]domain.Project, error) {
	return s.Repository.List()
}

func (s *Service) Get(projectID string) (domain.Project, error) {
	return s.Repository.Get(projectID)
}

func (s *Service) Update(projectID, name, description string) (domain.Project, error) {
	return s.Repository.Update(projectID, name, description)
}

func (s *Service) Delete(projectID string) error {
	projectID = strings.TrimSpace(projectID)
	if err := s.Repository.Delete(projectID); err != nil {
		return err
	}
	var active string
	err := s.DB.QueryRow(`SELECT value FROM application_identity WHERE key=?`, activeProjectIdentityKey).Scan(&active)
	if err == nil && strings.TrimSpace(active) == projectID {
		_, _ = s.DB.Exec(`DELETE FROM application_identity WHERE key=?`, activeProjectIdentityKey)
	}
	s.Filesystem.RemoveProjectRoot(projectID)
	return nil
}

func (s *Service) Open(projectID string) (domain.Project, error) {
	projectID = strings.TrimSpace(projectID)
	project, err := s.Repository.Get(projectID)
	if err != nil {
		return domain.Project{}, err
	}
	root, err := s.Repository.GetWorkspaceRoot(projectID)
	if err != nil {
		return domain.Project{}, err
	}
	if err := s.Filesystem.SetProjectRoot(projectID, root); err != nil {
		return domain.Project{}, err
	}
	if _, err := s.DB.Exec(`
		INSERT INTO application_identity(key,value) VALUES(?,?)
		ON CONFLICT(key) DO UPDATE SET value=excluded.value`,
		activeProjectIdentityKey, projectID); err != nil {
		return domain.Project{}, err
	}
	return project, nil
}

func (s *Service) GetContext() (map[string]interface{}, error) {
	projectID := ""
	err := s.DB.QueryRow(`SELECT value FROM application_identity WHERE key=?`, activeProjectIdentityKey).Scan(&projectID)
	if err != nil && !errors.Is(err, sql.ErrNoRows) {
		return nil, err
	}
	projectID = strings.TrimSpace(projectID)
	if projectID == "" {
		return map[string]interface{}{"active": false, "project": nil, "workspaceRoot": nil}, nil
	}
	project, err := s.Repository.Get(projectID)
	if errors.Is(err, persistence.ErrProjectNotFound) {
		_, _ = s.DB.Exec(`DELETE FROM application_identity WHERE key=?`, activeProjectIdentityKey)
		return map[string]interface{}{"active": false, "project": nil, "workspaceRoot": nil}, nil
	}
	if err != nil {
		return nil, err
	}
	root, err := s.Repository.GetWorkspaceRoot(projectID)
	if err != nil {
		return nil, err
	}
	return map[string]interface{}{"active": true, "project": project, "workspaceRoot": root}, nil
}

func (s *Service) ListRoots() ([]domain.WorkspaceRoot, error) {
	rows, err := s.DB.Query(`
		SELECT project_id,workspace_root,created_at,updated_at
		FROM project_filesystem_roots
		ORDER BY created_at ASC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	result := make([]domain.WorkspaceRoot, 0)
	for rows.Next() {
		var item domain.WorkspaceRoot
		var created, updated string
		if err := rows.Scan(&item.ProjectID, &item.Root, &created, &updated); err != nil {
			return nil, err
		}
		item.CreatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(created))
		item.UpdatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(updated))
		result = append(result, item)
	}
	return result, rows.Err()
}

func (s *Service) UpdateRoot(projectID, root string) (domain.WorkspaceRoot, error) {
	projectID = strings.TrimSpace(projectID)
	if _, err := s.Filesystem.ValidateProjectRoot(root); err != nil {
		return domain.WorkspaceRoot{}, err
	}
	absolute, err := s.Repository.RegisterWorkspaceRoot(projectID, root)
	if err != nil {
		return domain.WorkspaceRoot{}, err
	}
	if err := s.Filesystem.SetProjectRoot(projectID, absolute); err != nil {
		return domain.WorkspaceRoot{}, err
	}
	var created, updated string
	err = s.DB.QueryRow(`
		SELECT workspace_root,created_at,updated_at
		FROM project_filesystem_roots WHERE project_id=?`, projectID).Scan(&absolute, &created, &updated)
	if err != nil {
		return domain.WorkspaceRoot{}, err
	}
	item := domain.WorkspaceRoot{ProjectID: projectID, Root: absolute}
	item.CreatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(created))
	item.UpdatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(updated))
	return item, nil
}

func (s *Service) RemoveRoot(projectID string) (map[string]interface{}, error) {
	projectID = strings.TrimSpace(projectID)
	if _, err := s.Repository.Get(projectID); err != nil {
		return nil, err
	}
	tx, err := s.DB.Begin()
	if err != nil {
		return nil, err
	}
	result, err := tx.Exec(`DELETE FROM project_filesystem_roots WHERE project_id=?`, projectID)
	if err != nil {
		tx.Rollback()
		return nil, err
	}
	if err := tx.Commit(); err != nil {
		return nil, err
	}
	affected, err := result.RowsAffected()
	if err != nil {
		return nil, err
	}
	if affected == 0 {
		return nil, fmt.Errorf("project workspace root is not registered")
	}
	s.Filesystem.RemoveProjectRoot(projectID)
	return map[string]interface{}{"projectId": projectID, "removed": true, "workspaceFilesPreserved": true}, nil
}

func (s *Service) ResolveRoot(projectID, path string) (map[string]interface{}, error) {
	resolved, err := s.Filesystem.ResolveProject(projectID, path)
	if err != nil {
		return nil, err
	}
	return map[string]interface{}{"projectId": projectID, "path": resolved}, nil
}
