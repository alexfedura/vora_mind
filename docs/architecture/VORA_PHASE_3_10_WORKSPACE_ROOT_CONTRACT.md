# VORA Phase 3.10 Workspace Root Contract

## Supported operations

| Operation | Capability |
|---|---|
| Get one root | `project.filesystem_root` |
| List roots | `project.filesystem_roots` |
| Update/register root | `project.filesystem_root_update` |
| Remove root registration | `project.filesystem_root_remove` |
| Resolve project-relative path | `project.filesystem_root_resolve` |

## Rules

- Project roots are persistent in `project_filesystem_roots`.
- Project creation automatically registers a root under the application data directory.
- Root updates require an absolute root inside a configured allowed filesystem scope.
- Removing a root preserves workspace files.
- Project deletion removes metadata/root registration and preserves workspace files.
- Project-relative resolution rejects absolute, UNC, drive-qualified, encoded traversal, and `..` paths.
