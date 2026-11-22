package application

import (
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
	goruntime "runtime"
	"sort"
	"strings"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/audit"
	"github.com/alexfedura/vora_mind/go-app/backend/capabilities"
	"github.com/alexfedura/vora_mind/go-app/backend/domain"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	"github.com/alexfedura/vora_mind/go-app/backend/security"
	"github.com/alexfedura/vora_mind/go-app/backend/workflows"
)

type PermissionPolicy struct{}

func (PermissionPolicy) Allows(origin capabilities.ExecutionOrigin, permission security.Permission) bool {
	switch permission {
	case security.ProjectRead, security.ProjectWrite, security.DatabaseInspect:
		return true
	case security.FilesystemRead, security.FilesystemWrite:
		return origin == capabilities.OriginHuman || origin == capabilities.OriginInternal
	default:
		return false
	}
}

type Runtime struct {
	Registry       *capabilities.Registry
	DB             *sql.DB
	DatabasePath   string
	ProjectRepo    *persistence.ProjectRepository
	AuditStore     *audit.SQLiteStore
	Filesystem     *infrastructure.FilesystemScope
	AppName        string
	AppVersion     string
	WailsVersion   string
	WorkflowEngine *workflows.Engine
}

func NewRuntime(store *persistence.Store, fs *infrastructure.FilesystemScope, appName, appVersion, wailsVersion string) *Runtime {
	registry := capabilities.NewRegistry(PermissionPolicy{})
	repo := &persistence.ProjectRepository{DB: store.DB}
	auditStore := &audit.SQLiteStore{DB: store.DB}
	r := &Runtime{
		Registry: registry, DB: store.DB, DatabasePath: store.Database,
		ProjectRepo: repo, AuditStore: auditStore, Filesystem: fs,
		AppName: appName, AppVersion: appVersion, WailsVersion: wailsVersion,
	}
	r.WorkflowEngine = workflows.NewEngine(store.DB, r)
	r.registerCapabilities()
	return r
}

func (r *Runtime) Execute(capabilityID string, input map[string]interface{}, origin string) capabilities.Result {
	context := capabilities.ExecutionContext{
		Origin:    capabilities.ExecutionOrigin(origin),
		Principal: capabilities.Principal{ID: "legacy_internal", Type: "internal"},
	}
	if !validOrigin(context.Origin) {
		context.Origin = ""
	}
	context.Permissions = permissionsForOrigin(context.Origin)
	return r.ExecuteWithContext(context, capabilityID, input)
}

func (r *Runtime) ExecuteWithContext(context capabilities.ExecutionContext, capabilityID string, input map[string]interface{}) capabilities.Result {
	start := time.Now()
	executionID := newID("exec")
	capabilityOrigin := context.Origin
	if !validOrigin(capabilityOrigin) {
		capabilityOrigin = ""
	}
	descriptor, ok := r.Registry.Get(capabilityID)
	if !ok {
		return r.finish(context, capabilityID, executionID, start, capabilityOrigin, "", nil, &capabilities.StructuredError{
			Code: "CAPABILITY_NOT_FOUND", Message: "The requested capability is not registered.", Capability: capabilityID,
		})
	}
	if !hasRequiredPermissions(context.Permissions, descriptor.Permissions) || !r.Registry.CanExecute(capabilityOrigin, descriptor) {
		return r.finish(context, capabilityID, executionID, start, capabilityOrigin, "", nil, &capabilities.StructuredError{
			Code: "PERMISSION_DENIED", Message: "The execution origin does not have the required permissions.", Capability: capabilityID,
		})
	}
	if descriptor.ApprovalRequired && !context.Approved {
		return r.finish(context, capabilityID, executionID, start, capabilityOrigin, projectScope(input), nil, &capabilities.StructuredError{
			Code: "APPROVAL_REQUIRED", Message: "This capability requires explicit approval.", Capability: capabilityID,
		})
	}
	if descriptor.Validate != nil {
		if err := descriptor.Validate(input); err != nil {
			return r.finish(context, capabilityID, executionID, start, capabilityOrigin, "", nil, &capabilities.StructuredError{
				Code: "INVALID_INPUT", Message: err.Error(), Capability: capabilityID,
			})
		}
	}
	data, err := descriptor.Handler(input)
	if err != nil {
		code := "CAPABILITY_EXECUTION_ERROR"
		var structured *capabilities.StructuredError
		if errors.As(err, &structured) {
			return r.finish(context, capabilityID, executionID, start, capabilityOrigin, projectScope(input), nil, structured)
		}
		return r.finish(context, capabilityID, executionID, start, capabilityOrigin, projectScope(input), nil, &capabilities.StructuredError{
			Code: code, Message: err.Error(), Capability: capabilityID,
		})
	}
	return r.finish(context, capabilityID, executionID, start, capabilityOrigin, projectScope(input), data, nil)
}

func (r *Runtime) finish(context capabilities.ExecutionContext, capabilityID, executionID string, start time.Time, origin capabilities.ExecutionOrigin, projectScope string, data interface{}, structured *capabilities.StructuredError) capabilities.Result {
	status := "succeeded"
	var auditError *audit.ErrorDetails
	if structured != nil {
		status = "failed"
		auditError = &audit.ErrorDetails{Code: structured.Code, Message: structured.Message}
	}
	duration := time.Since(start).Milliseconds()
	permissions := []security.Permission{}
	if descriptor, ok := r.Registry.Get(capabilityID); ok {
		permissions = descriptor.Permissions
	}
	_ = r.AuditStore.Record(audit.Event{
		ExecutionID: executionID, CapabilityID: capabilityID,
		Timestamp: time.Now().UTC(), Origin: audit.ExecutionOrigin(origin),
		ProjectScope: projectScope, Status: status, DurationMS: duration,
		Error: auditError, Permissions: permissions,
		PrincipalID: context.Principal.ID, CorrelationID: context.CorrelationID, WorkflowID: context.WorkflowID,
	})
	resultSummary, _ := json.Marshal(data)
	errorJSON, _ := json.Marshal(structured)
	startedAt := start.UTC().Format(time.RFC3339Nano)
	endedAt := time.Now().UTC().Format(time.RFC3339Nano)
	_, _ = r.DB.Exec(`
		INSERT OR REPLACE INTO execution_history(
			execution_id, capability_id, origin, principal_id, project_scope,
			workflow_id, correlation_id, started_at, ended_at, duration_ms,
			status, result_summary, error_json
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		executionID, capabilityID, origin, context.Principal.ID, projectScope,
		context.WorkflowID, context.CorrelationID, startedAt, endedAt, duration,
		status, string(resultSummary), string(errorJSON),
	)
	return capabilities.Result{
		ExecutionID: executionID, CapabilityID: capabilityID, Status: status,
		Data: data, Error: structured, DurationMS: duration,
	}
}

func validOrigin(origin capabilities.ExecutionOrigin) bool {
	switch origin {
	case capabilities.OriginHuman, capabilities.OriginInternal, capabilities.OriginAI, capabilities.OriginExternal:
		return true
	default:
		return false
	}
}

func permissionsForOrigin(origin capabilities.ExecutionOrigin) []security.Permission {
	switch origin {
	case capabilities.OriginHuman, capabilities.OriginInternal:
		return []security.Permission{
			security.ProjectRead, security.ProjectWrite, security.DatabaseInspect,
			security.FilesystemRead, security.FilesystemWrite,
		}
	case capabilities.OriginAI:
		return []security.Permission{
			security.ProjectRead, security.ProjectWrite, security.DatabaseInspect,
			security.FilesystemRead,
		}
	case capabilities.OriginExternal:
		return []security.Permission{security.ProjectRead, security.DatabaseInspect}
	default:
		return nil
	}
}

func hasRequiredPermissions(granted, required []security.Permission) bool {
	grantedSet := make(map[security.Permission]bool, len(granted))
	for _, permission := range granted {
		grantedSet[permission] = true
	}
	for _, permission := range required {
		if !grantedSet[permission] {
			return false
		}
	}
	return true
}

func projectScope(input map[string]interface{}) string {
	if input == nil {
		return ""
	}
	if value, ok := input["projectId"].(string); ok {
		return value
	}
	if value, ok := input["id"].(string); ok {
		return value
	}
	return ""
}

func newID(prefix string) string {
	bytes := make([]byte, 16)
	_, _ = rand.Read(bytes)
	return prefix + "_" + hex.EncodeToString(bytes)
}

func (r *Runtime) registerCapabilities() {
	r.mustRegister(capabilities.Descriptor{
		ID: "system.get_info", Version: "1.0.0", Name: "Get application information",
		Description:  "Returns application, backend, runtime, database, and capability metadata.",
		InputSchema:  capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"applicationName": capabilities.JSONStringSchema("Application name")}),
		Permissions:  []security.Permission{},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			return map[string]interface{}{
				"applicationName":           r.AppName,
				"applicationVersion":        r.AppVersion,
				"backendVersion":            goruntime.Version(),
				"runtime":                   map[string]interface{}{"goVersion": goruntime.Version(), "goos": goruntime.GOOS, "goarch": goruntime.GOARCH},
				"database":                  map[string]interface{}{"status": "connected", "path": r.DatabasePath},
				"registeredCapabilityCount": len(r.Registry.List()),
				"wailsVersion":              r.WailsVersion,
			}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "project.create", Version: "1.0.0", Name: "Create project",
		Description: "Creates a project in the clean application database.",
		InputSchema: capabilities.JSONObjectSchema([]string{"name"}, map[string]interface{}{
			"name": capabilities.JSONStringSchema("Project name"), "description": capabilities.JSONStringSchema("Project description"),
		}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"project": map[string]interface{}{"type": "object"}}),
		Permissions:  []security.Permission{security.ProjectWrite},
		Validate: func(input map[string]interface{}) error {
			_, err := requiredString(input, "name", 1, 120)
			_, descriptionErr := optionalString(input, "description", 2000)
			if descriptionErr != nil {
				return descriptionErr
			}
			return err
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			name, _ := requiredString(input, "name", 1, 120)
			description, _ := optionalString(input, "description", 2000)
			now := time.Now().UTC()
			project := domain.Project{ID: newID("project"), Name: name, Description: description, CreatedAt: now, UpdatedAt: now}
			if err := r.ProjectRepo.Create(project); err != nil {
				return nil, fmt.Errorf("could not create project: %w", err)
			}
			return map[string]interface{}{"project": project}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "project.list", Version: "1.0.0", Name: "List projects",
		Description:  "Lists projects in creation order.",
		InputSchema:  capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"projects": map[string]interface{}{"type": "array"}}),
		Permissions:  []security.Permission{security.ProjectRead},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			projects, err := r.ProjectRepo.List()
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"projects": projects}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "project.get", Version: "1.0.0", Name: "Get project",
		Description:  "Returns one project by stable ID.",
		InputSchema:  capabilities.JSONObjectSchema([]string{"id"}, map[string]interface{}{"id": capabilities.JSONStringSchema("Project ID")}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"project": map[string]interface{}{"type": "object"}}),
		Permissions:  []security.Permission{security.ProjectRead},
		Validate: func(input map[string]interface{}) error {
			_, err := requiredString(input, "id", 1, 128)
			return err
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			id, _ := requiredString(input, "id", 1, 128)
			project, err := r.ProjectRepo.Get(id)
			if errors.Is(err, persistence.ErrProjectNotFound) {
				return nil, fmt.Errorf("project %s was not found", id)
			}
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"project": project}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "project.update", Version: "1.0.0", Name: "Update project",
		Description: "Updates project name and description.",
		InputSchema: capabilities.JSONObjectSchema([]string{"id", "name"}, map[string]interface{}{
			"id": capabilities.JSONStringSchema("Project ID"), "name": capabilities.JSONStringSchema("Project name"),
			"description": capabilities.JSONStringSchema("Project description"),
		}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"project": map[string]interface{}{"type": "object"}}),
		Permissions:  []security.Permission{security.ProjectWrite},
		Validate: func(input map[string]interface{}) error {
			if _, err := requiredString(input, "id", 1, 128); err != nil {
				return err
			}
			if _, err := requiredString(input, "name", 1, 120); err != nil {
				return err
			}
			_, err := optionalString(input, "description", 2000)
			return err
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			id, _ := requiredString(input, "id", 1, 128)
			name, _ := requiredString(input, "name", 1, 120)
			description, _ := optionalString(input, "description", 2000)
			project, err := r.ProjectRepo.Update(id, name, description)
			if errors.Is(err, persistence.ErrProjectNotFound) {
				return nil, fmt.Errorf("project %s was not found", id)
			}
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"project": project}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "database.get_status", Version: "1.0.0", Name: "Get database status",
		Description:  "Returns database connection and migration status.",
		InputSchema:  capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		Permissions:  []security.Permission{security.DatabaseInspect},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			if err := r.DB.Ping(); err != nil {
				return nil, err
			}
			var migrations int
			if err := r.DB.QueryRow("SELECT COUNT(*) FROM schema_migrations").Scan(&migrations); err != nil {
				return nil, err
			}
			return map[string]interface{}{"status": "connected", "path": r.DatabasePath, "migrationCount": migrations, "foreignKeys": r.foreignKeyStatus()}, nil
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "database.list_tables", Version: "1.0.0", Name: "List database tables",
		Description:  "Lists application tables without exposing SQL execution.",
		InputSchema:  capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"tables": map[string]interface{}{"type": "array"}}),
		Permissions:  []security.Permission{security.DatabaseInspect},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			rows, err := r.DB.Query("SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
			if err != nil {
				return nil, err
			}
			defer rows.Close()
			tables := make([]map[string]interface{}, 0)
			for rows.Next() {
				var name, sqlText string
				if err := rows.Scan(&name, &sqlText); err != nil {
					return nil, err
				}
				tables = append(tables, map[string]interface{}{"name": name, "sql": sqlText})
			}
			return map[string]interface{}{"tables": tables}, rows.Err()
		},
	})

	r.mustRegister(capabilities.Descriptor{
		ID: "database.get_schema", Version: "1.0.0", Name: "Get database table schema",
		Description:  "Returns a structured allowlisted schema for one application table.",
		InputSchema:  capabilities.JSONObjectSchema([]string{"table"}, map[string]interface{}{"table": capabilities.JSONStringSchema("Table name")}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		Permissions:  []security.Permission{security.DatabaseInspect},
		Validate: func(input map[string]interface{}) error {
			_, err := requiredString(input, "table", 1, 128)
			return err
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			table, _ := requiredString(input, "table", 1, 128)
			if !regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*$`).MatchString(table) {
				return nil, fmt.Errorf("invalid table name")
			}
			var createSQL string
			err := r.DB.QueryRow("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?", table).Scan(&createSQL)
			if err != nil {
				if errors.Is(err, sql.ErrNoRows) {
					return nil, fmt.Errorf("table %s was not found", table)
				}
				return nil, err
			}
			rows, err := r.DB.Query("PRAGMA table_info(" + table + ")")
			if err != nil {
				return nil, err
			}
			defer rows.Close()
			columns := make([]map[string]interface{}, 0)
			for rows.Next() {
				var cid, notNull, primaryKey int
				var name, columnType string
				var defaultValue interface{}
				if err := rows.Scan(&cid, &name, &columnType, &notNull, &defaultValue, &primaryKey); err != nil {
					return nil, err
				}
				columns = append(columns, map[string]interface{}{
					"cid": cid, "name": name, "type": columnType, "notNull": notNull != 0,
					"defaultValue": defaultValue, "primaryKey": primaryKey != 0,
				})
			}
			return map[string]interface{}{"name": table, "sql": createSQL, "columns": columns}, rows.Err()
		},
	})

	r.registerFilesystemCapabilities()
	r.registerDiscoveryCapabilities()
	r.registerWorkflowCapabilities()
}

func (r *Runtime) foreignKeyStatus() bool {
	var value int
	if err := r.DB.QueryRow("PRAGMA foreign_keys").Scan(&value); err != nil {
		return false
	}
	return value == 1
}

func (r *Runtime) registerFilesystemCapabilities() {
	r.mustRegister(capabilities.Descriptor{
		ID: "filesystem.list", Version: "1.0.0", Name: "List filesystem entries",
		Description: "Lists entries inside configured allowed scopes.",
		InputSchema: capabilities.JSONObjectSchema([]string{"path"}, map[string]interface{}{"path": capabilities.JSONStringSchema("Relative or allowed absolute path")}),
		Permissions: []security.Permission{security.FilesystemRead},
		Validate:    func(input map[string]interface{}) error { _, err := requiredString(input, "path", 1, 4096); return err },
		Handler: func(input map[string]interface{}) (interface{}, error) {
			path, _ := requiredString(input, "path", 1, 4096)
			entries, err := r.Filesystem.List(path)
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"entries": entries}, nil
		},
	})
	r.mustRegister(capabilities.Descriptor{
		ID: "filesystem.stat", Version: "1.0.0", Name: "Inspect filesystem entry",
		Description: "Returns metadata for an entry inside allowed scopes.",
		InputSchema: capabilities.JSONObjectSchema([]string{"path"}, map[string]interface{}{"path": capabilities.JSONStringSchema("Relative or allowed absolute path")}),
		Permissions: []security.Permission{security.FilesystemRead},
		Validate:    func(input map[string]interface{}) error { _, err := requiredString(input, "path", 1, 4096); return err },
		Handler: func(input map[string]interface{}) (interface{}, error) {
			path, _ := requiredString(input, "path", 1, 4096)
			return r.Filesystem.Stat(path)
		},
	})
	r.mustRegister(capabilities.Descriptor{
		ID: "filesystem.read_text", Version: "1.0.0", Name: "Read text file",
		Description: "Reads a UTF-8 text file inside allowed scopes.",
		InputSchema: capabilities.JSONObjectSchema([]string{"path"}, map[string]interface{}{"path": capabilities.JSONStringSchema("Relative or allowed absolute path")}),
		Permissions: []security.Permission{security.FilesystemRead},
		Validate:    func(input map[string]interface{}) error { _, err := requiredString(input, "path", 1, 4096); return err },
		Handler: func(input map[string]interface{}) (interface{}, error) {
			path, _ := requiredString(input, "path", 1, 4096)
			return r.Filesystem.ReadText(path)
		},
	})
	r.mustRegister(capabilities.Descriptor{
		ID: "filesystem.create_text", Version: "1.0.0", Name: "Create text file",
		Description: "Creates or replaces a text file inside allowed scopes.",
		InputSchema: capabilities.JSONObjectSchema([]string{"path", "content"}, map[string]interface{}{
			"path":    capabilities.JSONStringSchema("Relative or allowed absolute path"),
			"content": capabilities.JSONStringSchema("UTF-8 text content"),
		}),
		Permissions: []security.Permission{security.FilesystemWrite},
		Validate: func(input map[string]interface{}) error {
			if _, err := requiredString(input, "path", 1, 4096); err != nil {
				return err
			}
			value, _ := input["content"].(string)
			if len([]rune(value)) > 10*1024*1024 {
				return fmt.Errorf("content exceeds 10 MiB limit")
			}
			return nil
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			path, _ := requiredString(input, "path", 1, 4096)
			content, _ := input["content"].(string)
			return r.Filesystem.CreateText(path, content)
		},
	})
	r.registerFilesystemAction("filesystem.edit_text", "Edit a text file inside allowed scopes.", "edit")
	r.registerFilesystemAction("filesystem.rename", "Rename a file inside allowed scopes.", "rename")
	r.registerFilesystemAction("filesystem.copy", "Copy a file inside allowed scopes.", "copy")
	r.registerFilesystemAction("filesystem.move", "Move a file inside allowed scopes.", "move")
	r.registerFilesystemAction("filesystem.delete", "Delete a file inside allowed scopes.", "delete")
}

func (r *Runtime) registerFilesystemAction(id, description, operation string) {
	approvalRequired := operation == "delete"
	r.mustRegister(capabilities.Descriptor{
		ID: id, Version: "1.0.0", Name: id, Description: description,
		InputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{
			"path":        capabilities.JSONStringSchema("Source path"),
			"source":      capabilities.JSONStringSchema("Source path"),
			"destination": capabilities.JSONStringSchema("Destination path"),
			"content":     capabilities.JSONStringSchema("UTF-8 text content"),
		}),
		OutputSchema:     capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		Permissions:      []security.Permission{security.FilesystemWrite},
		ApprovalRequired: approvalRequired,
		Validate: func(input map[string]interface{}) error {
			if operation == "rename" || operation == "copy" || operation == "move" {
				source, sourceErr := requiredString(input, "source", 1, 4096)
				destination, destinationErr := requiredString(input, "destination", 1, 4096)
				if sourceErr != nil {
					return sourceErr
				}
				if destinationErr != nil {
					return destinationErr
				}
				if source == destination {
					return fmt.Errorf("source and destination must differ")
				}
				return nil
			}
			_, err := requiredString(input, "path", 1, 4096)
			return err
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			switch operation {
			case "edit":
				path, _ := requiredString(input, "path", 1, 4096)
				content, _ := input["content"].(string)
				return r.Filesystem.CreateText(path, content)
			case "rename":
				source, _ := requiredString(input, "source", 1, 4096)
				destination, _ := requiredString(input, "destination", 1, 4096)
				return r.Filesystem.Rename(source, destination)
			case "copy":
				source, _ := requiredString(input, "source", 1, 4096)
				destination, _ := requiredString(input, "destination", 1, 4096)
				return r.Filesystem.Copy(source, destination)
			case "move":
				source, _ := requiredString(input, "source", 1, 4096)
				destination, _ := requiredString(input, "destination", 1, 4096)
				return r.Filesystem.Move(source, destination)
			case "delete":
				path, _ := requiredString(input, "path", 1, 4096)
				return r.Filesystem.Delete(path)
			default:
				return nil, fmt.Errorf("unsupported filesystem operation")
			}
		},
	})
}

func (r *Runtime) registerDiscoveryCapabilities() {
	r.mustRegister(capabilities.Descriptor{
		ID: "capability.list", Version: "1.0.0", Name: "List capabilities",
		Description:  "Returns machine-readable capability metadata.",
		InputSchema:  capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"capabilities": map[string]interface{}{"type": "array"}}),
		Permissions:  []security.Permission{},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			descriptors := r.Registry.List()
			sort.Slice(descriptors, func(i, j int) bool { return descriptors[i].ID < descriptors[j].ID })
			output := make([]map[string]interface{}, 0, len(descriptors))
			for _, d := range descriptors {
				output = append(output, map[string]interface{}{
					"id": d.ID, "version": d.Version, "name": d.Name, "description": d.Description,
					"inputSchema": d.InputSchema, "outputSchema": d.OutputSchema, "requiredPermissions": d.Permissions,
				})
			}

			return map[string]interface{}{"capabilities": output}, nil
		},
	})
	r.mustRegister(capabilities.Descriptor{
		ID: "capability.get", Version: "1.0.0", Name: "Get capability",
		Description:  "Returns metadata for one capability.",
		InputSchema:  capabilities.JSONObjectSchema([]string{"id"}, map[string]interface{}{"id": capabilities.JSONStringSchema("Capability ID")}),
		OutputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{}),
		Permissions:  []security.Permission{},
		Validate:     func(input map[string]interface{}) error { _, err := requiredString(input, "id", 1, 128); return err },
		Handler: func(input map[string]interface{}) (interface{}, error) {
			id, _ := requiredString(input, "id", 1, 128)
			d, ok := r.Registry.Get(id)
			if !ok {
				return nil, fmt.Errorf("capability %s was not found", id)
			}
			return map[string]interface{}{
				"id": d.ID, "version": d.Version, "name": d.Name, "description": d.Description,
				"inputSchema": d.InputSchema, "outputSchema": d.OutputSchema, "requiredPermissions": d.Permissions,
			}, nil
		},
	})
}

func (r *Runtime) registerWorkflowCapabilities() {
	r.mustRegister(capabilities.Descriptor{
		ID: "workflow.create", Version: "1.0.0", Name: "Create workflow", Description: "Creates a durable workflow.",
		InputSchema: capabilities.JSONObjectSchema([]string{"name", "steps"}, map[string]interface{}{"name": capabilities.JSONStringSchema("Workflow name"), "description": capabilities.JSONStringSchema("Description"), "projectScope": capabilities.JSONStringSchema("Project scope"), "steps": map[string]interface{}{"type": "array"}}),
		Permissions: []security.Permission{security.ProjectRead},
		Validate: func(input map[string]interface{}) error {
			name, _ := requiredString(input, "name", 1, 200)
			if name == "" {
				return fmt.Errorf("name is required")
			}
			steps, ok := input["steps"].([]interface{})
			if !ok {
				return fmt.Errorf("steps must be an array")
			}
			if len(steps) > 100 {
				return fmt.Errorf("workflow cannot exceed 100 steps")
			}
			return nil
		},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			name, _ := requiredString(input, "name", 1, 200)
			description, _ := optionalString(input, "description", 2000)
			projectScope, _ := optionalString(input, "projectScope", 128)
			raw, _ := input["steps"].([]interface{})
			steps := make([]map[string]interface{}, 0, len(raw))
			for _, item := range raw {
				if step, ok := item.(map[string]interface{}); ok {
					steps = append(steps, step)
				}
			}
			workflow, err := r.WorkflowEngine.Create(name, description, projectScope, "human_ui", capabilities.OriginHuman, steps)
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"workflow": workflow}, nil
		},
	})
	r.registerWorkflowAction("workflow.get", "Gets a workflow.", false)
	r.registerWorkflowAction("workflow.start", "Starts a workflow.", true)
	r.registerWorkflowAction("workflow.cancel", "Cancels a workflow.", true)
	r.registerWorkflowAction("workflow.resume", "Resumes a workflow.", true)
	r.registerWorkflowAction("workflow.approve_step", "Approves or rejects the current workflow step.", true)
	r.registerWorkflowHistory()
}

func (r *Runtime) registerWorkflowAction(id, description string, write bool) {
	permissions := []security.Permission{security.ProjectRead}
	if write {
		permissions = append(permissions, security.ProjectWrite)
	}
	r.mustRegister(capabilities.Descriptor{
		ID: id, Version: "1.0.0", Name: id, Description: description,
		InputSchema: capabilities.JSONObjectSchema([]string{"id"}, map[string]interface{}{"id": capabilities.JSONStringSchema("Workflow ID")}),
		Permissions: permissions,
		Validate:    func(input map[string]interface{}) error { _, err := requiredString(input, "id", 1, 128); return err },
		Handler: func(input map[string]interface{}) (interface{}, error) {
			workflowID, _ := requiredString(input, "id", 1, 128)
			var workflow workflows.Workflow
			var err error
			switch id {
			case "workflow.get":
				workflow, err = r.WorkflowEngine.Get(workflowID)
			case "workflow.start":
				workflow, err = r.WorkflowEngine.Start(workflowID)
			case "workflow.cancel":
				workflow, err = r.WorkflowEngine.Cancel(workflowID)
			case "workflow.resume":
				workflow, err = r.WorkflowEngine.Resume(workflowID)
			case "workflow.approve_step":
				approved, _ := input["approved"].(bool)
				workflow, err = r.WorkflowEngine.ApproveStep(workflowID, approved)
			default:
				workflow, err = r.WorkflowEngine.Get(workflowID)
			}
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"workflow": workflow}, nil
		},
	})
}

func (r *Runtime) registerWorkflowHistory() {
	r.mustRegister(capabilities.Descriptor{
		ID: "workflow.execution_history", Version: "1.0.0", Name: "Workflow execution history", Description: "Lists unified execution history.",
		InputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{"limit": map[string]interface{}{"type": "number"}}),
		Permissions: []security.Permission{security.ProjectRead},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			limit := 100
			if value, ok := input["limit"].(float64); ok {
				limit = int(value)
			}
			history, err := r.WorkflowEngine.ExecutionHistory(limit)
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"executions": history}, nil
		},
	})
	r.mustRegister(capabilities.Descriptor{
		ID: "workflow.list", Version: "1.0.0", Name: "List workflows", Description: "Lists durable workflows.",
		InputSchema: capabilities.JSONObjectSchema(nil, map[string]interface{}{}), Permissions: []security.Permission{security.ProjectRead},
		Handler: func(input map[string]interface{}) (interface{}, error) {
			list, err := r.WorkflowEngine.List()
			if err != nil {
				return nil, err
			}
			return map[string]interface{}{"workflows": list}, nil
		},
	})
}

func (r *Runtime) mustRegister(d capabilities.Descriptor) {
	if err := r.Registry.Register(d); err != nil {
		panic(err)
	}
}

func requiredString(input map[string]interface{}, key string, min, max int) (string, error) {
	value, _ := input[key].(string)
	value = strings.TrimSpace(value)
	if len([]rune(value)) < min {
		return "", fmt.Errorf("%s must contain at least %d character(s)", key, min)
	}
	if len([]rune(value)) > max {
		return "", fmt.Errorf("%s must contain at most %d character(s)", key, max)
	}
	return value, nil
}

func optionalString(input map[string]interface{}, key string, max int) (string, error) {
	value, _ := input[key].(string)
	value = strings.TrimSpace(value)
	if len([]rune(value)) > max {
		return "", fmt.Errorf("%s must contain at most %d character(s)", key, max)
	}
	return value, nil
}
