package mcpgateway

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	goruntime "runtime"
	"testing"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	officialmcp "github.com/modelcontextprotocol/go-sdk/mcp"
)

func TestMCPProtocolDiscoveryAndInvocation(t *testing.T) {
	store, err := persistence.Open(t.TempDir() + "/mcp.db")
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	filesystem, err := infrastructure.NewFilesystemScope(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	runtime := application.NewRuntime(store, filesystem, "Vora Test", "0.26.10", "v3.1.0")
	gateway := NewGateway(runtime)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	serverTransport, clientTransport := officialmcp.NewInMemoryTransports()
	serverDone := make(chan error, 1)
	go func() { serverDone <- gateway.server.Run(ctx, serverTransport) }()

	client := officialmcp.NewClient(&officialmcp.Implementation{Name: "vora-test-client", Version: "1.0.0"}, nil)
	session, err := client.Connect(ctx, clientTransport, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer session.Close()

	tools, err := session.ListTools(ctx, &officialmcp.ListToolsParams{})
	if err != nil {
		t.Fatal(err)
	}
	if len(tools.Tools) == 0 {
		t.Fatal("MCP tool discovery returned no tools")
	}

	result, err := session.CallTool(ctx, &officialmcp.CallToolParams{
		Name:      "system_get_info",
		Arguments: map[string]interface{}{},
	})
	if err != nil {
		t.Fatal(err)
	}
	if result.IsError {
		t.Fatalf("MCP system tool failed: %#v", result)
	}
	cancel()
	select {
	case <-serverDone:
	case <-time.After(2 * time.Second):
		t.Fatal("MCP server did not stop after cancellation")
	}
}

func TestMCPExecutableSubprocess(t *testing.T) {
	executableName := "vora-mcp"
	if os.PathSeparator == '\\' {
		executableName += ".exe"
	}
	executable := filepath.Join(t.TempDir(), executableName)
	_, testFile, _, ok := goruntime.Caller(0)
	if !ok {
		t.Fatal("could not resolve MCP test source path")
	}
	moduleRoot := filepath.Clean(filepath.Join(filepath.Dir(testFile), "..", ".."))
	buildCtx, buildCancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer buildCancel()
	cmd := exec.CommandContext(buildCtx, "go", "build", "-o", executable, "./cmd/vora-mcp")
	cmd.Dir = moduleRoot
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("MCP executable build failed: %v\ncwd=%s\n%s", err, cmd.Dir, output)
	}
	dataDir := t.TempDir()
	process := exec.Command(executable)
	process.Env = append(os.Environ(), "VORA_GO_DATA_DIR="+dataDir)
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	client := officialmcp.NewClient(&officialmcp.Implementation{Name: "vora-subprocess-test", Version: "1.0.0"}, nil)
	session, err := client.Connect(ctx, &officialmcp.CommandTransport{
		Command: process, TerminateDuration: 3 * time.Second,
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	tools, err := session.ListTools(ctx, &officialmcp.ListToolsParams{})
	if err != nil {
		t.Fatal(err)
	}
	if len(tools.Tools) == 0 {
		t.Fatal("subprocess MCP discovery returned no tools")
	}
	info, err := session.CallTool(ctx, &officialmcp.CallToolParams{
		Name: "system_get_info", Arguments: map[string]interface{}{},
	})
	if err != nil {
		t.Fatal(err)
	}
	if info.IsError {
		t.Fatalf("subprocess MCP read failed: %#v", info)
	}
	denied, err := session.CallTool(ctx, &officialmcp.CallToolParams{
		Name:      "filesystem_create_text",
		Arguments: map[string]interface{}{"path": "denied.txt", "content": "x"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if !denied.IsError {
		t.Fatal("external MCP client unexpectedly received filesystem write permission")
	}
	malformed, err := session.CallTool(ctx, &officialmcp.CallToolParams{
		Name:      "project_create",
		Arguments: map[string]interface{}{"name": ""},
	})
	if err != nil {
		t.Fatal(err)
	}
	if !malformed.IsError {
		t.Fatal("malformed project input unexpectedly succeeded")
	}
	if err := session.Close(); err != nil {
		t.Fatal(err)
	}
	cancel()
}
