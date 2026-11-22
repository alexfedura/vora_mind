# VORA Phase 3.11 Windows Filesystem Report

## Automated Windows evidence

Passed:

- Windows junction escape rejection
- project root inside/outside allowed scope
- absolute path rejection
- UNC traversal rejection
- encoded and mixed traversal rejection
- broken symlink rejection

Directory symlink and junction tests execute on Windows; environment permission failures are reported as skips rather than passes.

Source evidence:

- `go-app/backend/infrastructure/filesystem.go`
- `go-app/backend/infrastructure/filesystem_test.go`

## Canonicalization

Canonicalization now handles:

- existing-path symlinks
- broken symlinks
- Windows reparse targets
- junction targets
- containment against the canonical project root
