# VORA Phase 3.9 Go Capability Catalog

## Count

The actual Go registry contains 29 capabilities:

- 1 system
- 6 project
- 3 database
- 9 filesystem
- 2 discovery
- 8 workflow

This differs from the earlier 20-capability estimate because dynamic filesystem and workflow registrations were not included in that estimate.

The machine-readable catalog is [VORA_PHASE_3_9_CAPABILITIES.json](./VORA_PHASE_3_9_CAPABILITIES.json).

## Capability groups

| Group | IDs |
|---|---|
| System | `system.get_info` |
| Projects | `project.create`, `project.list`, `project.get`, `project.update`, `project.delete`, `project.filesystem_root` |
| Database | `database.get_status`, `database.list_tables`, `database.get_schema` |
| Filesystem | `filesystem.list`, `filesystem.stat`, `filesystem.read_text`, `filesystem.create_text`, `filesystem.edit_text`, `filesystem.rename`, `filesystem.copy`, `filesystem.move`, `filesystem.delete` |
| Discovery | `capability.list`, `capability.get` |
| Workflows | `workflow.create`, `workflow.get`, `workflow.start`, `workflow.cancel`, `workflow.resume`, `workflow.approve_step`, `workflow.execution_history`, `workflow.list` |

## Adapter availability

- Wails: project, database, discovery, system, and workflow surfaces are partially represented; credentials and backups use direct `RuntimeService` methods.
- HTTP: 12 route registrations cover health, info, capabilities, execution, projects, workflows, credentials, and protected adapters.
- MCP: tools are generated dynamically from the registry.
- Workflow: the workflow engine independently persists workflow steps and exact approval fingerprints.
- AI: MCP execution currently uses the shared registry, but provider-independent orchestration is not implemented as a separate Go domain.
