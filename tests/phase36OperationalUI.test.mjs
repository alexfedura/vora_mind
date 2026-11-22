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
