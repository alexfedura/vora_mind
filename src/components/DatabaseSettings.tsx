import React, { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Check, Copy, Database, Download, FolderOpen } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { voraDb } from "@/services/voraDb";

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

interface DatabaseSettingsProps {
  variant?: "settings" | "policy";
}

export const DatabaseSettings: React.FC<DatabaseSettingsProps> = ({ variant = "settings" }) => {
  const [databasePath, setDatabasePath] = useState<string | null>(null);
  const [settingsPath, setSettingsPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    void Promise.all([
      voraDb.getCurrentDatabasePath(),
      voraDb.getSettingsFilePath(),
    ]).then(([nextDatabasePath, nextSettingsPath]) => {
      setDatabasePath(nextDatabasePath);
      setSettingsPath(nextSettingsPath);
    }).catch((cause: unknown) => {
      setError(getErrorMessage(cause));
    });
  }, []);

  const exportDatabaseBackup = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const backupPath = await save({
        title: "Export complete Vora database",
        defaultPath: "Vora-Mind-Full-Backup.vora",
        filters: [{ name: "Vora database backup", extensions: ["vora"] }],
      });
      if (!backupPath) return;
      await voraDb.exportDatabaseBackup(backupPath);
      setMessage("Complete database backup exported. Settings are stored separately in JSON.");
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const importDatabaseBackup = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const backupPath = await open({
        title: "Import a Vora database backup",
        multiple: false,
        filters: [{ name: "Vora database backup", extensions: ["vora"] }],
      });
      if (typeof backupPath !== "string") return;
      if (!window.confirm("This replaces all current workspace database data with the selected .vora backup. Your current settings JSON will be kept. Continue?")) return;
      await voraDb.importDatabaseBackup(backupPath);
      setMessage("Database backup imported. Restarting the workspace…");
      window.setTimeout(() => window.location.reload(), 500);
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const exportSettings = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const filePath = await save({
        title: "Export all Vora settings",
        defaultPath: "Vora-Mind-Settings.json",
        filters: [{ name: "Vora settings", extensions: ["json"] }],
      });
      if (!filePath) return;
      await voraDb.exportSettingsJson(filePath);
      setMessage("All workspace settings exported to JSON.");
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const importSettings = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const filePath = await open({
        title: "Import Vora settings",
        multiple: false,
        filters: [{ name: "Vora settings", extensions: ["json"] }],
      });
      if (typeof filePath !== "string") return;
      const result = await voraDb.importSettingsJson(filePath);
      setMessage(`Settings imported: ${result.added} added; ${result.alreadyPresent} existing keys kept.`);
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const copyPath = async (): Promise<void> => {
    if (!databasePath) return;
    setError(null);
    try {
      await navigator.clipboard.writeText(databasePath);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    }
  };

  const openDatabaseFolder = async (): Promise<void> => {
    if (!databasePath) return;
    setError(null);
    try {
      await revealItemInDir(databasePath);
    } catch (cause: unknown) {
      setError(getErrorMessage(cause));
    }
  };

  const isPolicyVariant = variant === "policy";

  return (
    <section className={isPolicyVariant
      ? "flex flex-col gap-2 text-[13px] text-[#d6dbe2]"
      : "flex flex-col gap-3 rounded-xl border border-[#242835] bg-[#1a1d25] p-4"
    }>
      <div className="flex items-center gap-2">
        <div className={isPolicyVariant
          ? "flex h-6 w-6 shrink-0 items-center justify-center text-[#ff9800]"
          : "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-cyan-400/20 bg-cyan-400/10 text-cyan-300"
        }>
          <Database size={16} aria-hidden="true" />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className={isPolicyVariant
            ? "text-[13px] font-medium text-[#d6dbe2]"
            : "text-xs font-semibold text-gray-100"
          }>
            {isPolicyVariant ? "Workspace storage" : "Workspace Storage"}
          </h3>
          {!isPolicyVariant && (
            <p className="mt-1 text-[11px] leading-relaxed text-gray-400">
              Workspace data and settings are stored in two files: this database and its companion JSON.
            </p>
          )}
        </div>
      </div>

      <div className={isPolicyVariant
        ? "flex min-w-0 items-center gap-1.5 rounded border border-[#383838] bg-[#0f0f0f] p-1.5"
        : "flex min-w-0 items-center gap-2 rounded-lg border border-[#2c3240] bg-[#11141a] p-2"
      }>
        <p className={isPolicyVariant
          ? "min-w-0 flex-1 break-all font-mono text-[11px] leading-relaxed text-[#d6dbe2]"
          : "min-w-0 flex-1 break-all font-mono text-[11px] leading-relaxed text-gray-300"
        } aria-label="Current database path">
          {databasePath ?? "Loading database path…"}
        </p>
        <button
          type="button"
          onClick={() => void openDatabaseFolder()}
          disabled={!databasePath || busy}
          title="Show database in File Explorer"
          aria-label="Show database in File Explorer"
          className={isPolicyVariant
            ? "flex h-7 w-7 shrink-0 items-center justify-center rounded text-[#9a9da5] transition hover:bg-[#2a2a2a] hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
            : "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-gray-400 transition hover:bg-[#252a36] hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
          }
        >
          <FolderOpen size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          onClick={() => void copyPath()}
          disabled={!databasePath || busy}
          title="Copy database path"
          aria-label="Copy database path"
          className={isPolicyVariant
            ? "flex h-7 w-7 shrink-0 items-center justify-center rounded text-[#9a9da5] transition hover:bg-[#2a2a2a] hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
            : "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-gray-400 transition hover:bg-[#252a36] hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
          }
        >
          {copied ? <Check size={15} className="text-emerald-300" /> : <Copy size={15} />}
        </button>
      </div>

      <div className="rounded-lg border border-[#2c3240] bg-[#11141a] p-2">
        <p className="text-[10px] font-medium uppercase tracking-wide text-gray-500">
          Workspace settings JSON
        </p>
        <p
          className="mt-1 break-all font-mono text-[11px] leading-relaxed text-gray-300"
          aria-label="Current settings file path"
        >
          {settingsPath ?? "Loading settings file path…"}
        </p>
      </div>

      {!isPolicyVariant && (
        <>
          <div className="rounded-lg border border-cyan-400/15 bg-[#11141a] p-3">
            <p className="text-[11px] font-semibold text-gray-200">Complete database · .vora</p>
            <p className="mt-1 text-[10px] leading-relaxed text-gray-400">
              Includes all SQLite workspace data and saved snapshots. It does not include settings JSON.
            </p>
            <div className="mt-2 flex flex-wrap gap-2">
              <button type="button" onClick={() => void exportDatabaseBackup()} disabled={busy} className="flex items-center gap-2 rounded-lg border border-cyan-500/30 bg-cyan-600/10 px-3 py-2 text-xs font-medium text-cyan-200 transition hover:bg-cyan-600/20 disabled:opacity-50">
                <Download size={14} /> Export .vora
              </button>
              <button type="button" onClick={() => void importDatabaseBackup()} disabled={busy} className="flex items-center gap-2 rounded-lg border border-amber-500/30 bg-amber-600/10 px-3 py-2 text-xs font-medium text-amber-200 transition hover:bg-amber-600/20 disabled:opacity-50">
                <Database size={14} /> Import .vora
              </button>
            </div>
          </div>
          <div className="rounded-lg border border-[#2c3240] bg-[#11141a] p-3">
            <p className="text-[11px] font-semibold text-gray-200">Complete settings · .json</p>
            <p className="mt-1 text-[10px] leading-relaxed text-gray-400">
              Export all settings. Import only adds missing keys and keeps current values when a key already exists.
            </p>
            <div className="mt-2 flex flex-wrap gap-2">
              <button type="button" onClick={() => void exportSettings()} disabled={busy} className="flex items-center gap-2 rounded-lg border border-violet-500/30 bg-violet-600/10 px-3 py-2 text-xs font-medium text-violet-200 transition hover:bg-violet-600/20 disabled:opacity-50">
                <Download size={14} /> Export .json
              </button>
              <button type="button" onClick={() => void importSettings()} disabled={busy} className="flex items-center gap-2 rounded-lg border border-violet-500/30 bg-violet-600/10 px-3 py-2 text-xs font-medium text-violet-200 transition hover:bg-violet-600/20 disabled:opacity-50">
                <FolderOpen size={14} /> Import .json
              </button>
            </div>
          </div>
        </>
      )}
      {message && (
        <p role="status" className={isPolicyVariant ? "text-[11px] leading-relaxed text-[#ffb74d]" : "text-[11px] text-green-300"}>
          {message}
        </p>
      )}
      {error && (
        <p role="alert" className={isPolicyVariant ? "break-words text-[11px] leading-relaxed text-[#ef9a9a]" : "break-words text-[11px] text-rose-300"}>
          {error}
        </p>
      )}
    </section>
  );
};
