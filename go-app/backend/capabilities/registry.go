package capabilities

import (
	"errors"
	"fmt"
	"strings"
	"sync"

	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type Policy interface {
	Allows(origin ExecutionOrigin, permission security.Permission) bool
}

type Registry struct {
	mu           sync.RWMutex
	capabilities map[string]Descriptor
	policy       Policy
}

func NewRegistry(policy Policy) *Registry {
	return &Registry{
		capabilities: make(map[string]Descriptor),
		policy:       policy,
	}
}

func (r *Registry) Register(d Descriptor) error {
	if strings.TrimSpace(d.ID) == "" {
		return errors.New("capability id is required")
	}
	if d.Handler == nil {
		return fmt.Errorf("capability %s has no handler", d.ID)
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, exists := r.capabilities[d.ID]; exists {
		return fmt.Errorf("capability %s is already registered", d.ID)
	}
	r.capabilities[d.ID] = d
	return nil
}

func (r *Registry) Get(id string) (Descriptor, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	d, ok := r.capabilities[id]
	return d, ok
}

func (r *Registry) List() []Descriptor {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]Descriptor, 0, len(r.capabilities))
	for _, d := range r.capabilities {
		result = append(result, d)
	}
	return result
}

func (r *Registry) CanExecute(origin ExecutionOrigin, d Descriptor) bool {
	for _, permission := range d.Permissions {
		if !r.policy.Allows(origin, permission) {
			return false
		}
	}
	return true
}
