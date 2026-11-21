package capabilities

import (
	"encoding/json"

	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type ExecutionOrigin string

const (
	OriginHuman    ExecutionOrigin = "human_ui"
	OriginInternal ExecutionOrigin = "internal"
	OriginAI       ExecutionOrigin = "ai_agent"
	OriginExternal ExecutionOrigin = "external_api"
)

type Descriptor struct {
	ID           string                             `json:"id"`
	Version      string                             `json:"version"`
	Name         string                             `json:"name"`
	Description  string                             `json:"description"`
	InputSchema  map[string]interface{}             `json:"inputSchema"`
	OutputSchema map[string]interface{}             `json:"outputSchema"`
	Permissions  []security.Permission              `json:"requiredPermissions"`
	Handler      Handler                            `json:"-"`
	Validate     func(map[string]interface{}) error `json:"-"`
}

type Handler func(input map[string]interface{}) (interface{}, error)

type StructuredError struct {
	Code       string `json:"code"`
	Message    string `json:"message"`
	Capability string `json:"capability,omitempty"`
	Retryable  bool   `json:"retryable"`
}

func (e *StructuredError) Error() string {
	return e.Message
}

type Result struct {
	ExecutionID  string           `json:"executionId"`
	CapabilityID string           `json:"capabilityId"`
	Status       string           `json:"status"`
	Data         interface{}      `json:"data,omitempty"`
	Error        *StructuredError `json:"error,omitempty"`
	DurationMS   int64            `json:"durationMs"`
}

func JSONObjectSchema(required []string, properties map[string]interface{}) map[string]interface{} {
	return map[string]interface{}{
		"type":                 "object",
		"properties":           properties,
		"required":             required,
		"additionalProperties": false,
	}
}

func JSONStringSchema(description string) map[string]interface{} {
	return map[string]interface{}{"type": "string", "description": description}
}

func MarshalDescriptor(d Descriptor) ([]byte, error) {
	return json.Marshal(d)
}
