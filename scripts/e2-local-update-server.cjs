const https = require("node:https");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(process.argv[2] || "artifacts/e2-update-test");
const keyPath = process.env.E2_UPDATE_TLS_KEY;
const certPath = process.env.E2_UPDATE_TLS_CERT;
const port = Number(process.env.E2_UPDATE_PORT || "8443");

if (!keyPath || !certPath) {
  console.error("E2_UPDATE_TLS_KEY and E2_UPDATE_TLS_CERT are required");
  process.exit(1);
}

const server = https.createServer({
  key: fs.readFileSync(keyPath),
  cert: fs.readFileSync(certPath)
}, (request, response) => {
  const requested = decodeURIComponent((request.url || "/").split("?")[0]);
  const safeName = path.basename(requested);
  const filePath = path.join(root, safeName);
  if (!safeName || !fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    response.statusCode = 404;
    response.end("Not found");
    return;
  }
  response.setHeader("Content-Length", fs.statSync(filePath).size);
  response.setHeader("Cache-Control", "no-store");
  fs.createReadStream(filePath).pipe(response);
});

server.listen(port, "127.0.0.1", () => {
  console.log(JSON.stringify({
    mode: "test-only",
    url: `https://127.0.0.1:${port}`,
    root
  }));
});
