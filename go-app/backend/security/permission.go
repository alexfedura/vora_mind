package security

type Permission string

const (
	ProjectRead     Permission = "project.read"
	ProjectWrite    Permission = "project.write"
	DatabaseInspect Permission = "database.inspect"
	FilesystemRead  Permission = "filesystem.read"
	FilesystemWrite Permission = "filesystem.write"
)
