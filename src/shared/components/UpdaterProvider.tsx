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
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";

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

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function UpdaterProvider({ children }: { children: ReactNode }): ReactElement {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);
  const busyRef = useRef(false);

  const reportError = useCallback((error: unknown, prefix: string): void => {
    const detail = getErrorMessage(error);
    const errorMessage = `${prefix}: ${detail}`;
    console.error(errorMessage, error);
    setStatus("error");
  }, []);

  const installAvailableUpdate = useCallback(async (): Promise<void> => {
    const update = updateRef.current;
    if (!update || busyRef.current) return;

    busyRef.current = true;
    setStatus("installing");
    setNotice(null);
    try {
      console.info(`Vora Mind updater: silently installing version ${update.version}.`);
      await update.install({ restartAfterInstall: true });
      updateRef.current = null;
      setStatus("idle");
      setDownloadProgress(null);
    } catch (error: unknown) {
      reportError(error, "Could not install the Vora Mind update");
      try {
        await update.close();
      } catch (closeError: unknown) {
        console.error("Could not release the Vora Mind update resource:", closeError);
      }
      updateRef.current = null;
    } finally {
      busyRef.current = false;
    }
  }, [reportError]);

  const checkForUpdates = useCallback(async (): Promise<void> => {
    if (import.meta.env.DEV) {
      if (!notice) setNotice("Update checks are available in installed desktop builds.");
      return;
    }
    if (busyRef.current || updateRef.current) return;

    busyRef.current = true;
    setStatus("checking");
    setDownloadProgress(null);
    setNotice(null);
    if (isWindows) {
      try {
        setStatus("downloading");
        const installedVersion = await invoke<string | null>("install_update_with_helper");
        if (installedVersion) {
          console.info(`Vora Mind updater: handing version ${installedVersion} to the detached installer helper.`);
          setStatus("installing");
        } else {
          console.info("Vora Mind updater: no update available.");
          setStatus("idle");
        }
      } catch (error: unknown) {
        reportError(error, "Could not check or install Vora Mind updates");
      } finally {
        busyRef.current = false;
      }
      return;
    }

    let update: Update | null = null;

    try {
      update = await check();
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

      updateRef.current = update;
      setStatus("ready");
      busyRef.current = false;
      await installAvailableUpdate();
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
    if (import.meta.env.DEV) return;
    void checkForUpdates();
    const interval = window.setInterval(() => {
      void checkForUpdates();
    }, AUTO_CHECK_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [checkForUpdates]);

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
