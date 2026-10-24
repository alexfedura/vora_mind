import { openDB } from "idb";
import type { VoraSnapshot, SnapshotMetadata } from "./types";
import { voraDb } from "@/services/voraDb";

const LEGACY_DB_NAME = "vora_snapshots";
const LEGACY_STORE = "snapshots";
const LEGACY_MIGRATION_KEY = "vora.portable.legacy_snapshots_migrated";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isVoraSnapshot(value: unknown): value is VoraSnapshot {
  if (!isRecord(value)) return false;
  const snapshot = value;
  return snapshot.format === "vora-snapshot" &&
    typeof snapshot.schemaVersion === "number" &&
    typeof snapshot.id === "string" &&
    typeof snapshot.name === "string" &&
    typeof snapshot.createdAt === "number" &&
    typeof snapshot.updatedAt === "number" &&
    typeof snapshot.checksum === "string" &&
    Array.isArray(snapshot.tags) &&
    Array.isArray(snapshot.drawings) &&
    typeof snapshot.chart === "object" && snapshot.chart !== null &&
    typeof snapshot.viewport === "object" && snapshot.viewport !== null &&
    typeof snapshot.policy === "object" && snapshot.policy !== null &&
    typeof snapshot.theme === "object" && snapshot.theme !== null;
}

export async function migrateLegacySnapshots(): Promise<void> {
  if (localStorage.getItem(LEGACY_MIGRATION_KEY) === "1") return;

  const legacyDb = await openDB(LEGACY_DB_NAME, 1, {
    upgrade(db) {
      if (!db.objectStoreNames.contains(LEGACY_STORE)) {
        const store = db.createObjectStore(LEGACY_STORE, { keyPath: "id" });
        store.createIndex("createdAt", "createdAt");
        store.createIndex("name", "name");
        store.createIndex("symbol", "chart.symbol");
        store.createIndex("updatedAt", "updatedAt");
      }
    },
  });
  try {
    const legacyRows = await legacyDb.getAll(LEGACY_STORE);
    const legacySnapshots = legacyRows.filter(isVoraSnapshot);
    if (legacySnapshots.length > 0) {
      const current = await voraDb.getAllSnapshots();
      const existingIds = new Set(current.map((snapshot) => snapshot.id));
      await replaceSnapshots([
        ...current,
        ...legacySnapshots.filter((snapshot) => !existingIds.has(snapshot.id)),
      ]);
    }
    localStorage.setItem(LEGACY_MIGRATION_KEY, "1");
  } finally {
    legacyDb.close();
  }
}

export async function saveSnapshot(snapshot: VoraSnapshot): Promise<void> {
  await voraDb.saveSnapshot(snapshot);
  await updateMetaList();
}

export async function loadSnapshot(id: string): Promise<VoraSnapshot | null> {
  const snapshots = await voraDb.getAllSnapshots();
  return snapshots.find((snapshot) => snapshot.id === id) ?? null;
}

export async function deleteSnapshot(id: string): Promise<void> {
  await voraDb.deleteSnapshot(id);
  await updateMetaList();
}

export async function listSnapshots(): Promise<VoraSnapshot[]> {
  const all = await voraDb.getAllSnapshots();
  return all.sort((a, b) => b.createdAt - a.createdAt);
}

export async function replaceSnapshots(snapshots: VoraSnapshot[]): Promise<void> {
  await voraDb.replaceSnapshots(snapshots);
  await updateMetaList();
}

export async function listMetadata(): Promise<SnapshotMetadata[]> {
  const all = await listSnapshots();
  return all.map((s) => ({
    id: s.id,
    name: s.name,
    symbol: s.chart.symbol,
    interval: s.chart.interval,
    createdAt: s.createdAt,
    updatedAt: s.updatedAt,
    drawingCount: s.drawings.length,
    tags: s.tags,
  }));
}

async function updateMetaList(): Promise<void> {
  const meta = await listMetadata();
  try {
    localStorage.setItem("vora.snapshots.meta", JSON.stringify(meta));
  } catch (e) {
    console.warn("Failed to mirror snapshot metadata to localStorage", e);
  }
}

export function readMetaFromLocalStorage(): SnapshotMetadata[] {
  try {
    const raw = localStorage.getItem("vora.snapshots.meta");
    if (!raw) return [];
    return JSON.parse(raw) as SnapshotMetadata[];
  } catch {
    return [];
  }
}
