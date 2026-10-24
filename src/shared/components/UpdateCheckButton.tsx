import { AlertCircle, Check, RefreshCw } from "lucide-react";
import { useUpdater } from "./UpdaterProvider";

interface UpdateCheckButtonProps {
  className: string;
}

export function UpdateCheckButton({ className }: UpdateCheckButtonProps): React.ReactElement {
  const { status, downloadProgress, notice, checkForUpdates, installAvailableUpdate } = useUpdater();
  const isBusy = status === "checking" || status === "downloading" || status === "prompting" || status === "installing";
  const title = notice
    ?? (status === "ready"
      ? "Update ready to install"
      : status === "prompting"
        ? "Confirming update installation"
        : "Check for updates");

  return (
    <button
      type="button"
      onClick={() => {
        if (status === "ready") void installAvailableUpdate();
        else void checkForUpdates();
      }}
      disabled={isBusy}
      title={title}
      aria-label={title}
      aria-live="polite"
      className={className}
    >
      {status === "ready" ? (
        <Check size={14} className="text-emerald-400" />
      ) : status === "downloading" || status === "prompting" || status === "installing" ? (
        <svg
          className={`h-4 w-4${downloadProgress === null ? " animate-spin" : ""}`}
          viewBox="0 0 16 16"
          role="img"
          aria-label={status === "prompting"
            ? "Confirming update installation"
            : downloadProgress === null
              ? "Downloading update"
              : `${Math.round(downloadProgress)}% downloaded`}
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
