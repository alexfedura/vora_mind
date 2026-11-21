package audit

import (
	"database/sql"
	"encoding/json"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type ExecutionOrigin string

const (
	OriginHuman    ExecutionOrigin = "human_ui"
	OriginInternal ExecutionOrigin = "internal"
	OriginAI       ExecutionOrigin = "ai_agent"
	OriginExternal ExecutionOrigin = "external_api"
)

type Event struct {
	ExecutionID  string                `json:"executionId"`
	CapabilityID string                `json:"capabilityId"`
	Timestamp    time.Time             `json:"timestamp"`
	Origin       ExecutionOrigin       `json:"origin"`
	ProjectScope string                `json:"projectScope,omitempty"`
	Status       string                `json:"status"`
	DurationMS   int64                 `json:"durationMs"`
	Error        *ErrorDetails         `json:"error,omitempty"`
	Permissions  []security.Permission `json:"permissions,omitempty"`
}

type ErrorDetails struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

type Store interface {
	Record(event Event) error
}

type SQLiteStore struct {
	DB *sql.DB
}

func (s *SQLiteStore) Record(event Event) error {
	var errorJSON []byte
	if event.Error != nil {
		errorJSON, _ = json.Marshal(event.Error)
	}
	var permissionsJSON []byte
	if event.Permissions != nil {
		permissionsJSON, _ = json.Marshal(event.Permissions)
	}
	_, err := s.DB.Exec(`
		INSERT INTO audit_events (
			execution_id, capability_id, timestamp, origin, project_scope,
			status, duration_ms, error_json, permissions_json
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		event.ExecutionID, event.CapabilityID, event.Timestamp.UTC().Format(time.RFC3339Nano),
		event.Origin, event.ProjectScope, event.Status, event.DurationMS,
		string(errorJSON), string(permissionsJSON),
	)
	return err
}
