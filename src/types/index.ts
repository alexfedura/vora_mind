export interface BackupSelectableRecord {
  id: string;
  name: string;
  detail: string;
}

export interface BackupCatalog {
  boards: BackupSelectableRecord[];
  sources: BackupSelectableRecord[];
  charts: BackupSelectableRecord[];
  chartSnapshots: BackupSelectableRecord[];
  libraryGroups: BackupSelectableRecord[];
  drawingTemplates: BackupSelectableRecord[];
  drawingAlerts: BackupSelectableRecord[];
  chartDrawings: BackupSelectableRecord[];
}

export interface PackageSelection {
  boards: string[];
  sources: string[];
  charts: string[];
  chartSnapshots: string[];
  libraryGroups: string[];
  drawingTemplates: string[];
  drawingAlerts: string[];
  chartDrawings: string[];
}

export interface BackupFileInfo {
  kind: "selective" | "legacy-full";
  packageName: string | null;
  recordCount: number;
}

export interface PackageImportSummary {
  added: number;
  skipped: number;
  packageName: string;
}

export interface ImportedItemOrigin {
  entityType: string;
  entityId: string;
  displayName: string;
  sourcePackageId: string;
  sourcePackageName: string;
  importedAt: number;
}

export interface TaskFlowItem {
  id: string;
  title: string;
  completed: boolean;
  position: number;
  createdAt: number;
  updatedAt: number;
}

export interface JournalEntry {
  date: string;
  content: string;
  createdAt: number;
  updatedAt: number;
}
