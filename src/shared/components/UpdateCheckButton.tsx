import { useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { AlertCircle, Check, RefreshCw } from "lucide-react";

type UpdateStatus = "idle" | "checking" | "up-to-date" | "downloading" | "installing" | "error";

interface UpdateCheckButtonProps {
  className: string;
}

export function UpdateCheckButton({ className }: UpdateCheckButtonProps): React.ReactElement {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const resetTimer = useRef<number | null>(null);

  useEffect(() => () => {
    if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
  }, []);

  const checkForUpdates = async (): Promise<void> => {
    if (status === "checking" || status === "downloading" || status === "installing") return;
    if (resetTimer.current !== null) {
      window.clearTimeout(resetTimer.current);
      resetTimer.current = null;
    }
    if (import.meta.env.DEV) {
      setStatus("error");
      setMessage("Update checks are available in installed desktop builds.");
      return;
    }

    setStatus("checking");
    setDownloadProgress(null);
    setMessage(null);
    let update: Update | null = null;

    try {
      update = await check();
      if (!update) {
        console.info("Vora Mind updater: no update available.");
        setStatus("up-to-date");
        resetTimer.current = window.setTimeout(() => {
          setStatus("idle");
          resetTimer.current = null;
        }, 1400);
        return;
      }

      console.info(`Vora Mind updater: version ${update.version} is available.`);
      setStatus("downloading");
      setMessage(`Downloading version ${update.version}`);
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

      setStatus("installing");
      setMessage(`Installing version ${update.version}`);
      await update.install({ restartAfterInstall: true });
    } catch (error: unknown) {
      console.error("Could not check or install Vora Mind updates:", error);
      setStatus("error");
      setMessage(error instanceof Error ? error.message : String(error));
      if (update) {
        try {
          await update.close();
        } catch (closeError: unknown) {
          console.error("Could not release the Vora Mind update resource:", closeError);
        }
      }
    }
  };

  const isBusy = status === "checking" || status === "downloading" || status === "installing";
  const title = message
    ?? (status === "up-to-date" ? "You are using the latest version" : "Check for updates");

  return (
    <button
      type="button"
      onClick={() => void checkForUpdates()}
      disabled={isBusy}
      title={title}
      aria-label={title}
      aria-live="polite"
      className={className}
    >
      {status === "up-to-date" ? (
        <Check size={14} className="text-emerald-400" />
      ) : status === "downloading" || status === "installing" ? (
        <svg
          className={`h-4 w-4${downloadProgress === null ? " animate-spin" : ""}`}
          viewBox="0 0 16 16"
          role="img"
          aria-label={downloadProgress === null ? "Downloading update" : `${Math.round(downloadProgress)}% downloaded`}
        >
          <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2" />
          <circle
            cx="8"
            cy="8"
            r="6.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeDasharray={40.84}
            strokeDashoffset={downloadProgress === null ? 30 : 40.84 * (1 - downloadProgress / 100)}
            transform="rotate(-90 8 8)"
            className="text-[#ffb74d]"
          />
        </svg>
      ) : status === "error" ? (
        <AlertCircle size={14} className="text-rose-400" />
      ) : (
        <RefreshCw size={13} className={status === "checking" ? "animate-spin" : ""} />
      )}
    </button>
  );
}
