const test = require("node:test");
const assert = require("node:assert/strict");
const { spawn } = require("node:child_process");
const path = require("node:path");

test("packaged executable honors single-instance lock", { timeout: 10000 }, async () => {
  const executable = process.env.VORA_E2_PACKAGED_EXE;
  if (!executable) {
    assert.fail("VORA_E2_PACKAGED_EXE is required");
  }
  assert.equal(path.basename(executable).toLowerCase(), "vora mind.exe");

  const first = spawn(executable, ["--no-sandbox"], {
    stdio: "ignore",
    env: { ...process.env, VORA_E2_SMOKE: "1" }
  });
  await new Promise((resolve) => setTimeout(resolve, 250));
  const second = spawn(executable, ["--no-sandbox"], {
    stdio: "ignore",
    env: { ...process.env, VORA_E2_SMOKE: "1" }
  });

  const secondExitCode = await new Promise((resolve) => {
    second.once("exit", (code) => resolve(code));
    setTimeout(() => resolve("timeout"), 3000);
  });
  await new Promise((resolve) => {
    first.once("exit", () => resolve());
    setTimeout(resolve, 5000);
  });

  assert.equal(secondExitCode, 0);
});
