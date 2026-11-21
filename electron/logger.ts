import { app } from "electron";
import fs from "node:fs";
import path from "node:path";

let logPath: string | null = null;

export function getLogPath(): string {
  if (!logPath) {
    const logDir = app.isReady() ? app.getPath("logs") : path.join(app.getPath("userData"), "logs");
    fs.mkdirSync(logDir, { recursive: true });
    logPath = path.join(logDir, "vora-electron.log");
  }
  return logPath;
}

export function log(level: "info" | "warn" | "error", message: string): void {
  const entry = `${new Date().toISOString()} [${level}] ${message}\n`;
  fs.appendFileSync(getLogPath(), entry, "utf8");
  if (level === "error") console.error(message);
  else if (level === "warn") console.warn(message);
  else console.log(message);
}
