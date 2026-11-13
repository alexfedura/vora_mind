import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { updaterService } from "@/services/updaterService";
import {
  AUTOMATIC_UPDATES_SETTING_KEY,
  useSettingsStore,
} from "@/shared/store/useSettingsStore";

type UpdateStatus = "idle" | "checking" | "downloading" | "ready" | "installing" | "error";

interface UpdaterContextValue {
  status: UpdateStatus;
  downloadProgress: number | null;
  notice: string | null;
  checkForUpdates: () => Promise<void>;
  installAvailableUpdate: () => Promise<void>;
}

const UpdaterContext = createContext<UpdaterContextValue | null>(null);
const AUTO_CHECK_INTERVAL_MS = 60_000;
const isWindows = navigator.userAgent.toLowerCase().includes("windows");

interface UpdaterDownloadProgress {
  downloadedBytes: number;
  contentLength: number | null;
}

interface UpdateFailureRecord {
  lastFailedVersion: string;
  failedAt: number;
}

const UPDATE_FAILURE_STORAGE_KEY = "vora.updater.lastFailedVersion";
const UPDATE_FAILURE_COOLDOWN_MS = 60 * 60 * 1_000;
const UPDATE_FAILURE_NOTICE = "Update failed. Please check the logs or try again later.";
const UPDATE_READY_NOTICE =
  "An update is available. Click the update button to install and restart Vora Mind.";

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function readUpdateFailureRecord(): UpdateFailureRecord | null {
  const rawRecord = localStorage.getItem(UPDATE_FAILURE_STORAGE_KEY);
  if (!rawRecord) return null;

  let parsed: unknown;
  try {
    parsed = JSON.parse(rawRecord);
  } catch (error: unknown) {
    console.error("Could not parse the stored updater failure record:", error);
    throw new Error("The stored updater failure record is invalid.");
  }

  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("lastFailedVersion" in parsed) ||
    typeof parsed.lastFailedVersion !== "string" ||
    !("failedAt" in parsed) ||
    typeof parsed.failedAt !== "number"
  ) {
    throw new Error("The stored updater failure record has an invalid shape.");
  }

  return {
    lastFailedVersion: parsed.lastFailedVersion,
    failedAt: parsed.failedAt,
  };
}

function writeUpdateFailureRecord(version: string, failedAt = Date.now()): void {
  localStorage.setItem(
    UPDATE_FAILURE_STORAGE_KEY,
    JSON.stringify({ lastFailedVersion: version, failedAt }),
  );
}

function isRecentFailure(record: UpdateFailureRecord | null, version: string): boolean {
  if (!record || record.lastFailedVersion !== version) return false;
  const elapsed = Date.now() - record.failedAt;
  return elapsed >= 0 && elapsed < UPDATE_FAILURE_COOLDOWN_MS;
}

export function UpdaterProvider({ children }: { children: ReactNode }): ReactElement {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);
  const pendingWindowsVersionRef = useRef<string | null>(null);
  const busyRef = useRef(false);
  const automaticUpdatesEnabled = useSettingsStore(
    (state) => state.values[AUTOMATIC_UPDATES_SETTING_KEY] !== "false",
  );
  const automaticUpdatesEnabledRef = useRef(automaticUpdatesEnabled);
  automaticUpdatesEnabledRef.current = automaticUpdatesEnabled;

  const reportError = useCallback((error: unknown, prefix: string): void => {
    const detail = getErrorMessage(error);
    const errorMessage = `${prefix}: ${detail}`;
    console.error(errorMessage, error);
    setStatus("error");
  }, []);

  const installAvailableUpdate = useCallback(async (): Promise<void> => {
    const update = updateRef.current;
    const windowsVersion = pendingWindowsVersionRef.current;
    const requestedVersion = windowsVersion ?? update?.version;
    if (!requestedVersion || busyRef.current) return;

    busyRef.current = true;
    let attemptedInstallation = false;
    try {
      const failureRecord = readUpdateFailureRecord();
      if (isRecentFailure(failureRecord, requestedVersion)) {
        setStatus("error");
        setNotice(UPDATE_FAILURE_NOTICE);
        await update?.close();
        updateRef.current = null;
        pendingWindowsVersionRef.current = null;
        return;
      }
      setStatus("installing");
      setNotice(null);
      console.info(`Vora Mind updater: installing version ${requestedVersion}.`);
      attemptedInstallation = true;
      if (windowsVersion) {
        const installedVersion = await updaterService.install(windowsVersion);
        if (!installedVersion) {
          pendingWindowsVersionRef.current = null;
          setStatus("idle");
          setDownloadProgress(null);
          return;
        }
      } else if (update) {
        await update.install({ restartAfterInstall: true });
      }
      try {
        localStorage.removeItem(UPDATE_FAILURE_STORAGE_KEY);
      } catch (storageError: unknown) {
        console.error("Could not clear the Vora Mind updater failure record:", storageError);
      }
      updateRef.current = null;
      pendingWindowsVersionRef.current = null;
      setStatus("idle");
      setDownloadProgress(null);
    } catch (error: unknown) {
      if (attemptedInstallation) {
        try {
          writeUpdateFailureRecord(requestedVersion);
        } catch (storageError: unknown) {
          console.error("Could not save the Vora Mind updater failure record:", storageError);
        }
      }
      reportError(
        error,
        attemptedInstallation
          ? "Could not install the Vora Mind update"
          : "Could not read the Vora Mind updater failure record",
      );
      if (attemptedInstallation) setNotice(UPDATE_FAILURE_NOTICE);
      try {
        await update?.close();
      } catch (closeError: unknown) {
        console.error("Could not release the Vora Mind update resource:", closeError);
      }
      updateRef.current = null;
      pendingWindowsVersionRef.current = null;
    } finally {
      busyRef.current = false;
    }
  }, [reportError]);

  const checkForUpdates = useCallback(async (automatic = false): Promise<void> => {
    if (automatic && !automaticUpdatesEnabledRef.current) return;
    if (import.meta.env.DEV) {
      if (!notice) setNotice("Update checks are available in installed desktop builds.");
      return;
    }
    if (busyRef.current || updateRef.current || pendingWindowsVersionRef.current) return;

    busyRef.current = true;
    setStatus("checking");
    setDownloadProgress(null);
    if (isWindows) {
      let update: Update | null = null;
      let requestedVersion: string | null = null;
      try {
        update = await check();
        if (automatic && !automaticUpdatesEnabledRef.current) {
          await update?.close();
          setStatus("idle");
          return;
        }
        if (!update) {
          console.info("Vora Mind updater: no update available.");
          setStatus("idle");
          setNotice(null);
          return;
        }

        requestedVersion = update.version;
        const failureRecord = readUpdateFailureRecord();
        if (isRecentFailure(failureRecord, requestedVersion)) {
          setStatus("error");
          setNotice(UPDATE_FAILURE_NOTICE);
          await update.close();
          update = null;
          return;
        }

        setNotice(null);
        await update.close();
        update = null;
        if (automatic && !automaticUpdatesEnabledRef.current) {
          setStatus("idle");
          return;
        }
        pendingWindowsVersionRef.current = requestedVersion;
        setNotice(UPDATE_READY_NOTICE);
        setStatus("ready");
      } catch (error: unknown) {
        if (requestedVersion) {
          try {
            writeUpdateFailureRecord(requestedVersion);
          } catch (storageError: unknown) {
            console.error("Could not save the Vora Mind updater failure record:", storageError);
          }
        }
        if (update) {
          try {
            await update.close();
          } catch (closeError: unknown) {
            console.error("Could not release the Vora Mind update resource:", closeError);
          }
        }
        reportError(error, "Could not check or install Vora Mind updates");
      } finally {
        busyRef.current = false;
      }
      return;
    }

    let update: Update | null = null;

    try {
      setNotice(null);
      update = await check();
      if (automatic && !automaticUpdatesEnabledRef.current) {
        await update?.close();
        update = null;
        setStatus("idle");
        return;
      }
      if (!update) {
        console.info("Vora Mind updater: no update available.");
        setStatus("idle");
        return;
      }

      console.info(`Vora Mind updater: version ${update.version} is available.`);
      setStatus("downloading");
      let downloadedBytes = 0;
      let contentLength: number | undefined;
      await update.download((event) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength;
          setDownloadProgress(contentLength ? 0 : null);
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (contentLength && contentLength > 0) {
            setDownloadProgress(Math.min(100, (downloadedBytes / contentLength) * 100));
          }
        } else if (event.event === "Finished") {
          setDownloadProgress(100);
        }
      });

      if (automatic && !automaticUpdatesEnabledRef.current) {
        await update.close();
        update = null;
        setStatus("idle");
        setDownloadProgress(null);
        return;
      }
      updateRef.current = update;
      setNotice(UPDATE_READY_NOTICE);
      setStatus("ready");
    } catch (error: unknown) {
      reportError(error, "Could not check or download Vora Mind updates");
      if (update && updateRef.current !== update) {
        try {
          await update.close();
        } catch (closeError: unknown) {
          console.error("Could not release the Vora Mind update resource:", closeError);
        }
      } else if (updateRef.current === update) {
        setStatus("ready");
      }
    } finally {
      busyRef.current = false;
    }
  }, [installAvailableUpdate, reportError]);

  useEffect(() => {
    if (!isWindows) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    void listen<UpdaterDownloadProgress>("vora-updater-progress", (event) => {
      if (!active) return;
      const { downloadedBytes, contentLength } = event.payload;
      setDownloadProgress(
        contentLength && contentLength > 0
          ? Math.min(100, (downloadedBytes / contentLength) * 100)
          : null,
      );
    }).then((stopListening) => {
      if (active) unlisten = stopListening;
      else stopListening();
    }).catch((error: unknown) => {
      console.error("Could not listen for updater download progress:", error);
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (import.meta.env.DEV || !automaticUpdatesEnabled) return;
    let active = true;
    let interval: number | undefined;
    const initialize = async (): Promise<void> => {
      if (isWindows) {
        try {
          const marker = await updaterService.getLastFailureMarker();
          if (
            marker !== null &&
            (typeof marker.version !== "string" ||
              typeof marker.failedAt !== "number" ||
              !Number.isFinite(marker.failedAt))
          ) {
            throw new Error("The updater returned an invalid failure marker.");
          }
          if (active && marker) {
            const storedRecord = readUpdateFailureRecord();
            if (!storedRecord || marker.failedAt > storedRecord.failedAt) {
              writeUpdateFailureRecord(marker.version, marker.failedAt);
            }
          }
        } catch (error: unknown) {
          if (!active) return;
          reportError(error, "Could not load the Vora Mind updater failure record");
          setNotice(UPDATE_FAILURE_NOTICE);
          return;
        }
      }
      if (!active) return;
      void checkForUpdates(true);
      interval = window.setInterval(() => {
        void checkForUpdates(true);
      }, AUTO_CHECK_INTERVAL_MS);
    };
    void initialize();
    return () => {
      active = false;
      if (interval !== undefined) window.clearInterval(interval);
    };
  }, [automaticUpdatesEnabled, checkForUpdates, reportError]);

  return (
    <UpdaterContext.Provider
      value={{ status, downloadProgress, notice, checkForUpdates, installAvailableUpdate }}
    >
      {children}
      {notice && (
        <div
          role="status"
          aria-live="polite"
          className="fixed bottom-5 left-1/2 z-[100] max-w-[min(90vw,520px)] -translate-x-1/2 rounded border border-[#343a46] bg-[#181818] px-3 py-2 text-xs text-[#d6dbe2] shadow-lg"
        >
          <span>{notice}</span>
          <button
            type="button"
            className="ml-3 text-[#aeb4c0] hover:text-white"
            aria-label="Dismiss updater notification"
            onClick={() => setNotice(null)}
          >
            Dismiss
          </button>
        </div>
      )}
    </UpdaterContext.Provider>
  );
}

export function useUpdater(): UpdaterContextValue {
  const context = useContext(UpdaterContext);
  if (!context) throw new Error("useUpdater must be used within an UpdaterProvider.");
  return context;
}
