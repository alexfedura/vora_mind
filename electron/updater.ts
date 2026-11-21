import { autoUpdater } from "electron-updater";
import { app } from "electron";
import { log } from "./logger";

export function configureUpdater(): void {
  const updateUrl = process.env.VORA_UPDATE_URL?.trim();
  if (!updateUrl) {
    log("warn", "Auto-updater is disabled: VORA_UPDATE_URL is not configured");
    return;
  }

  autoUpdater.autoDownload = true;
  autoUpdater.autoInstallOnAppQuit = true;
  autoUpdater.setFeedURL({
    provider: "generic",
    url: updateUrl,
    channel: "latest"
  });
  autoUpdater.on("checking-for-update", () => log("info", "Checking for updates"));
  autoUpdater.on("update-available", () => log("info", "Update available"));
  autoUpdater.on("update-not-available", () => log("info", "No update available"));
  autoUpdater.on("download-progress", (progress) => log("info", `Update download progress: ${Math.round(progress.percent)}%`));
  autoUpdater.on("update-downloaded", () => {
    log("info", "Update downloaded; installation requires quitAndInstall");
    autoUpdater.quitAndInstall(false, true);
  });
  autoUpdater.on("error", (error) => log("error", `Auto-updater error: ${error.message}`));

  app.whenReady().then(() => {
    void autoUpdater.checkForUpdatesAndNotify();
  }).catch((error: unknown) => {
    log("error", `Auto-updater startup failed: ${error instanceof Error ? error.message : String(error)}`);
  });
}
