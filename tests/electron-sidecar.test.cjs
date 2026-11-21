const test = require("node:test");
const assert = require("node:assert/strict");
const { spawn } = require("node:child_process");
const path = require("node:path");

function requestSidecar(sidecarPath, method, params = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(sidecarPath, [], {
      shell: false,
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"]
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      const newline = stdout.indexOf("\n");
      if (newline < 0) return;
      const line = stdout.slice(0, newline).trim();
      try {
        resolve(JSON.parse(line));
      } catch (error) {
        reject(error);
      }
      child.stdin.end();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code !== 0 && code !== null) {
        reject(new Error(`Sidecar exited with code ${code}: ${stderr}`));
      }
    });
    const requestId = `test-${method}-${Date.now()}`;
    child.stdin.write(JSON.stringify({
      protocolVersion: 1,
      requestId,
      method,
      params
    }) + "\n");
  });
}

test("real sidecar returns health", { timeout: 10000 }, async () => {
  const sidecar = path.resolve(__dirname, "..", "electron", "sidecar", "bin", "vora_e021_sidecar.exe");
  const response = await requestSidecar(sidecar, "health");
  assert.equal(response.ok, true);
  assert.equal(response.result.status, "ready");
  assert.equal(typeof response.result.pid, "number");
});

test("real sidecar executes synthetic in-memory SQL", { timeout: 10000 }, async () => {
  const sidecar = path.resolve(__dirname, "..", "electron", "sidecar", "bin", "vora_e021_sidecar.exe");
  const child = spawn(sidecar, [], {
    shell: false,
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"]
  });
  let stdout = "";
  const results = [];
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
    let newline;
    while ((newline = stdout.indexOf("\n")) >= 0) {
      const line = stdout.slice(0, newline).trim();
      stdout = stdout.slice(newline + 1);
      if (line) results.push(JSON.parse(line));
    }
  });

  const createId = "test-create";
  const ddlId = "test-ddl";
  const insertId = "test-insert";
  const selectId = "test-select";
  child.stdin.write(JSON.stringify({
    protocolVersion: 1,
    requestId: createId,
    method: "create_synthetic_database",
    params: { database: ":memory:" }
  }) + "\n");
  await new Promise((resolve) => {
    const poll = setInterval(() => {
      if (results.some((item) => item.requestId === createId)) {
        clearInterval(poll);
        resolve();
      }
    }, 10);
  });
  child.stdin.write(JSON.stringify({
    protocolVersion: 1,
    requestId: ddlId,
    method: "execute_approved_query",
    params: {
      database: ":memory:",
      sql: "CREATE TABLE smoke(value INTEGER);"
    }
  }) + "\n");
  await new Promise((resolve) => {
    const poll = setInterval(() => {
      if (results.some((item) => item.requestId === ddlId)) {
        clearInterval(poll);
        resolve();
      }
    }, 10);
  });
  const ddlResult = results.find((item) => item.requestId === ddlId);
  assert.equal(ddlResult.ok, true);

  child.stdin.write(JSON.stringify({
    protocolVersion: 1,
    requestId: insertId,
    method: "execute_approved_query",
    params: {
      database: ":memory:",
      sql: "INSERT INTO smoke VALUES (1),(2),(3);"
    }
  }) + "\n");
  await new Promise((resolve) => {
    const poll = setInterval(() => {
      if (results.some((item) => item.requestId === insertId)) {
        clearInterval(poll);
        resolve();
      }
    }, 10);
  });
  assert.equal(results.find((item) => item.requestId === insertId).ok, true);

  child.stdin.write(JSON.stringify({
    protocolVersion: 1,
    requestId: selectId,
    method: "execute_approved_query",
    params: {
      database: ":memory:",
      sql: "SELECT COUNT(*) AS count FROM smoke;"
    }
  }) + "\n");
  await new Promise((resolve) => {
    const poll = setInterval(() => {
      if (results.some((item) => item.requestId === selectId)) {
        clearInterval(poll);
        resolve();
      }
    }, 10);
  });
  const selectResult = results.find((item) => item.requestId === selectId);
  assert.equal(selectResult.ok, true);
  assert.equal(selectResult.result.rows[0].column0, 3);
  child.kill();
});
