package workflows

import (
	"testing"

	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
)

type testExecutor struct{}

func (testExecutor) ExecuteWithContext(context capabilities.ExecutionContext, capabilityID string, input map[string]interface{}) capabilities.Result {
	return capabilities.Result{ExecutionID: "exec_test", CapabilityID: capabilityID, Status: "succeeded", Data: map[string]interface{}{"ok": true}}
}

func TestWorkflowPersistenceCancelAndResume(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/workflow.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	engine := NewEngine(store.DB, testExecutor{})
	workflow, err := engine.Create("Test", "", "", "tester", capabilities.OriginHuman, []map[string]interface{}{
		{"capabilityId": "system.get_info", "input": map[string]interface{}{}},
	})
	if err != nil {
		t.Fatal(err)
	}
	started, err := engine.Start(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if started.Status != Succeeded {
		t.Fatalf("status=%s", started.Status)
	}
	cancelled, err := engine.Cancel(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if cancelled.Status != Cancelled {
		t.Fatalf("cancel status=%s", cancelled.Status)
	}
	resumed, err := engine.Resume(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if resumed.CancellationRequested == false {
		t.Fatal("resume should preserve cancellation")
	}
}

func TestWorkflowApprovalGate(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/approval.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	engine := NewEngine(store.DB, testExecutor{})
	workflow, err := engine.Create("Approval", "", "", "tester", capabilities.OriginHuman, []map[string]interface{}{
		{"capabilityId": "filesystem.create_text", "input": map[string]interface{}{"path": "test.txt", "content": "x"}, "approvalRequired": true},
	})
	if err != nil {
		t.Fatal(err)
	}
	waiting, err := engine.Start(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if waiting.Status != AwaitingApproval {
		t.Fatalf("status=%s", waiting.Status)
	}
	rejected, err := engine.ApproveStep(workflow.ID, false)
	if err != nil {
		t.Fatal(err)
	}
	if rejected.Status != Failed {
		t.Fatalf("rejected status=%s", rejected.Status)
	}
}

func TestWorkflowInterruptedIdempotentStepRecovery(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/recovery.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	engine := NewEngine(store.DB, testExecutor{})
	workflow, err := engine.Create("Recovery", "", "", "tester", capabilities.OriginHuman, []map[string]interface{}{
		{"capabilityId": "system.get_info", "input": map[string]interface{}{}, "executionPolicy": "naturally_idempotent", "idempotencyKey": "recovery-1"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`UPDATE workflows SET status=? WHERE id=?`, Running, workflow.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`UPDATE workflow_steps SET status=? WHERE workflow_id=?`, Running, workflow.ID); err != nil {
		t.Fatal(err)
	}
	recovered, err := engine.RecoverAfterRestart()
	if err != nil {
		t.Fatal(err)
	}
	if len(recovered) != 1 || recovered[0].Status != Succeeded {
		t.Fatalf("idempotent recovery status=%v", recovered)
	}
}

func TestWorkflowInterruptedUnknownStepRequiresApproval(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/uncertain.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	engine := NewEngine(store.DB, testExecutor{})
	workflow, err := engine.Create("Recovery", "", "", "tester", capabilities.OriginHuman, []map[string]interface{}{
		{"capabilityId": "filesystem.create_text", "input": map[string]interface{}{"path": "x.txt", "content": "x"}, "executionPolicy": "unknown"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`UPDATE workflows SET status=? WHERE id=?`, Running, workflow.ID); err != nil {
		t.Fatal(err)
	}
	recovered, err := engine.RecoverAfterRestart()
	if err != nil {
		t.Fatal(err)
	}
	if len(recovered) != 1 || recovered[0].Status != AwaitingApproval {
		t.Fatalf("uncertain recovery status=%v", recovered)
	}
}

func TestWorkflowApprovalRejectsParameterSubstitution(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/approval-substitution.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	engine := NewEngine(store.DB, testExecutor{})
	workflow, err := engine.Create("Approval", "", "", "tester", capabilities.OriginHuman, []map[string]interface{}{
		{"capabilityId": "filesystem.delete", "input": map[string]interface{}{"path": "approved.txt"}, "approvalRequired": true},
	})
	if err != nil {
		t.Fatal(err)
	}
	waiting, err := engine.Start(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if waiting.Status != AwaitingApproval {
		t.Fatalf("expected approval wait, got %s", waiting.Status)
	}
	approved, err := engine.ApproveStep(workflow.ID, true)
	if err != nil {
		t.Fatal(err)
	}
	if approved.Status == AwaitingApproval {
		t.Fatal("approval should initially be accepted")
	}
	if _, err := store.DB.Exec(`UPDATE workflow_steps SET input_json=? WHERE workflow_id=?`,
		`{"path":"changed.txt"}`, workflow.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`UPDATE workflows SET status=?,approval_state=?,current_step=? WHERE id=?`,
		Pending, "approved", 0, workflow.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := store.DB.Exec(`UPDATE workflow_steps SET status=?,approval_status=? WHERE workflow_id=?`,
		Draft, "approved", workflow.ID); err != nil {
		t.Fatal(err)
	}
	var status, approvalStatus, inputJSON string
	if err := store.DB.QueryRow(`SELECT status,approval_status,input_json FROM workflow_steps WHERE workflow_id=?`, workflow.ID).
		Scan(&status, &approvalStatus, &inputJSON); err != nil {
		t.Fatal(err)
	}
	if status != string(Draft) || approvalStatus != "approved" || inputJSON != `{"path":"changed.txt"}` {
		t.Fatalf("unexpected substituted approval state: status=%q approval=%q input=%s", status, approvalStatus, inputJSON)
	}
	var workflowStatus, workflowApprovalState string
	if err := store.DB.QueryRow(`SELECT status,approval_state FROM workflows WHERE id=?`, workflow.ID).
		Scan(&workflowStatus, &workflowApprovalState); err != nil {
		t.Fatal(err)
	}
	if workflowStatus != string(Pending) || workflowApprovalState != "approved" {
		t.Fatalf("unexpected workflow approval state: status=%q approval=%q", workflowStatus, workflowApprovalState)
	}
	reloaded, err := engine.Get(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if reloaded.Status != Pending || reloaded.Steps[0].ApprovalStatus != "approved" {
		t.Fatalf("unexpected reloaded state: status=%s approval=%s", reloaded.Status, reloaded.Steps[0].ApprovalStatus)
	}
	changedFingerprint, err := approvalFingerprint(reloaded.Steps[0].CapabilityID, reloaded.Steps[0].Input)
	if err != nil {
		t.Fatal(err)
	}
	if reloaded.Steps[0].ApprovalFingerprint == changedFingerprint {
		t.Fatalf("test setup unexpectedly retained changed approval fingerprint")
	}
	changed, err := engine.Start(workflow.ID)
	if err != nil {
		t.Fatal(err)
	}
	if changed.Status != AwaitingApproval {
		t.Fatalf("changed parameters must require fresh approval, got %s", changed.Status)
	}
}
