package mcpgateway

import (
	"context"
	"encoding/json"
	"strings"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
	officialmcp "github.com/modelcontextprotocol/go-sdk/mcp"
)

type Gateway struct {
	runtime *application.Runtime
	server  *officialmcp.Server
}

func NewGateway(runtime *application.Runtime) *Gateway {
	gateway := &Gateway{
		runtime: runtime,
		server: officialmcp.NewServer(
			&officialmcp.Implementation{Name: "vora-mind", Title: "Vora Mind", Version: "0.26.10"},
			nil,
		),
	}
	gateway.registerTools()
	return gateway
}

func (g *Gateway) registerTools() {
	for _, descriptor := range g.runtime.Registry.List() {
		descriptor := descriptor
		toolName := strings.ReplaceAll(descriptor.ID, ".", "_")
		inputSchema := rawJSONSchema(descriptor.InputSchema)
		outputSchema := rawJSONSchema(descriptor.OutputSchema)
		g.server.AddTool(
			&officialmcp.Tool{
				Name:         toolName,
				Description:  descriptor.Description,
				InputSchema:  inputSchema,
				OutputSchema: outputSchema,
			},
			func(ctx context.Context, request *officialmcp.CallToolRequest) (*officialmcp.CallToolResult, error) {
				input := map[string]interface{}{}
				if request.Params != nil {
					if err := json.Unmarshal(request.Params.Arguments, &input); err != nil {
						return &officialmcp.CallToolResult{
							Content: []officialmcp.Content{&officialmcp.TextContent{Text: "invalid tool arguments"}},
							IsError: true,
						}, nil
					}
				}
				result := g.runtime.ExecuteWithContext(
					capabilities.ExecutionContext{
						Origin:    capabilities.OriginAI,
						Principal: capabilities.Principal{ID: "mcp_local_client", Type: "ai_client"},
						Permissions: []security.Permission{
							security.ProjectRead,
							security.ProjectWrite,
							security.DatabaseInspect,
							security.FilesystemRead,
						},
						CorrelationID: "mcp_" + timeSafeID(),
					},
					descriptor.ID,
					input,
				)
				payload, _ := json.Marshal(result)
				content := &officialmcp.TextContent{Text: string(payload)}
				return &officialmcp.CallToolResult{
					Content:           []officialmcp.Content{content},
					StructuredContent: result,
					IsError:           result.Status == "failed",
				}, nil
			},
		)
	}
}

func rawJSONSchema(schema map[string]interface{}) interface{} {
	if len(schema) == 0 {
		schema = map[string]interface{}{"type": "object", "properties": map[string]interface{}{}, "required": []string{}}
	}
	if _, ok := schema["type"]; !ok {
		schema = map[string]interface{}{"type": "object", "properties": schema, "required": []string{}}
	}
	payload, _ := json.Marshal(schema)
	return json.RawMessage(payload)
}

func timeSafeID() string {
	return strings.ReplaceAll(strings.TrimSpace(descriptorTime()), " ", "_")
}

func descriptorTime() string {
	return timeNow()
}

func timeNow() string {
	return time.Now().UTC().Format("20060102T150405.000000000Z")
}

func (g *Gateway) Run(ctx context.Context) error {
	return g.server.Run(ctx, &officialmcp.StdioTransport{})
}
