import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { app } from "electron";
import fs from "node:fs";
import path from "node:path";
import {
  SidecarRequestSchema,
  SidecarResponseSchema,
  type SidecarRequest,
  type SidecarResponse
} from "./sidecar-protocol";

interface PendingRequest {
  resolve: (response: SidecarResponse) => void;
  reject: (error: Error & { code?: string }) => void;
  timer: NodeJS.Timeout;
}

export interface SidecarManagerOptions {
  executable?: string;
  startupTimeoutMs?: number;
  requestTimeoutMs?: number;
  shutdownTimeoutMs?: number;
  maxPending?: number;
  maxStdoutLineBytes?: number;
}

export class SidecarManager {
  private process: ChildProcessWithoutNullStreams | null = null;
  private pending = new Map<string, PendingRequest>();
  private buffer = "";
  private sequence = 0;
  private stderrBuffer = "";
  private startupPromise: Promise<void> | null = null;
  private shutdownPromise: Promise<void> | null = null;
  private readonly options: Required<Omit<SidecarManagerOptions, "executable">> & Pick<SidecarManagerOptions, "executable">;

  constructor(options: SidecarManagerOptions = {}) {
    this.options = {
      executable: options.executable,
      startupTimeoutMs: options.startupTimeoutMs ?? 5000,
      requestTimeoutMs: options.requestTimeoutMs ?? 5000,
      shutdownTimeoutMs: options.shutdownTimeoutMs ?? 1000,
      maxPending: options.maxPending ?? 64,
      maxStdoutLineBytes: options.maxStdoutLineBytes ?? 1024 * 1024
    };
  }

  resolveExecutable(): string {
    if (this.options.executable) return this.options.executable;
    if (app.isPackaged) {
      return path.join(process.resourcesPath, "sidecar", "bin", "vora_e021_sidecar.exe");
    }
    return path.join(app.getAppPath(), "electron", "sidecar", "bin", "vora_e021_sidecar.exe");
  }

  setExecutable(executable: string): void {
    if (this.process && this.process.exitCode === null) {
      throw new Error("Cannot change executable while Sidecar is running");
    }
    this.options.executable = executable;
  }

  start(): Promise<void> {
    if (this.process && this.process.exitCode === null) {
      return this.startupPromise ?? Promise.resolve();
    }
    if (this.startupPromise) return this.startupPromise;

    const executable = this.resolveExecutable();
    if (!fs.existsSync(executable)) {
      return Promise.reject(Object.assign(
        new Error(`Sidecar not found: ${executable}`),
        { code: "SIDECAR_START_FAILED" }
      ));
    }

    const spawned = spawn(executable, [], {
      shell: false,
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"]
    });
    this.process = spawned;
    spawned.stdout.setEncoding("utf8");
    spawned.stderr.setEncoding("utf8");
    spawned.stdout.on("data", (chunk: string) => this.onStdout(chunk));
    spawned.stdout.once("close", () => {
      if (this.process !== spawned) return;
      if (this.buffer.length > 0) {
        this.rejectPending(Object.assign(
          new Error("Sidecar stdout closed with partial protocol data"),
          { code: "SIDECAR_PROTOCOL_ERROR" }
        ));
      }
    });
    spawned.stderr.on("data", (chunk: string) => {
      this.stderrBuffer = (this.stderrBuffer + chunk).slice(-32768);
      console.error(`[sidecar stderr] ${chunk.trim()}`);
    });
    spawned.on("exit", (code, signal) => {
      if (this.process !== spawned) return;
      this.rejectPending(Object.assign(
        new Error(`Sidecar exited with code ${code ?? "unknown"} signal ${signal ?? "none"}`),
        { code: "SIDECAR_EXITED" }
      ));
      this.process = null;
      this.startupPromise = null;
    });

    this.startupPromise = new Promise<void>((resolve, reject) => {
      let settled = false;
      const timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        spawned.kill();
        this.startupPromise = null;
        reject(Object.assign(new Error("Sidecar startup timed out"), {
          code: "SIDECAR_STARTUP_TIMEOUT"
        }));
      }, this.options.startupTimeoutMs);

      spawned.once("error", (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        this.startupPromise = null;
        reject(Object.assign(new Error(`Sidecar could not start: ${error.message}`), {
          code: "SIDECAR_START_FAILED"
        }));
      });
      spawned.once("exit", (code, signal) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        this.startupPromise = null;
        reject(Object.assign(new Error(`Sidecar exited during startup with code ${code ?? "unknown"} signal ${signal ?? "none"}`), {
          code: "SIDECAR_START_FAILED"
        }));
      });

      this.request("health").then(() => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve();
      }).catch((error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        if (spawned.exitCode === null) spawned.kill();
        this.startupPromise = null;
        reject(error);
      });
    });
    return this.startupPromise;
  }

  private rejectPending(error: Error & { code?: string }): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
    this.buffer = "";
  }

  private onStdout(chunk: string): void {
    this.buffer += chunk;
    let newline = this.buffer.indexOf("\n");
    while (newline >= 0) {
      const line = this.buffer.slice(0, newline).replace(/\r$/, "");
      this.buffer = this.buffer.slice(newline + 1);
      if (line.length > this.options.maxStdoutLineBytes) {
        this.rejectPending(Object.assign(
          new Error("Sidecar emitted an oversized protocol line"),
          { code: "SIDECAR_PROTOCOL_ERROR" }
        ));
        return;
      }
      if (this.buffer.length > this.options.maxStdoutLineBytes && this.buffer.indexOf("\n") < 0) {
        this.rejectPending(Object.assign(
          new Error("Sidecar emitted an oversized protocol line"),
          { code: "SIDECAR_PROTOCOL_ERROR" }
        ));
        return;
      }
      if (line.length > 0) this.parseLine(line);
      newline = this.buffer.indexOf("\n");
    }
    if (this.buffer.length > this.options.maxStdoutLineBytes) {
      this.rejectPending(Object.assign(
        new Error("Sidecar emitted an oversized protocol line"),
        { code: "SIDECAR_PROTOCOL_ERROR" }
      ));
    }
  }

  private parseLine(line: string): void {
    let parsedJson: unknown;
    try {
      parsedJson = JSON.parse(line);
    } catch {
      this.rejectPending(Object.assign(
        new Error("Sidecar emitted malformed JSON"),
        { code: "SIDECAR_PROTOCOL_ERROR" }
      ));
      console.error("[sidecar] malformed JSON output");
      return;
    }

    const parsed = SidecarResponseSchema.safeParse(parsedJson);
    if (!parsed.success) {
      const raw = typeof parsedJson === "object" && parsedJson !== null
        ? parsedJson as Record<string, unknown>
        : {};
      const requestId = typeof raw.requestId === "string" ? raw.requestId : "";
      if (requestId) {
        const pending = this.pending.get(requestId);
        if (pending) {
          clearTimeout(pending.timer);
          this.pending.delete(requestId);
          pending.reject(Object.assign(new Error("Malformed Sidecar response"), {
            code: "SIDECAR_PROTOCOL_ERROR"
          }));
        }
      }
      console.error("[sidecar] malformed or unmatched response");
      return;
    }

    const response: SidecarResponse = parsed.data;
    const pending = this.pending.get(response.requestId);
    if (!pending) {
      console.error(`[sidecar] unmatched response ID ${response.requestId}`);
      return;
    }
    clearTimeout(pending.timer);
    this.pending.delete(response.requestId);
    pending.resolve(response);
  }

  request(method: SidecarRequest["method"], params: Record<string, unknown> = {}): Promise<SidecarResponse> {
    if (!this.process || this.process.exitCode !== null) {
      return Promise.reject(Object.assign(new Error("Sidecar is not running"), { code: "SIDECAR_NOT_STARTED" }));
    }
    if (this.pending.size >= this.options.maxPending) {
      return Promise.reject(Object.assign(new Error("Too many pending Sidecar requests"), { code: "SIDECAR_QUEUE_FULL" }));
    }
    const requestId = `e24-${++this.sequence}`;
    const parsed = SidecarRequestSchema.safeParse({
      protocolVersion: 1,
      requestId,
      method,
      params
    });
    if (!parsed.success) {
      return Promise.reject(Object.assign(new Error("Invalid Sidecar request"), { code: "INVALID_REQUEST" }));
    }

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        if (this.process && this.process.exitCode === null) {
          this.process.kill();
        }
        reject(Object.assign(new Error("Sidecar request timed out"), { code: "SIDECAR_TIMEOUT" }));
      }, this.options.requestTimeoutMs);
      this.pending.set(requestId, { resolve, reject, timer });
      this.process!.stdin.write(JSON.stringify(parsed.data) + "\n");
    });
  }

  async shutdown(): Promise<void> {
    if (this.shutdownPromise) return this.shutdownPromise;
    if (!this.process || this.process.exitCode !== null) return;
    const processIdentity = this.process;
    this.shutdownPromise = new Promise<void>((resolve) => {
      let resolved = false;
      const complete = () => {
        if (resolved) return;
        resolved = true;
        clearTimeout(shutdownTimer);
        if (this.process === processIdentity) this.process = null;
        this.shutdownPromise = null;
        resolve();
      };
      const shutdownTimer = setTimeout(() => {
        if (processIdentity.exitCode === null) processIdentity.kill("SIGKILL");
      }, this.options.shutdownTimeoutMs);
      processIdentity.once("exit", complete);
      void this.request("shutdown", {}).catch(() => undefined).finally(() => {
        if (processIdentity.exitCode === null) processIdentity.stdin.end();
      });
    });
    return this.shutdownPromise;
  }

  getStatus(): { alive: boolean; pending: number } {
    return {
      alive: Boolean(this.process && this.process.exitCode === null),
      pending: this.pending.size
    };
  }

  getRecentStderr(): string {
    return this.stderrBuffer;
  }
}
