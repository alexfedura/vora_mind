package api

import (
	"context"
	"encoding/json"
	"net"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
)

type Server struct {
	server            *http.Server
	runtime           *application.Runtime
	token             string
	limitMu           sync.Mutex
	requests          map[string][]time.Time
	running           bool
	address           string
	credentials       *security.CredentialManager
	tokenCredentialID string
}

type credentialContextKey struct{}

func NewServer(runtime *application.Runtime) *Server {
	s := &Server{runtime: runtime, requests: make(map[string][]time.Time), credentials: security.NewCredentialManager(runtime.DB)}
	credential, token, err := s.credentials.Generate([]security.Permission{
		security.ProjectRead,
		security.ProjectWrite,
		security.DatabaseInspect,
		security.CapabilitiesDiscover,
	})
	if err != nil {
		s.token = ""
	} else {
		s.token = token
	}
	s.tokenCredentialID = credential.ID
	mux := http.NewServeMux()
	s.register(mux)
	s.server = &http.Server{Handler: withSecurityHeaders(mux), ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 10 * time.Second, WriteTimeout: 30 * time.Second, IdleTimeout: 60 * time.Second, MaxHeaderBytes: 16 * 1024}
	s.address = "127.0.0.1:0"
	return s
}

func (s *Server) Address() string              { return s.address }
func (s *Server) Running() bool                { return s.running }
func (s *Server) AuthenticationStatus() string { return "bearer token required" }
func (s *Server) Token() string                { return s.token }

func (s *Server) Start(ctx context.Context) error {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	s.address = listener.Addr().String()
	s.running = true
	go func() { <-ctx.Done(); _ = s.Stop() }()
	go func() {
		if err := s.server.Serve(listener); err != nil && err != http.ErrServerClosed {
			s.running = false
		}
	}()
	return nil
}

func (s *Server) Stop() error { s.running = false; return s.server.Shutdown(context.Background()) }

func withSecurityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Host != "" {
			hostName, _, err := net.SplitHostPort(r.Host)
			if err != nil {
				hostName = r.Host
			}
			if hostName != "127.0.0.1" && hostName != "localhost" && hostName != "::1" {
				writeJSON(w, http.StatusForbidden, structuredError("INVALID_HOST", "Only loopback hosts are accepted", http.StatusForbidden))
				return
			}
		}
		origin := r.Header.Get("Origin")
		if origin != "" {
			writeJSON(w, http.StatusForbidden, structuredError("ORIGIN_NOT_ALLOWED", "Cross-origin browser requests are not allowed", http.StatusForbidden))
			return
		}
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("Referrer-Policy", "no-referrer")
		next.ServeHTTP(w, r)
	})
}

func writeJSON(w http.ResponseWriter, status int, value interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(value)
}

func structuredError(code, message string, status int) map[string]interface{} {
	return map[string]interface{}{"error": map[string]interface{}{"code": code, "message": message, "retryable": false}, "status": status}
}

func (s *Server) authenticate(w http.ResponseWriter, r *http.Request) (security.Credential, bool) {
	authorization := r.Header.Get("Authorization")
	const prefix = "Bearer "
	if !strings.HasPrefix(authorization, prefix) {
		writeJSON(w, http.StatusUnauthorized, structuredError("UNAUTHORIZED", "Bearer authentication is required", http.StatusUnauthorized))
		return security.Credential{}, false
	}
	token := strings.TrimSpace(strings.TrimPrefix(authorization, prefix))
	if credential, ok := s.credentials.Authenticate(token); ok {
		return credential, true
	}
	writeJSON(w, http.StatusUnauthorized, structuredError("UNAUTHORIZED", "Invalid or revoked bearer token", http.StatusUnauthorized))
	return security.Credential{}, false
}

func (s *Server) rateLimited(r *http.Request) bool {
	key := r.RemoteAddr
	now := time.Now()
	s.limitMu.Lock()
	defer s.limitMu.Unlock()
	window := make([]time.Time, 0)
	for _, timestamp := range s.requests[key] {
		if now.Sub(timestamp) < time.Minute {
			window = append(window, timestamp)
		}
	}
	if len(window) >= 120 {
		s.requests[key] = window
		return true
	}
	window = append(window, now)
	s.requests[key] = window
	return false
}

func (s *Server) register(mux *http.ServeMux) {
	mux.HandleFunc("/api/v1/health", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w)
		} else {
			writeJSON(w, http.StatusOK, map[string]interface{}{"status": "ok", "loopbackOnly": true})
		}
	})
	mux.HandleFunc("/api/v1/info", s.protected(func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet {
			s.executeResponse(w, r, "system.get_info", nil)
		} else {
			methodNotAllowed(w)
		}
	}))
	mux.HandleFunc("/api/v1/capabilities", s.protected(func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet {
			s.executeResponse(w, r, "capability.list", nil)
		} else {
			methodNotAllowed(w)
		}
	}))
	mux.HandleFunc("/api/v1/capabilities/", s.protected(s.capabilityRoute))
	mux.HandleFunc("/api/v1/executions", s.protected(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w)
			return
		}
		limit := 100
		if value := r.URL.Query().Get("limit"); value != "" {
			parsed, err := strconv.Atoi(value)
			if err == nil {
				limit = parsed
			}
		}
		s.runtimeExecuteHistory(w, limit)
	}))
	mux.HandleFunc("/api/v1/executions/", s.protected(s.executionRoute))
	mux.HandleFunc("/api/v1/projects", s.protected(s.projectsRoute))
	mux.HandleFunc("/api/v1/projects/", s.protected(s.projectRoute))
	mux.HandleFunc("/api/v1/workflows", s.protected(s.workflowsRoute))
	mux.HandleFunc("/api/v1/workflows/", s.protected(s.workflowRoute))
}

func methodNotAllowed(w http.ResponseWriter) {
	writeJSON(w, http.StatusMethodNotAllowed, structuredError("METHOD_NOT_ALLOWED", "Method is not allowed", http.StatusMethodNotAllowed))
}

func (s *Server) protected(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.ContentLength > 2*1024*1024 {
			writeJSON(w, http.StatusRequestEntityTooLarge, structuredError("REQUEST_TOO_LARGE", "Request body exceeds 2 MiB", http.StatusRequestEntityTooLarge))
			return
		}
		if r.Method == http.MethodPost || r.Method == http.MethodPatch {
			if r.Header.Get("Content-Type") != "application/json" && r.Header.Get("Content-Type") != "application/json; charset=utf-8" {
				writeJSON(w, http.StatusUnsupportedMediaType, structuredError("INVALID_CONTENT_TYPE", "Content-Type must be application/json", http.StatusUnsupportedMediaType))
				return
			}
		}
		if s.rateLimited(r) {
			writeJSON(w, http.StatusTooManyRequests, structuredError("RATE_LIMITED", "Too many requests", http.StatusTooManyRequests))
			return
		}
		credential, authenticated := s.authenticate(w, r)
		if authenticated {
			ctx := context.WithValue(r.Context(), credentialContextKey{}, credential)
			next(w, r.WithContext(ctx))
		}
	}
}

func (s *Server) executeResponse(w http.ResponseWriter, r *http.Request, id string, input map[string]interface{}) {
	credential, _ := r.Context().Value(credentialContextKey{}).(security.Credential)
	result := s.runtime.ExecuteWithContext(trustedContext(r, credential), id, input)
	status := http.StatusOK
	if result.Status == "failed" {
		status = http.StatusBadRequest
		if result.Error != nil && (result.Error.Code == "PERMISSION_DENIED") {
			status = http.StatusForbidden
		}
	}
	writeJSON(w, status, result)
}

func trustedContext(r *http.Request, credential security.Credential) capabilities.ExecutionContext {
	correlation := r.Header.Get("X-Correlation-ID")
	if correlation == "" {
		correlation = "http_" + time.Now().UTC().Format("150405.000000000")
	}
	principalID := "bearer_external"
	if credential.ID != "" {
		principalID = credential.ID
	}
	return capabilities.ExecutionContext{
		ExecutionID:   "",
		Origin:        capabilities.OriginExternal,
		Principal:     capabilities.Principal{ID: principalID, Type: "api_client"},
		Permissions:   credential.Permissions,
		CorrelationID: correlation,
	}
}

func (s *Server) decodeJSON(w http.ResponseWriter, r *http.Request) (map[string]interface{}, bool) {
	r.Body = http.MaxBytesReader(w, r.Body, 2*1024*1024)
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	var input map[string]interface{}
	if err := decoder.Decode(&input); err != nil {
		writeJSON(w, http.StatusBadRequest, structuredError("INVALID_JSON", "Request body must be valid JSON", http.StatusBadRequest))
		return nil, false
	}
	if input == nil {
		input = map[string]interface{}{}
	}
	return input, true
}

func (s *Server) capabilityRoute(w http.ResponseWriter, r *http.Request) {
	remainder := strings.TrimPrefix(r.URL.Path, "/api/v1/capabilities/")
	parts := strings.Split(strings.Trim(remainder, "/"), "/")
	if len(parts) == 0 || parts[0] == "" {
		methodNotAllowed(w)
		return
	}
	if len(parts) == 1 {
		if r.Method == http.MethodGet {
			s.executeResponse(w, r, "capability.get", map[string]interface{}{"id": parts[0]})
			return
		}
		methodNotAllowed(w)
		return
	}
	if len(parts) == 2 && parts[1] == "execute" && r.Method == http.MethodPost {
		input, ok := s.decodeJSON(w, r)
		if !ok {
			return
		}
		s.executeResponse(w, r, parts[0], input)
		return
	}
	writeJSON(w, http.StatusNotFound, structuredError("NOT_FOUND", "Endpoint was not found", http.StatusNotFound))
}

func (s *Server) executionRoute(w http.ResponseWriter, r *http.Request) {
	id := strings.Trim(strings.TrimPrefix(r.URL.Path, "/api/v1/executions/"), "/")
	if r.Method != http.MethodGet {
		methodNotAllowed(w)
		return
	}
	if id == "" {
		methodNotAllowed(w)
		return
	}
	history, err := s.runtime.WorkflowEngine.ExecutionHistory(1000)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, structuredError("EXECUTION_HISTORY_ERROR", err.Error(), http.StatusInternalServerError))
		return
	}
	for _, item := range history {
		if item["executionId"] == id {
			writeJSON(w, http.StatusOK, item)
			return
		}
	}
	writeJSON(w, http.StatusNotFound, structuredError("EXECUTION_NOT_FOUND", "Execution was not found", http.StatusNotFound))
}

func (s *Server) runtimeExecuteHistory(w http.ResponseWriter, limit int) {
	history, err := s.runtime.WorkflowEngine.ExecutionHistory(limit)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, structuredError("EXECUTION_HISTORY_ERROR", err.Error(), http.StatusInternalServerError))
		return
	}
	writeJSON(w, http.StatusOK, map[string]interface{}{"executions": history})
}

func (s *Server) projectsRoute(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		s.executeResponse(w, r, "project.list", nil)
	case http.MethodPost:
		input, ok := s.decodeJSON(w, r)
		if !ok {
			return
		}
		s.executeResponse(w, r, "project.create", input)
	default:
		methodNotAllowed(w)
	}
}

func (s *Server) projectRoute(w http.ResponseWriter, r *http.Request) {
	id := strings.Trim(strings.TrimPrefix(r.URL.Path, "/api/v1/projects/"), "/")
	if id == "" {
		methodNotAllowed(w)
		return
	}
	switch r.Method {
	case http.MethodGet:
		s.executeResponse(w, r, "project.get", map[string]interface{}{"id": id})
	case http.MethodPatch:
		input, ok := s.decodeJSON(w, r)
		if !ok {
			return
		}
		input["id"] = id
		s.executeResponse(w, r, "project.update", input)
	default:
		methodNotAllowed(w)
	}
}

func (s *Server) workflowsRoute(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		s.executeResponse(w, r, "workflow.list", nil)
	case http.MethodPost:
		input, ok := s.decodeJSON(w, r)
		if !ok {
			return
		}
		s.executeResponse(w, r, "workflow.create", input)
	default:
		methodNotAllowed(w)
	}
}

func (s *Server) workflowRoute(w http.ResponseWriter, r *http.Request) {
	remainder := strings.Trim(strings.TrimPrefix(r.URL.Path, "/api/v1/workflows/"), "/")
	parts := strings.Split(remainder, "/")
	if len(parts) == 0 || parts[0] == "" {
		writeJSON(w, http.StatusNotFound, structuredError("NOT_FOUND", "Workflow was not found", http.StatusNotFound))
		return
	}
	if len(parts) == 1 {
		if r.Method == http.MethodGet {
			s.executeResponse(w, r, "workflow.get", map[string]interface{}{"id": parts[0]})
			return
		}
		methodNotAllowed(w)
		return
	}
	if len(parts) == 2 && r.Method == http.MethodPost {
		input, ok := s.decodeJSON(w, r)
		if !ok {
			return
		}
		input["id"] = parts[0]
		action := "workflow." + parts[1]
		if parts[1] == "approve" || parts[1] == "approve_step" {
			action = "workflow.approve_step"
		}
		if parts[1] == "history" {
			history, err := s.runtime.WorkflowEngine.ExecutionHistory(100)
			if err != nil {
				writeJSON(w, http.StatusInternalServerError, structuredError("HISTORY_ERROR", err.Error(), http.StatusInternalServerError))
				return
			}
			writeJSON(w, http.StatusOK, map[string]interface{}{"executions": history})
			return
		}
		s.executeResponse(w, r, action, input)
		return
	}
	writeJSON(w, http.StatusNotFound, structuredError("NOT_FOUND", "Endpoint was not found", http.StatusNotFound))
}
