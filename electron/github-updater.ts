import { autoUpdater } from "electron-updater";
import { app } from "electron";
import { getGitHubUpdateFeed } from "./update-config";

export function configureGitHubUpdater(
  log: (level: "info" | "warn" | "error", message: string) => void,
): void {
  if (!app.isPackaged) {
    log("warn", "Auto-updater disabled in development");
    return;
  }
  autoUpdater.autoDownload = true;
  autoUpdater.autoInstallOnAppQuit = true;
  autoUpdater.setFeedURL(getGitHubUpdateFeed());
  autoUpdater.on("checking-for-update", () => log("info", "Checking GitHub Releases"));
  autoUpdater.on("update-available", () => log("info", "GitHub update available"));
  autoUpdater.on("update-not-available", () => log("info", "No GitHub update available"));
  autoUpdater.on("download-progress", (progress) => log("info", `Update download progress: ${Math.round(progress.percent)}%`));
  autoUpdater.on("update-downloaded", () => {
    log("info", "GitHub update downloaded");
    autoUpdater.quitAndInstall(false, true);
  });
  autoUpdater.on("error", (error) => log("error", `GitHub updater error: ${error.message}`));
  void app.whenReady().then(() => autoUpdater.checkForUpdatesAndNotify());
}
