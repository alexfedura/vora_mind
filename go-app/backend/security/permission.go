package security

type Permission string

const (
	ProjectRead          Permission = "project.read"
	ProjectWrite         Permission = "project.write"
	DatabaseInspect      Permission = "database.inspect"
	FilesystemRead       Permission = "filesystem.read"
	FilesystemWrite      Permission = "filesystem.write"
	CapabilitiesDiscover Permission = "capabilities.discover"
	CapabilitiesExecute  Permission = "capabilities.execute"
	WorkflowsRead        Permission = "workflows.read"
	WorkflowsWrite       Permission = "workflows.write"
	WorkflowsApprove     Permission = "workflows.approve"
	BackupCreate         Permission = "backup.create"
	BackupRestore        Permission = "backup.restore"
)
