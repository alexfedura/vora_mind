import { invoke } from "@tauri-apps/api/core";
import type { VoraSnapshot } from "@/lib/snapshot/types";
import type {
  BackupCatalog,
  BackupFileInfo,
  ImportedItemOrigin,
  PackageImportSummary,
  PackageSelection,
} from "@/types";
export type {
  BackupCatalog,
  BackupFileInfo,
  BackupSelectableRecord,
  ImportedItemOrigin,
  PackageImportSummary,
  PackageSelection,
} from "@/types";

export type VoraItemType = "board" | "chart" | "library";

export interface VoraItem<TData = unknown> {
  id: string;
  type: VoraItemType;
  name: string;
  data: TData;
  created_at: number;
  updated_at: number;
}

function chartId(snapshot: VoraSnapshot): string {
  return `chart:${snapshot.chart.symbol}:${snapshot.chart.interval}`;
}

export const voraDb = {
  async getCurrentDatabasePath(): Promise<string> {
    return await invoke<string>("get_current_db_path");
  },

  async getSettingsFilePath(): Promise<string> {
    return await invoke<string>("get_settings_file_path");
  },

  async exportDatabaseBackup(backupPath: string): Promise<void> {
    await invoke("export_database_backup", { backupPath });
  },

  async importDatabaseBackup(backupPath: string): Promise<void> {
    await invoke("import_database_backup", { backupPath });
  },

  async exportSettingsJson(filePath: string): Promise<void> {
    await invoke("export_settings_json", { filePath });
  },

  async importSettingsJson(filePath: string): Promise<{ added: number; alreadyPresent: number }> {
    return await invoke<{ added: number; alreadyPresent: number }>("import_settings_json", { filePath });
  },

  async saveSetting(key: string, value: string): Promise<void> {
    await invoke("save_setting", { key, value });
  },

  async getSetting(key: string): Promise<string | null> {
    return await invoke<string | null>("get_setting", { key });
  },

  async getAllSettings(): Promise<Record<string, string>> {
    return await invoke<Record<string, string>>("get_all_settings");
  },

  async deleteSetting(key: string): Promise<void> {
    await invoke("delete_setting", { key });
  },

  async replaceAllSettings(settings: Record<string, string>): Promise<void> {
    await invoke("replace_all_settings", { settings });
  },

  async saveItem<TData>(
    id: string,
    itemType: VoraItemType,
    name: string,
    data: TData,
  ): Promise<void> {
    await invoke("save_vora_item", {
      id,
      itemType,
      name,
      data: JSON.stringify(data),
    });
  },

  async getItems<TData>(itemType: VoraItemType): Promise<VoraItem<TData>[]> {
    return await invoke<VoraItem<TData>[]>("get_vora_items", { itemType });
  },

  async saveSnapshot(snapshot: VoraSnapshot): Promise<void> {
    const id = chartId(snapshot);
    await this.saveItem(id, "chart", `${snapshot.chart.symbol} ${snapshot.chart.interval}`, snapshot.chart);
    await invoke("save_vora_snapshot", {
      id: snapshot.id,
      chartId: id,
      name: snapshot.name,
      data: JSON.stringify(snapshot),
    });
  },

  async getSnapshots(chartIdValue: string): Promise<VoraSnapshot[]> {
    return await invoke<VoraSnapshot[]>("get_vora_snapshots", { chartId: chartIdValue });
  },

  async getAllSnapshots(): Promise<VoraSnapshot[]> {
    return await invoke<VoraSnapshot[]>("get_all_vora_snapshots");
  },

  async deleteSnapshot(id: string): Promise<void> {
    await invoke("delete_vora_snapshot", { id });
  },

  async replaceSnapshots(snapshots: VoraSnapshot[]): Promise<void> {
    await invoke("replace_vora_snapshots", {
      snapshots: snapshots.map((snapshot) => JSON.stringify(snapshot)),
    });
  },

  async getBackupCatalog(): Promise<BackupCatalog> {
    return await invoke<BackupCatalog>("get_backup_catalog");
  },

  async exportSelectivePackage(
    filePath: string,
    packageName: string,
    selection: PackageSelection,
  ): Promise<void> {
    await invoke("export_selective_package", { filePath, packageName, selection });
  },

  async inspectBackupFile(filePath: string): Promise<BackupFileInfo> {
    return await invoke<BackupFileInfo>("inspect_backup_file", { filePath });
  },

  async importSelectivePackage(filePath: string): Promise<PackageImportSummary> {
    return await invoke<PackageImportSummary>("import_selective_package", { filePath });
  },

  async getImportedItemOrigins(): Promise<ImportedItemOrigin[]> {
    return await invoke<ImportedItemOrigin[]>("get_imported_item_origins");
  },
};
