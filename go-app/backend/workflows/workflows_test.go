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
