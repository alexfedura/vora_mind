package capabilities

import (
	"testing"

	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type allowPolicy struct{}

func (allowPolicy) Allows(origin ExecutionOrigin, permission security.Permission) bool {
	return false
}

func TestRegistryRegistrationAndDiscovery(t *testing.T) {
	registry := NewRegistry(allowPolicy{})
	err := registry.Register(Descriptor{
		ID: "test.echo", Version: "1.0.0", Name: "Echo",
		InputSchema:  JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: JSONObjectSchema(nil, map[string]interface{}{}),
		Handler:      func(input map[string]interface{}) (interface{}, error) { return input, nil },
	})
	if err != nil {
		t.Fatal(err)
	}
	descriptor, found := registry.Get("test.echo")
	if !found || descriptor.ID != "test.echo" {
		t.Fatalf("registered capability was not discoverable")
	}
	if len(registry.List()) != 1 {
		t.Fatalf("expected one capability")
	}
}

func TestRegistryRejectsDuplicateAndUnhandledCapability(t *testing.T) {
	registry := NewRegistry(allowPolicy{})
	descriptor := Descriptor{ID: "test.echo", Handler: func(map[string]interface{}) (interface{}, error) { return nil, nil }}
	if err := registry.Register(descriptor); err != nil {
		t.Fatal(err)
	}
	if err := registry.Register(descriptor); err == nil {
		t.Fatal("duplicate registration should fail")
	}
	if err := registry.Register(Descriptor{ID: "test.missing"}); err == nil {
		t.Fatal("capability without handler should fail")
	}
}
