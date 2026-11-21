package persistence

import (
	"database/sql"
	"errors"
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
