package mcpgateway

import (
	"context"
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
	runtime := application.NewRuntime(store, filesystem, "Vora Test", "0.26.6", "v3.1.0")
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
