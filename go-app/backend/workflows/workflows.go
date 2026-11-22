package workflows

import (
	"database/sql"
	"encoding/json"
	"errors"
	"strings"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type Status string

const (
	Draft            Status = "Draft"
	Pending          Status = "Pending"
	Running          Status = "Running"
	AwaitingApproval Status = "AwaitingApproval"
	Succeeded        Status = "Succeeded"
	Failed           Status = "Failed"
	Cancelled        Status = "Cancelled"
)

type Step struct {
	ID               string                        `json:"id"`
	WorkflowID       string                        `json:"workflowId"`
	Index            int                           `json:"index"`
	CapabilityID     string                        `json:"capabilityId"`
	Input            map[string]interface{}        `json:"input"`
	Status           Status                        `json:"status"`
	ExecutionID      string                        `json:"executionId,omitempty"`
	Result           interface{}                   `json:"result,omitempty"`
	Error            *capabilities.StructuredError `json:"error,omitempty"`
	RetryCount       int                           `json:"retryCount"`
	MaxRetries       int                           `json:"maxRetries"`
	ApprovalRequired bool                          `json:"approvalRequired"`
	ApprovalStatus   string                        `json:"approvalStatus"`
	StartedAt        *time.Time                    `json:"startedAt,omitempty"`
	EndedAt          *time.Time                    `json:"endedAt,omitempty"`
	ExecutionPolicy  string                        `json:"executionPolicy"`
	IdempotencyKey   string                        `json:"idempotencyKey"`
	RecoveryState    string                        `json:"recoveryState"`
}

type RecoveryState string

const (
	RecoveryNone     RecoveryState = "none"
	RecoveryRetry    RecoveryState = "retry"
	RecoveryApproval RecoveryState = "approval_required"
	RecoveryManual   RecoveryState = "manual_resolution_required"
)

type Workflow struct {
	ID                    string                        `json:"id"`
	Name                  string                        `json:"name"`
	Description           string                        `json:"description"`
	ProjectScope          string                        `json:"projectScope"`
	Creator               string                        `json:"creator"`
	Origin                capabilities.ExecutionOrigin  `json:"origin"`
	Status                Status                        `json:"status"`
	CurrentStep           int                           `json:"currentStep"`
	CancellationRequested bool                          `json:"cancellationRequested"`
	ApprovalState         string                        `json:"approvalState"`
	Error                 *capabilities.StructuredError `json:"error,omitempty"`
	CreatedAt             time.Time                     `json:"createdAt"`
	UpdatedAt             time.Time                     `json:"updatedAt"`
	Steps                 []Step                        `json:"steps"`
}

type Executor interface {
	ExecuteWithContext(context capabilities.ExecutionContext, capabilityID string, input map[string]interface{}) capabilities.Result
}

type Engine struct {
	DB       *sql.DB
	Executor Executor
}

func NewEngine(db *sql.DB, executor Executor) *Engine {
	return &Engine{DB: db, Executor: executor}
}

func newWorkflowID() string {
	return "workflow_" + randomHex()
}

func newStepID() string {
	return "step_" + randomHex()
}

func randomHex() string {
	var value [16]byte
	for i := range value {
		value[i] = byte(time.Now().UnixNano() >> (i % 8 * 8))
	}
	return strings.ReplaceAll(time.Unix(0, time.Now().UnixNano()).UTC().Format("150405.000000000"), ".", "") + hexSuffix(value[:])
}

func hexSuffix(value []byte) string {
	const digits = "0123456789abcdef"
	out := make([]byte, len(value)*2)
	for i, b := range value {
		out[i*2] = digits[b>>4]
		out[i*2+1] = digits[b&0xf]
	}
	return string(out)
}

func (e *Engine) Create(name, description, projectScope, creator string, origin capabilities.ExecutionOrigin, steps []map[string]interface{}) (Workflow, error) {
	now := time.Now().UTC()
	workflow := Workflow{
		ID: newWorkflowID(), Name: strings.TrimSpace(name), Description: strings.TrimSpace(description),
		ProjectScope: strings.TrimSpace(projectScope), Creator: strings.TrimSpace(creator),
		Origin: origin, Status: Draft, CurrentStep: 0, CreatedAt: now, UpdatedAt: now,
		ApprovalState: "none", Steps: make([]Step, 0, len(steps)),
	}
	if workflow.Name == "" {
		return Workflow{}, errors.New("workflow name is required")
	}
	tx, err := e.DB.Begin()
	if err != nil {
		return Workflow{}, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec(`
		INSERT INTO workflows(id,name,description,project_scope,creator,origin,status,current_step,cancellation_requested,approval_state,error_json,created_at,updated_at)
		VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)`,
		workflow.ID, workflow.Name, workflow.Description, workflow.ProjectScope, workflow.Creator, workflow.Origin,
		workflow.Status, 0, 0, workflow.ApprovalState, "", now.Format(time.RFC3339Nano), now.Format(time.RFC3339Nano),
	); err != nil {
		return Workflow{}, err
	}
	for index, raw := range steps {
		capabilityID, _ := raw["capabilityId"].(string)
		input, _ := raw["input"].(map[string]interface{})
		if strings.TrimSpace(capabilityID) == "" {
			return Workflow{}, errors.New("workflow step capabilityId is required")
		}
		if input == nil {
			input = map[string]interface{}{}
		}
		approvalRequired, _ := raw["approvalRequired"].(bool)
		executionPolicy, _ := raw["executionPolicy"].(string)
		if executionPolicy == "" {
			executionPolicy = "unknown"
		}
		idempotencyKey, _ := raw["idempotencyKey"].(string)
		recoveryState, _ := raw["recoveryState"].(string)
		if recoveryState == "" {
			recoveryState = string(RecoveryNone)
		}
		maxRetries, _ := raw["maxRetries"].(float64)
		step := Step{
			ID: newStepID(), WorkflowID: workflow.ID, Index: index, CapabilityID: capabilityID,
			Input: input, Status: Draft, MaxRetries: int(maxRetries),
			ApprovalRequired: approvalRequired, ApprovalStatus: "not_required",
			ExecutionPolicy: executionPolicy, IdempotencyKey: idempotencyKey, RecoveryState: recoveryState,
		}
		if approvalRequired {
			step.ApprovalStatus = "pending"
		}
		inputJSON, _ := json.Marshal(input)
		if _, err = tx.Exec(`
			INSERT INTO workflow_steps(id,workflow_id,step_index,capability_id,input_json,status,retry_count,max_retries,approval_required,approval_status,execution_policy,idempotency_key,recovery_state)
			VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)`,
			step.ID, step.WorkflowID, step.Index, step.CapabilityID, string(inputJSON), step.Status,
			0, step.MaxRetries, boolInt(approvalRequired), step.ApprovalStatus, step.ExecutionPolicy, step.IdempotencyKey, step.RecoveryState,
		); err != nil {
			return Workflow{}, err
		}
		workflow.Steps = append(workflow.Steps, step)
	}
	if err = tx.Commit(); err != nil {
		return Workflow{}, err
	}
	return workflow, nil
}

func boolInt(value bool) int {
	if value {
		return 1
	}
	return 0
}

func (e *Engine) Get(id string) (Workflow, error) {
	var workflow Workflow
	var status, origin, created, updated string
	var current, cancelled int
	var approvalState, errorJSON string
	err := e.DB.QueryRow(`
		SELECT id,name,description,project_scope,creator,origin,status,current_step,cancellation_requested,approval_state,error_json,created_at,updated_at
		FROM workflows WHERE id=?`, id,
	).Scan(&workflow.ID, &workflow.Name, &workflow.Description, &workflow.ProjectScope, &workflow.Creator, &origin, &status, &current, &cancelled, &approvalState, &errorJSON, &created, &updated)
	if errors.Is(err, sql.ErrNoRows) {
		return Workflow{}, errors.New("workflow not found")
	}
	if err != nil {
		return Workflow{}, err
	}
	workflow.Origin = capabilities.ExecutionOrigin(origin)
	workflow.Status = Status(status)
	workflow.CurrentStep = current
	workflow.CancellationRequested = cancelled == 1
	workflow.ApprovalState = approvalState
	workflow.CreatedAt, _ = time.Parse(time.RFC3339Nano, created)
	workflow.UpdatedAt, _ = time.Parse(time.RFC3339Nano, updated)
	workflow.Steps, err = e.listSteps(id)
	return workflow, err
}

func (e *Engine) listSteps(id string) ([]Step, error) {
	rows, err := e.DB.Query(`
		SELECT id,step_index,capability_id,input_json,status,execution_id,result_json,error_json,retry_count,max_retries,approval_required,approval_status,started_at,ended_at,execution_policy,idempotency_key,recovery_state
		FROM workflow_steps WHERE workflow_id=? ORDER BY step_index`, id)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	steps := make([]Step, 0)
	for rows.Next() {
		var step Step
		var inputJSON, status, resultJSON, errorJSON, approvalStatus string
		var executionID string
		var approvalRequired int
		var started, ended sql.NullString
		if err := rows.Scan(&step.ID, &step.Index, &step.CapabilityID, &inputJSON, &status, &executionID, &resultJSON, &errorJSON, &step.RetryCount, &step.MaxRetries, &approvalRequired, &approvalStatus, &started, &ended, &step.ExecutionPolicy, &step.IdempotencyKey, &step.RecoveryState); err != nil {
			return nil, err
		}
		_ = json.Unmarshal([]byte(inputJSON), &step.Input)
		_ = json.Unmarshal([]byte(resultJSON), &step.Result)
		if errorJSON != "" {
			var stepError capabilities.StructuredError
			if json.Unmarshal([]byte(errorJSON), &stepError) == nil {
				step.Error = &stepError
			}
		}
		step.WorkflowID = id
		step.Status = Status(status)
		step.ExecutionID = executionID
		step.ApprovalRequired = approvalRequired == 1
		step.ApprovalStatus = approvalStatus
		if started.Valid {
			parsed, _ := time.Parse(time.RFC3339Nano, started.String)
			step.StartedAt = &parsed
		}
		if ended.Valid {
			parsed, _ := time.Parse(time.RFC3339Nano, ended.String)
			step.EndedAt = &parsed
		}
		steps = append(steps, step)
	}
	return steps, rows.Err()
}

func (e *Engine) List() ([]Workflow, error) {
	rows, err := e.DB.Query(`SELECT id FROM workflows ORDER BY created_at DESC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	ids := make([]string, 0)
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		ids = append(ids, id)
	}
	result := make([]Workflow, 0, len(ids))
	for _, id := range ids {
		workflow, err := e.Get(id)
		if err != nil {
			return nil, err
		}
		result = append(result, workflow)
	}
	return result, nil
}

func (e *Engine) Start(id string) (Workflow, error) {
	workflow, err := e.Get(id)
	if err != nil {
		return Workflow{}, err
	}
	if workflow.Status == Cancelled || workflow.Status == Succeeded {
		return workflow, nil
	}
	_, err = e.DB.Exec(`UPDATE workflows SET status=?,updated_at=? WHERE id=?`, Running, time.Now().UTC().Format(time.RFC3339Nano), id)
	if err != nil {
		return Workflow{}, err
	}
	for workflow.CurrentStep < len(workflow.Steps) {
		step := workflow.Steps[workflow.CurrentStep]
		var cancelled int
		_ = e.DB.QueryRow(`SELECT cancellation_requested FROM workflows WHERE id=?`, id).Scan(&cancelled)
		if cancelled == 1 {
			return e.Cancel(id)
		}
		if step.ApprovalRequired && step.ApprovalStatus == "pending" {
			_, _ = e.DB.Exec(`UPDATE workflows SET status=?,approval_state=?,updated_at=? WHERE id=?`, AwaitingApproval, "pending", time.Now().UTC().Format(time.RFC3339Nano), id)
			_, _ = e.DB.Exec(`UPDATE workflow_steps SET status=? WHERE id=?`, AwaitingApproval, step.ID)
			return e.Get(id)
		}
		now := time.Now().UTC()
		_, _ = e.DB.Exec(`UPDATE workflow_steps SET status=?,started_at=? WHERE id=?`, Running, now.Format(time.RFC3339Nano), step.ID)
		context := capabilities.ExecutionContext{
			Origin: workflow.Origin, Principal: capabilities.Principal{ID: workflow.Creator, Type: "workflow"},
			ProjectScope: workflow.ProjectScope, WorkflowID: workflow.ID,
			CorrelationID: "workflow_" + workflow.ID,
		}
		context.Permissions = permissionsForWorkflow(context.Origin)
		result := e.Executor.ExecuteWithContext(context, step.CapabilityID, step.Input)
		ended := time.Now().UTC()
		resultJSON, _ := json.Marshal(result.Data)
		errorJSON, _ := json.Marshal(result.Error)
		stepStatus := Succeeded
		if result.Status == "failed" {
			stepStatus = Failed
		}
		_, _ = e.DB.Exec(`UPDATE workflow_steps SET status=?,execution_id=?,result_json=?,error_json=?,ended_at=? WHERE id=?`,
			stepStatus, result.ExecutionID, string(resultJSON), string(errorJSON), ended.Format(time.RFC3339Nano), step.ID)
		if result.Status == "failed" {
			_, _ = e.DB.Exec(`UPDATE workflows SET status=?,error_json=?,updated_at=? WHERE id=?`, Failed, string(errorJSON), ended.Format(time.RFC3339Nano), id)
			return e.Get(id)
		}
		next := workflow.CurrentStep + 1
		_, _ = e.DB.Exec(`UPDATE workflows SET current_step=?,updated_at=? WHERE id=?`, next, ended.Format(time.RFC3339Nano), id)
		workflow.CurrentStep = next
	}
	_, _ = e.DB.Exec(`UPDATE workflows SET status=?,current_step=?,updated_at=? WHERE id=?`, Succeeded, len(workflow.Steps), time.Now().UTC().Format(time.RFC3339Nano), id)
	return e.Get(id)
}

func permissionsForWorkflow(origin capabilities.ExecutionOrigin) []security.Permission {
	switch origin {
	case capabilities.OriginHuman, capabilities.OriginInternal:
		return []security.Permission{
			security.ProjectRead, security.ProjectWrite, security.DatabaseInspect,
			security.FilesystemRead, security.FilesystemWrite,
		}
	case capabilities.OriginAI:
		return []security.Permission{security.ProjectRead, security.ProjectWrite, security.DatabaseInspect, security.FilesystemRead}
	default:
		return nil
	}
}

func (e *Engine) Cancel(id string) (Workflow, error) {
	now := time.Now().UTC().Format(time.RFC3339Nano)
	if _, err := e.DB.Exec(`UPDATE workflows SET cancellation_requested=1,status=?,updated_at=? WHERE id=?`, Cancelled, now, id); err != nil {
		return Workflow{}, err
	}
	if _, err := e.DB.Exec(`UPDATE workflow_steps SET status=? WHERE workflow_id=? AND status IN (?,?)`, Cancelled, id, Running, AwaitingApproval); err != nil {
		return Workflow{}, err
	}
	return e.Get(id)
}

func (e *Engine) Resume(id string) (Workflow, error) {
	workflow, err := e.Get(id)
	if err != nil {
		return Workflow{}, err
	}
	if workflow.CancellationRequested {
		return workflow, nil
	}
	if workflow.Status == AwaitingApproval {
		_, _ = e.DB.Exec(`UPDATE workflows SET status=?,updated_at=? WHERE id=?`, Pending, time.Now().UTC().Format(time.RFC3339Nano), id)
	}
	return e.Start(id)
}

func (e *Engine) RecoverAfterRestart() ([]Workflow, error) {
	rows, err := e.DB.Query(`SELECT id FROM workflows WHERE status=? OR status=?`, Running, AwaitingApproval)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	ids := make([]string, 0)
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		ids = append(ids, id)
	}
	recovered := make([]Workflow, 0, len(ids))
	for _, id := range ids {
		workflow, err := e.Get(id)
		if err != nil {
			return nil, err
		}
		if workflow.CancellationRequested {
			recovered = append(recovered, workflow)
			continue
		}
		if workflow.Status == AwaitingApproval {
			recovered = append(recovered, workflow)
			continue
		}
		if workflow.CurrentStep >= len(workflow.Steps) {
			recovered = append(recovered, workflow)
			continue
		}
		step := &workflow.Steps[workflow.CurrentStep]
		policy := strings.ToLower(strings.TrimSpace(step.ExecutionPolicy))
		if policy == "naturally_idempotent" || (policy == "idempotency_key" && strings.TrimSpace(step.IdempotencyKey) != "") {
			_, err = e.DB.Exec(`UPDATE workflow_steps SET status=?,recovery_state=? WHERE id=?`, Draft, string(RecoveryRetry), step.ID)
			if err != nil {
				return nil, err
			}
			_, err = e.DB.Exec(`UPDATE workflows SET status=?,updated_at=? WHERE id=?`, Running, time.Now().UTC().Format(time.RFC3339Nano), id)
			if err != nil {
				return nil, err
			}
			started, err := e.Start(id)
			if err != nil {
				return nil, err
			}
			recovered = append(recovered, started)
			continue
		}
		_, err = e.DB.Exec(`UPDATE workflow_steps SET status=?,recovery_state=? WHERE id=?`, AwaitingApproval, string(RecoveryApproval), step.ID)
		if err != nil {
			return nil, err
		}
		_, err = e.DB.Exec(`UPDATE workflows SET status=?,approval_state=?,updated_at=? WHERE id=?`, AwaitingApproval, "recovery_pending", time.Now().UTC().Format(time.RFC3339Nano), id)
		if err != nil {
			return nil, err
		}
		recovered = append(recovered, mustGetWorkflow(e, id))
	}
	return recovered, nil
}

func mustGetWorkflow(e *Engine, id string) Workflow {
	workflow, _ := e.Get(id)
	return workflow
}

func (e *Engine) ApproveStep(id string, approved bool) (Workflow, error) {
	workflow, err := e.Get(id)
	if err != nil {
		return Workflow{}, err
	}
	if workflow.CurrentStep >= len(workflow.Steps) {
		return workflow, nil
	}
	step := workflow.Steps[workflow.CurrentStep]
	status := "rejected"
	if approved {
		status = "approved"
	}
	_, err = e.DB.Exec(`UPDATE workflow_steps SET approval_status=?,status=? WHERE id=?`, status, Draft, step.ID)
	if err != nil {
		return Workflow{}, err
	}
	if approved {
		_, _ = e.DB.Exec(`UPDATE workflows SET status=?,approval_state=?,updated_at=? WHERE id=?`, Pending, "approved", time.Now().UTC().Format(time.RFC3339Nano), id)
		return e.Start(id)
	}
	_, _ = e.DB.Exec(`UPDATE workflows SET status=?,approval_state=?,updated_at=? WHERE id=?`, Failed, "rejected", time.Now().UTC().Format(time.RFC3339Nano), id)
	return e.Get(id)
}

func (e *Engine) ExecutionHistory(limit int) ([]map[string]interface{}, error) {
	if limit < 1 {
		limit = 100
	}
	if limit > 1000 {
		limit = 1000
	}
	rows, err := e.DB.Query(`
		SELECT execution_id, capability_id, origin, principal_id, project_scope, workflow_id,
		       correlation_id, started_at, ended_at, duration_ms, status, result_summary, error_json
		FROM execution_history ORDER BY started_at DESC LIMIT ?`, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	result := make([]map[string]interface{}, 0)
	for rows.Next() {
		var executionID, capabilityID, origin, principalID, projectScope, workflowID, correlationID string
		var startedAt, endedAt, status, resultSummary, errorJSON string
		var duration int64
		if err := rows.Scan(&executionID, &capabilityID, &origin, &principalID, &projectScope, &workflowID, &correlationID, &startedAt, &endedAt, &duration, &status, &resultSummary, &errorJSON); err != nil {
			return nil, err
		}
		var data interface{}
		var errorData interface{}
		_ = json.Unmarshal([]byte(resultSummary), &data)
		if errorJSON != "" {
			_ = json.Unmarshal([]byte(errorJSON), &errorData)
		}
		result = append(result, map[string]interface{}{
			"executionId": executionID, "capabilityId": capabilityID, "origin": origin, "principalId": principalID,
			"projectScope": projectScope, "workflowId": workflowID, "correlationId": correlationID,
			"startedAt": startedAt, "endedAt": endedAt, "durationMs": duration, "status": status,
			"result": data, "error": errorData,
		})
	}
	return result, rows.Err()
}
