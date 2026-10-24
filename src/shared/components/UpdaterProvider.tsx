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
import { ask, message } from "@tauri-apps/plugin-dialog";
import { check, type Update } from "@tauri-apps/plugin-updater";

type UpdateStatus = "idle" | "checking" | "downloading" | "ready" | "prompting" | "installing" | "error";

interface UpdaterContextValue {
  status: UpdateStatus;
  downloadProgress: number | null;
  notice: string | null;
  checkForUpdates: () => Promise<void>;
  installAvailableUpdate: () => Promise<void>;
}

const UpdaterContext = createContext<UpdaterContextValue | null>(null);
const AUTO_CHECK_INTERVAL_MS = 60_000;

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function UpdaterProvider({ children }: { children: ReactNode }): ReactElement {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);
  const busyRef = useRef(false);

  const reportError = useCallback((error: unknown, prefix: string): string => {
    const detail = getErrorMessage(error);
    const errorMessage = `${prefix}: ${detail}`;
    console.error(errorMessage, error);
    setStatus("error");
    setNotice(errorMessage);
    return errorMessage;
  }, []);

  const installAvailableUpdate = useCallback(async (): Promise<void> => {
    const update = updateRef.current;
    if (!update || busyRef.current) return;

    busyRef.current = true;
    setStatus("prompting");
    try {
      let shouldInstall: boolean;
      try {
        shouldInstall = await ask(
          `Version ${update.version} is downloaded and ready to install. Restart now?`,
          { title: "Vora Mind update ready", kind: "info" },
        );
      } catch (error: unknown) {
        reportError(error, "Could not confirm installation of the Vora Mind update");
        setStatus("ready");
        return;
      }
      if (!shouldInstall) {
        setStatus("ready");
        return;
      }

      setStatus("installing");
      setNotice(null);
      try {
        await update.install({ restartAfterInstall: true });
        updateRef.current = null;
        setStatus("idle");
        setDownloadProgress(null);
      } catch (error: unknown) {
        const errorMessage = reportError(error, "Could not install the Vora Mind update");
        try {
          await message(
            `${errorMessage}\n\nThe current version is still installed. You can try the update again later.`,
            { title: "Vora Mind update failed", kind: "error" },
          );
        } catch (dialogError: unknown) {
          console.error("Could not display the Vora Mind update error dialog:", dialogError);
        }

        try {
          await update.close();
        } catch (closeError: unknown) {
          console.error("Could not release the Vora Mind update resource:", closeError);
        }
        updateRef.current = null;
      }
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
