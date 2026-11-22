import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("Wails operational UI contains restore confirmation and one-time secret handling", () => {
  const source = readFileSync(new URL("../go-app/frontend/src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /Credential Management/);
  assert.match(source, /One-time secret/);
  assert.match(source, /navigator\.clipboard\.writeText/);
  assert.match(source, /setCredentialSecret\(""\)/);
  assert.match(source, /Backup and Restore/);
  assert.match(source, /Restore this database backup/);
  assert.match(source, /Validate the backup before restore/);
  assert.doesNotMatch(source, /localStorage|sessionStorage/);
});

test("Phase 3.7 UI exposes project delete confirmation", () => {
  const source = readFileSync(new URL("../go-app/frontend/src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /Delete project/);
  assert.match(source, /Workspace files will be preserved/);
  assert.match(source, /RuntimeService\.DeleteProject/);
});

test("Phase 3.7 Wails delete binding executes explicit human approval", () => {
  const source = readFileSync(new URL("../go-app/runtime_service.go", import.meta.url), "utf8");
  assert.match(source, /ExecuteWithContext/);
  assert.match(source, /OriginHuman/);
  assert.match(source, /Approved:\s*true/);
  assert.match(source, /ProjectScope:\s*id/);
});

test("Phase 3.10 Wails UI exposes project context and workspace roots", () => {
  const source = readFileSync(new URL("../go-app/frontend/src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /GetProjectContext/);
  assert.match(source, /OpenProject/);
  assert.match(source, /ListProjectFilesystemRoots/);
  assert.match(source, /UpdateProjectFilesystemRoot/);
  assert.match(source, /RemoveProjectFilesystemRoot/);
  assert.match(source, /Active project:/);
});

test("Phase 3.12 Wails UI syncs workspace root from the active project", () => {
  const source = readFileSync(new URL("../go-app/frontend/src/App.tsx", import.meta.url), "utf8");
  assert.match(source,/const activeRoot=activeId\?roots\.find\(root=>root\.projectId===activeId\):undefined/);
  assert.match(source,/setWorkspaceRoot\(activeRoot\?\.root\|\|""\)/);
});

test("Phase 3.12.2 Wails UI displays the existing runtime database status read-only", () => {
  const source = readFileSync(new URL("../go-app/frontend/src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /Database Status/);
  assert.match(source, /RuntimeService\.GetDatabaseStatus/);
  assert.match(source, /Loading database status/);
  assert.match(source, /Database status error:/);
  assert.match(source, /Database path:/);

  const refreshDatabaseStatus = source.match(/const refreshDatabaseStatus=async\(\)=>\{([\s\S]*?\n  \};)/);
  assert.ok(refreshDatabaseStatus, "database-status refresh function should exist");
  const diagnosticBody = refreshDatabaseStatus[1];
  assert.doesNotMatch(diagnosticBody, /CreateProject|DeleteProject|UpdateProject|OpenProject|CreateBackup|RestoreBackup|ValidateBackup|CreateText|DeleteText|UpdateProjectFilesystemRoot|RemoveProjectFilesystemRoot|persistence\.Open|VACUUM|INSERT|UPDATE|DELETE/);
});
