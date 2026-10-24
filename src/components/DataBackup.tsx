import React, { useEffect, useMemo, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Download, PackageOpen, RefreshCw, Upload } from "lucide-react";
import { dbService } from "@/mind/services/dbService";
import { listSnapshots, replaceSnapshots } from "@/lib/snapshot/storage";
import { flushSettings, refreshSettings } from "@/shared/store/useSettingsStore";
import { voraDb } from "@/services/voraDb";
import type { BackupCatalog, ImportedItemOrigin, PackageSelection } from "@/types";

const EMPTY_SELECTION: PackageSelection = {
  boards: [],
  sources: [],
  charts: [],
  chartSnapshots: [],
  libraryGroups: [],
  drawingTemplates: [],
  drawingAlerts: [],
  chartDrawings: [],
};

const PACKAGE_GROUPS: { key: keyof PackageSelection; label: string }[] = [
  { key: "boards", label: "Boards and their cards, paths, versions, and snapshots" },
  { key: "sources", label: "Sources and their segments, extracts, links, and revisions" },
  { key: "charts", label: "Saved charts" },
  { key: "chartSnapshots", label: "Chart snapshots" },
  { key: "libraryGroups", label: "Library item groups" },
  { key: "drawingTemplates", label: "Drawing templates" },
  { key: "drawingAlerts", label: "Drawing alerts" },
  { key: "chartDrawings", label: "Chart drawing sets" },
];

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const DataBackup: React.FC = () => {
  const [catalog, setCatalog] = useState<BackupCatalog | null>(null);
  const [selection, setSelection] = useState<PackageSelection>(EMPTY_SELECTION);
  const [packageName, setPackageName] = useState("");
  const [busy, setBusy] = useState<"load" | "export" | "import" | null>("load");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [origins, setOrigins] = useState<ImportedItemOrigin[]>([]);

  const selectedCount = useMemo(
    () => Object.values(selection).reduce((count, values) => count + values.length, 0),
    [selection],
  );

  const loadCatalog = async (): Promise<void> => {
    setBusy("load");
    setError(null);
    try {
      const [nextCatalog, nextOrigins] = await Promise.all([
        voraDb.getBackupCatalog(),
        voraDb.getImportedItemOrigins(),
      ]);
      setCatalog(nextCatalog);
      setOrigins(nextOrigins);
    } catch (cause: unknown) {
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  };

  useEffect(() => {
    void loadCatalog();
  }, []);

  const toggleSelection = (category: keyof PackageSelection, id: string): void => {
    setSelection((current) => {
      const selected = current[category];
      return {
        ...current,
        [category]: selected.includes(id)
          ? selected.filter((selectedId) => selectedId !== id)
          : [...selected, id],
      };
    });
  };

  const exportPackage = async (): Promise<void> => {
    if (!packageName.trim() || selectedCount === 0) return;
    setBusy("export");
    setMessage(null);
    setError(null);
    try {
      const filePath = await save({
        title: "Export selected Vora items",
        defaultPath: `${packageName.trim()}.vr`,
        filters: [{ name: "Vora selected-item package", extensions: ["vr"] }],
      });
      if (!filePath) return;
      await flushSettings();
      await voraDb.exportSelectivePackage(filePath, packageName, selection);
      setMessage(`"${packageName.trim()}" exported as a selective .vr package.`);
    } catch (cause: unknown) {
      console.error("Could not export selected Vora items:", cause);
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  };

  const importLegacyFullBackup = async (filePath: string): Promise<void> => {
    const confirmed = window.confirm(
      "This is an older full-workspace .vr backup. Restoring it replaces the current database, chart settings, drawings, and saved snapshots. To add selected content instead, import a selective package. Continue with full restore?",
    );
    if (!confirmed) return;

    const backupStorage = await dbService.readBoardBackupStorage(filePath);
    const previousSnapshots = backupStorage === null ? null : await listSnapshots();
    try {
      if (backupStorage !== null) {
        await replaceSnapshots(backupStorage.snapshots);
      }
      await dbService.importBoardFromFile(filePath);
      await refreshSettings();
    } catch (cause: unknown) {
      if (backupStorage !== null) {
        try {
          await replaceSnapshots(previousSnapshots ?? []);
        } catch (rollbackCause: unknown) {
          console.error("Could not roll back browser data after backup import failed:", rollbackCause);
        }
      }
      throw cause;
    }
    window.alert("Full backup restored. Vora Mind will now reload.");
    window.location.reload();
  };

  const importPackage = async (): Promise<void> => {
    setBusy("import");
    setMessage(null);
    setError(null);
    try {
      const selected = await open({
        title: "Import a Vora package",
        multiple: false,
        filters: [{ name: "Vora package", extensions: ["vr"] }],
      });
      if (typeof selected !== "string") return;

      const info = await voraDb.inspectBackupFile(selected);
      if (info.kind === "legacy-full") {
        await importLegacyFullBackup(selected);
        return;
      }
      if (!window.confirm(
        `Add ${info.recordCount} item(s) from "${info.packageName ?? "Vora package"}"? Existing items will be kept and duplicates skipped.`,
      )) return;

      const result = await voraDb.importSelectivePackage(selected);
      await refreshSettings();
      setMessage(
        `"${result.packageName}" imported: ${result.added} added; ${result.skipped} existing or duplicate items skipped.`,
      );
      const nextOrigins = await voraDb.getImportedItemOrigins();
      setOrigins(nextOrigins);
      await loadCatalog();
    } catch (cause: unknown) {
      console.error("Could not import Vora package:", cause);
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  };

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-[#242835] bg-[#1a1d25] p-4">
      <div>
        <h3 className="text-xs font-semibold text-gray-100">Share selected items (.vr)</h3>
        <p className="mt-1 text-[11px] leading-relaxed text-gray-400">
          Choose workspace content to share. Import adds missing items without replacing your workspace; duplicates are skipped.
        </p>
      </div>

      <label className="flex flex-col gap-1 text-[11px] text-gray-300">
        Package name
        <input
          value={packageName}
          onChange={(event) => setPackageName(event.target.value)}
          maxLength={100}
          placeholder="For example, Research board"
          className="rounded-lg border border-[#2c3240] bg-[#11141a] px-3 py-2 text-xs text-gray-100 outline-none focus:border-violet-500/60"
        />
      </label>

      <div className="max-h-64 overflow-y-auto rounded-lg border border-[#2c3240] bg-[#11141a]">
        {catalog === null && (
          <p className="p-3 text-[11px] text-gray-400">
            {busy === "load" ? "Loading items…" : "The item catalog is unavailable."}
          </p>
        )}
        {catalog && PACKAGE_GROUPS.map(({ key, label }) => {
          const records = catalog[key];
          return (
            <fieldset key={key} className="border-b border-[#252a36] p-3 last:border-b-0">
              <legend className="max-w-full px-1 text-[10px] font-semibold text-gray-300">
                {label} <span className="text-gray-500">({records.length})</span>
              </legend>
              {records.length === 0 ? (
                <p className="px-1 py-1 text-[10px] text-gray-500">Nothing to select.</p>
              ) : (
                <div className="flex flex-col gap-1">
                  {records.map((record) => (
                    <label
                      key={record.id}
                      className="flex cursor-pointer items-start gap-2 rounded px-1 py-1 hover:bg-white/5"
                    >
                      <input
                        type="checkbox"
                        checked={selection[key].includes(record.id)}
                        onChange={() => toggleSelection(key, record.id)}
                        className="mt-0.5 accent-violet-500"
                      />
                      <span className="min-w-0">
                        <span className="block truncate text-[11px] text-gray-200">{record.name}</span>
                        {record.detail && <span className="block truncate text-[10px] text-gray-500">{record.detail}</span>}
                      </span>
                    </label>
                  ))}
                </div>
              )}
            </fieldset>
          );
        })}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => void exportPackage()}
          disabled={busy !== null || !packageName.trim() || selectedCount === 0}
          className="flex items-center gap-2 rounded-lg border border-violet-500/30 bg-violet-600/15 px-3 py-2 text-xs font-medium text-violet-200 transition hover:bg-violet-600/25 disabled:opacity-50"
        >
          <Download size={14} />
          {busy === "export" ? "Exporting…" : `Export selected (${selectedCount})`}
        </button>
        <button
          type="button"
          onClick={() => void importPackage()}
          disabled={busy !== null}
          className="flex items-center gap-2 rounded-lg border border-amber-500/30 bg-amber-600/10 px-3 py-2 text-xs font-medium text-amber-200 transition hover:bg-amber-600/20 disabled:opacity-50"
        >
          <PackageOpen size={14} />
          {busy === "import" ? "Importing…" : "Import .vr package"}
        </button>
        <button
          type="button"
          onClick={() => void loadCatalog()}
          disabled={busy !== null}
          aria-label="Refresh package item catalog"
          title="Refresh item list"
          className="flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition hover:bg-white/5 hover:text-white disabled:opacity-50"
        >
          <RefreshCw size={14} />
        </button>
      </div>

      {message && <p role="status" className="text-[11px] text-green-300">{message}</p>}
      {error && <p role="alert" className="break-words text-[11px] text-rose-300">{error}</p>}

      <div className="border-t border-[#2a2e39] pt-3">
        <h4 className="mb-2 flex items-center gap-1.5 text-[11px] font-semibold text-gray-300">
          <Upload size={13} />
          Imported from shared packages
        </h4>
        {origins.length === 0 ? (
          <p className="text-[10px] text-gray-500">Imported items and their source packages will appear here.</p>
        ) : (
          <ul className="flex max-h-28 flex-col gap-1 overflow-y-auto">
            {origins.slice(0, 20).map((origin) => (
              <li
                key={`${origin.entityType}:${origin.entityId}`}
                className="flex min-w-0 items-center justify-between gap-2 text-[10px]"
              >
                <span className="min-w-0 truncate text-gray-300">
                  {origin.displayName} <span className="text-gray-500">({origin.entityType})</span>
                </span>
                <span className="shrink-0 text-gray-500">from {origin.sourcePackageName}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
};
