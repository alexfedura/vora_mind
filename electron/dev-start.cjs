const { spawn } = require("node:child_process");
const http = require("node:http");

const devUrl = process.env.VORA_DEV_SERVER_URL || "http://localhost:1420";
const vite = process.platform === "win32"
  ? spawn(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", "pnpm dev:frontend"], { stdio: "inherit", cwd: process.cwd(), shell: false })
  : spawn("pnpm", ["dev:frontend"], { stdio: "inherit", cwd: process.cwd(), shell: false });

let electronProcess = null;
let stopped = false;

function stop() {
  if (stopped) return;
  stopped = true;
  vite.kill();
  if (electronProcess && electronProcess.exitCode === null) electronProcess.kill();
  process.exit(0);
}

async function waitForServer(timeoutMs = 30000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const reachable = await new Promise((resolve) => {
      const request = http.get(devUrl, (response) => {
        response.resume();
        resolve((response.statusCode ?? 500) < 500);
      });
      request.on("error", () => resolve(false));
      request.setTimeout(1000, () => {
        request.destroy();
        resolve(false);
      });
    });
    if (reachable) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Vite did not become ready at ${devUrl}`);
}

vite.on("exit", (code) => {
  if (!stopped) {
    console.error(`Vite exited before Electron startup with code ${code ?? "unknown"}`);
    stop();
  }
});

process.on("SIGINT", stop);
process.on("SIGTERM", stop);

waitForServer().then(() => {
  electronProcess = process.platform === "win32"
    ? spawn(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", "node_modules\\.bin\\electron.cmd . --no-sandbox"], {
        stdio: "inherit",
        cwd: process.cwd(),
        env: { ...process.env, VORA_DEV_SERVER_URL: devUrl }
      })
    : spawn("node_modules/.bin/electron", [".", "--no-sandbox"], {
        stdio: "inherit",
        cwd: process.cwd(),
        env: { ...process.env, VORA_DEV_SERVER_URL: devUrl }
      });
  electronProcess.on("exit", (code) => {
    if (!stopped) stop();
  });
}).catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  stop();
});
