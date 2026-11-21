const test = require("node:test");
const assert = require("node:assert/strict");
const { isAllowedFrameUrl, validateShellStatusPayload } = require("../dist-electron/security.js");

test("allows exact development and file frame URLs", () => {
  assert.equal(isAllowedFrameUrl("http://127.0.0.1:1420/", ["http://127.0.0.1:1420/"]), true);
  assert.equal(isAllowedFrameUrl("file:///D:/vora/dist/renderer/index.html", ["file:///D:/vora/dist/renderer/index.html"]), true);
});

test("rejects external frames", () => {
  assert.equal(isAllowedFrameUrl("https://example.com/", ["http://127.0.0.1:1420/"]), false);
  assert.equal(isAllowedFrameUrl(undefined, ["http://127.0.0.1:1420/"]), false);
});

test("rejects unexpected shell status payload", () => {
  assert.deepEqual(validateShellStatusPayload(undefined), {});
  assert.throws(() => validateShellStatusPayload({ arbitrary: true }), /Invalid shell status payload/);
});
