package persistence

import (
	"database/sql"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/domain"
)

var ErrProjectNotFound = errors.New("project not found")

type ProjectRepository struct {
	DB *sql.DB
}

func (r *ProjectRepository) Create(project domain.Project) error {
	_, err := r.DB.Exec(`
		INSERT INTO projects(id, name, description, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?)`,
		project.ID, project.Name, project.Description,
		project.CreatedAt.UTC().Format(time.RFC3339Nano),
		project.UpdatedAt.UTC().Format(time.RFC3339Nano),
	)
	return err
}

func (r *ProjectRepository) List() ([]domain.Project, error) {
	rows, err := r.DB.Query(`SELECT id, name, description, created_at, updated_at FROM projects ORDER BY created_at ASC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	result := make([]domain.Project, 0)
	for rows.Next() {
		project, err := scanProject(rows)
		if err != nil {
			return nil, err
		}
		result = append(result, project)
	}
	return result, rows.Err()
}

func (r *ProjectRepository) Get(id string) (domain.Project, error) {
	row := r.DB.QueryRow(`SELECT id, name, description, created_at, updated_at FROM projects WHERE id = ?`, id)
	return scanProject(row)
}

func (r *ProjectRepository) Update(id, name, description string) (domain.Project, error) {
	now := time.Now().UTC()
	result, err := r.DB.Exec(`
		UPDATE projects SET name = ?, description = ?, updated_at = ?
		WHERE id = ?`,
		name, description, now.Format(time.RFC3339Nano), id,
	)
	if err != nil {
		return domain.Project{}, err
	}
	affected, err := result.RowsAffected()
	if err != nil {
		return domain.Project{}, err
	}
	if affected == 0 {
		return domain.Project{}, ErrProjectNotFound
	}
	return r.Get(id)
}

func (r *ProjectRepository) RegisterWorkspaceRoot(projectID, root string) (string, error) {
	projectID = strings.TrimSpace(projectID)
	root = strings.TrimSpace(root)
	if projectID == "" {
		return "", errors.New("project ID is required")
	}
	if root == "" {
		return "", errors.New("workspace root is required")
	}
	if _, err := r.Get(projectID); err != nil {
		return "", err
	}
	absolute, err := filepath.Abs(root)
	if err != nil {
		return "", err
	}
	absolute = filepath.Clean(absolute)
	if err := os.MkdirAll(absolute, 0o700); err != nil {
		return "", err
	}
	now := time.Now().UTC().Format(time.RFC3339Nano)
	_, err = r.DB.Exec(`
		INSERT INTO project_filesystem_roots(project_id,workspace_root,created_at,updated_at)
		VALUES(?,?,?,?)
		ON CONFLICT(project_id) DO UPDATE SET workspace_root=excluded.workspace_root,updated_at=excluded.updated_at`,
		projectID, absolute, now, now)
	if err != nil {
		return "", err
	}
	return absolute, nil
}

func (r *ProjectRepository) GetWorkspaceRoot(projectID string) (string, error) {
	projectID = strings.TrimSpace(projectID)
	if projectID == "" {
		return "", errors.New("project ID is required")
	}
	var root string
	err := r.DB.QueryRow(`SELECT workspace_root FROM project_filesystem_roots WHERE project_id=?`, projectID).Scan(&root)
	if errors.Is(err, sql.ErrNoRows) {
		return "", errors.New("project workspace root is not registered")
	}
	if err != nil {
		return "", err
	}
	return filepath.Clean(root), nil
}

func (r *ProjectRepository) Delete(projectID string) error {
	projectID = strings.TrimSpace(projectID)
	if projectID == "" {
		return errors.New("project ID is required")
	}
	var count int
	if err := r.DB.QueryRow(`SELECT COUNT(*) FROM projects WHERE id=?`, projectID).Scan(&count); err != nil {
		return err
	}
	if count != 1 {
		return ErrProjectNotFound
	}
	tx, err := r.DB.Begin()
	if err != nil {
		return err
	}
	if _, err := tx.Exec(`DELETE FROM project_filesystem_roots WHERE project_id=?`, projectID); err != nil {
		tx.Rollback()
		return err
	}
	if _, err := tx.Exec(`DELETE FROM projects WHERE id=?`, projectID); err != nil {
		tx.Rollback()
		return err
	}
	return tx.Commit()
}

type scanner interface {
	Scan(dest ...interface{}) error
}

func scanProject(row scanner) (domain.Project, error) {
	var project domain.Project
	var created, updated string
	if err := row.Scan(&project.ID, &project.Name, &project.Description, &created, &updated); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return domain.Project{}, ErrProjectNotFound
		}
		return domain.Project{}, err
	}
	project.CreatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(created))
	project.UpdatedAt, _ = time.Parse(time.RFC3339Nano, strings.TrimSpace(updated))
	return project, nil
}
