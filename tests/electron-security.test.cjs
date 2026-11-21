const test = require("node:test");
const assert = require("node:assert/strict");
const { isAllowedFrameUrl, validateShellStatusPayload } = require("../dist-electron/security.js");

test("allows exact development and file frame URLs", () => {
  assert.equal(isAllowedFrameUrl("http://127.0.0.1:1420/", ["http://127.0.0.1:1420/"]), true);
  assert.equal(isAllowedFrameUrl("file:///D:/vora/dist/index.html", ["file:///D:/vora/dist/index.html"]), true);
});

test("rejects external frames", () => {
  assert.equal(isAllowedFrameUrl("https://example.com/", ["http://127.0.0.1:1420/"]), false);
  assert.equal(isAllowedFrameUrl(undefined, ["http://127.0.0.1:1420/"]), false);
});

test("rejects unexpected shell status payload", () => {
  assert.deepEqual(validateShellStatusPayload(undefined), {});
  assert.throws(() => validateShellStatusPayload({ arbitrary: true }), /Invalid shell status payload/);
});

test("rejects arbitrary Sidecar IPC payloads", () => {
  const { validateSidecarIpcPayload } = require("../dist-electron/sidecar-ipc.js");
  assert.equal(validateSidecarIpcPayload(undefined), undefined);
  assert.throws(
    () => validateSidecarIpcPayload({ method: "shutdown" }),
    /Invalid Sidecar IPC payload/
  );
});

test("uses GitHub latest provider configuration", () => {
  const updaterModule = require("../dist-electron/update-config.js");
  assert.deepEqual(updaterModule.getGitHubUpdateFeed(), {
    provider: "github",
    owner: "alexfedura",
    repo: "vora_mind",
    private: false,
    channel: "latest"
  });
});
