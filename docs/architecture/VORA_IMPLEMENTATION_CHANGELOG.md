# Vora Implementation Changelog

## 2026-11-12

### Added

- `docs/architecture/VORA_CURRENT_STATE_BASELINE.md`
- `docs/architecture/VORA_TARGET_ARCHITECTURE.md`
- `docs/architecture/VORA_IMPLEMENTATION_PLAN.md`
- `docs/architecture/VORA_PROJECT_OWNERSHIP_SPEC.md`
- `docs/architecture/VORA_DATA_MIGRATION_SPEC.md`
- `docs/architecture/VORA_SCIENTIFIC_WORKFLOW_SPEC.md`
- `docs/architecture/VORA_TEST_AND_ACCEPTANCE_MATRIX.md`
- `docs/architecture/VORA_IMPLEMENTATION_CHANGELOG.md`

### Fixed

- project model workspace filtering in `src-tauri/src/model/repository.rs`: model and evidence queries now respect the selected project instead of returning the full global catalog.

### Notes

- This is the first stage of the planned project-ownership and isolation work.
- This does not yet claim full scientific workflow completion; the remaining gap list remains open and tracked in the architecture docs.
