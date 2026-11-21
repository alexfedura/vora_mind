import { ipcMain, type IpcMainInvokeEvent, type BrowserWindow } from "electron";
import { SidecarManager } from "./sidecar-manager";
import { authorizeInvoke } from "./security";

export function validateSidecarIpcPayload(payload: unknown): void {
  if (payload !== undefined) {
    throw Object.assign(new Error("Invalid Sidecar IPC payload"), { code: "INVALID_REQUEST" });
  }
}

export function registerSidecarIpc(manager: SidecarManager, allowedWindow: BrowserWindow): void {
  ipcMain.handle("vora:sidecar:status", (event: IpcMainInvokeEvent, payload?: unknown) => {
    authorizeInvoke(event, allowedWindow.webContents, getAllowedUrls(allowedWindow));
    validateSidecarIpcPayload(payload);
    return { ok: true, result: manager.getStatus() };
  });

  ipcMain.handle("vora:sidecar:engine-info", async (event: IpcMainInvokeEvent, payload?: unknown) => {
    authorizeInvoke(event, allowedWindow.webContents, getAllowedUrls(allowedWindow));
    validateSidecarIpcPayload(payload);
    const response = await manager.request("health");
    return { ok: response.ok, result: response.result, error: response.error };
  });

  ipcMain.handle("vora:sidecar:smoke", async (event: IpcMainInvokeEvent, payload?: unknown) => {
    authorizeInvoke(event, allowedWindow.webContents, getAllowedUrls(allowedWindow));
    validateSidecarIpcPayload(payload);
    await manager.request("create_synthetic_database", { database: ":memory:" });
    await manager.request("execute_approved_query", {
      database: ":memory:",
      sql: "CREATE TABLE smoke(value INTEGER); INSERT INTO smoke VALUES (1),(2),(3);"
    });
    const result = await manager.request("execute_approved_query", {
      database: ":memory:",
      sql: "SELECT COUNT(*) AS count FROM smoke"
    });
    return { ok: true, result };
  });
}

function getAllowedUrls(window: BrowserWindow): readonly string[] {
  const url = window.webContents.getURL();
  return url ? [url] : [];
}
