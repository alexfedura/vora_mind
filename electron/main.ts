import { app, BrowserWindow, ipcMain, session } from "electron";
import path from "node:path";
import { authorizeInvoke, validateShellStatusPayload } from "./security";
import { log } from "./logger";
import { configureUpdater } from "./updater";

const DEV_URL = process.env.VORA_DEV_SERVER_URL ?? "http://localhost:1420";
let mainWindow: BrowserWindow | null = null;

function allowedUrls(window: BrowserWindow): readonly string[] {
  const urls = [DEV_URL, `${DEV_URL.replace(/\/+$/, "")}/`];
  if (app.isPackaged) {
    const fileUrl = pathToFileUrl(path.join(__dirname, "..", "dist", "index.html"));
    urls.push(fileUrl);
  }
  return urls;
}

function pathToFileUrl(filePath: string): string {
  const normalized = filePath.replace(/\\/g, "/");
  return `file://${normalized.startsWith("/") ? "" : "/"}${normalized}`;
}

function isAllowedNavigationUrl(url: string): boolean {
  if (app.isPackaged) {
    const rendererUrl = pathToFileUrl(path.join(__dirname, "..", "dist", "index.html"));
    return url === rendererUrl;
  }
  return url === DEV_URL || url === `${DEV_URL.replace(/\/+$/, "")}/`;
}

async function createWindow(): Promise<void> {
  if (mainWindow && !mainWindow.isDestroyed()) {
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
    return;
  }

  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    show: false,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webSecurity: true
    }
  });

  const window = mainWindow;
  window.once("ready-to-show", () => window.show());
  window.on("closed", () => {
    mainWindow = null;
    log("info", "Main window closed");
  });

  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event, url) => {
    if (!isAllowedNavigationUrl(url)) {
      event.preventDefault();
      log("warn", "Blocked Electron renderer navigation");
    }
  });

  session.defaultSession.setPermissionRequestHandler((_webContents, _permission, callback) => callback(false));
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    const csp = app.isPackaged
      ? "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: file:; font-src 'self' data:; connect-src 'self'"
      : "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: file:; font-src 'self' data:; connect-src 'self' ws://localhost:1420 http://localhost:1420";
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        "Content-Security-Policy": [csp]
      }
    });
  });

  if (!app.isPackaged) {
    await window.loadURL(DEV_URL);
  } else {
    await window.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }
  log("info", `Renderer loaded: ${window.webContents.getURL()}`);
  if (process.env.VORA_E2_SMOKE === "1") {
    try {
      const runtimeInfo = await window.webContents.executeJavaScript(
        "window.voraElectron.getRuntimeInfo()"
      );
      log("info", `Renderer runtime info: ${JSON.stringify(runtimeInfo)}`);
    } catch (error: unknown) {
      log("error", `Renderer runtime info failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      log("info", "E2 smoke completed");
      window.close();
      app.exit(0);
    }
  }
}

function registerIpc(): void {
  ipcMain.handle("vora:shell:status", (event: Electron.IpcMainInvokeEvent) => {
    if (!mainWindow) {
      throw Object.assign(new Error("Main window unavailable"), { code: "WINDOW_UNAVAILABLE" });
    }
    authorizeInvoke(event, mainWindow.webContents, allowedUrls(mainWindow));
    validateShellStatusPayload(undefined);
    return {
      ok: true,
      result: {
        electron: process.versions.electron,
        node: process.versions.node,
        platform: process.platform,
        arch: process.arch,
        packaged: app.isPackaged
      }
    };
  });
  ipcMain.handle("vora:shell:runtime-info", (event: Electron.IpcMainInvokeEvent) => {
    if (!mainWindow) {
      throw Object.assign(new Error("Main window unavailable"), { code: "WINDOW_UNAVAILABLE" });
    }
    authorizeInvoke(event, mainWindow.webContents, allowedUrls(mainWindow));
    validateShellStatusPayload(undefined);
    return {
      ok: true,
      result: {
        electron: process.versions.electron,
        node: process.versions.node,
        platform: process.platform,
        arch: process.arch,
        packaged: app.isPackaged,
        appVersion: app.getVersion()
      }
    };
  });
}

const hasSingleInstanceLock = app.requestSingleInstanceLock();
if (!hasSingleInstanceLock) {
  app.quit();
} else {
  app.on("second-instance", () => {
    log("info", "Second instance ignored");
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
  });

  app.whenReady().then(async () => {
    registerIpc();
    configureUpdater();
    await createWindow();
  }).catch((error: unknown) => {
    log("error", `Electron startup failed: ${error instanceof Error ? error.message : String(error)}`);
    app.quit();
  });
}

app.on("before-quit", () => {
  log("info", "Electron before-quit");
});

app.on("window-all-closed", () => {
  app.quit();
});
