import { create } from "zustand";
import { voraDb } from "@/services/voraDb";

const LEGACY_APP_STORE_KEY = "vora-app-store";
const LEGACY_CHART_STATE_ID = "chart:local-storage";

interface SettingsState {
  values: Record<string, string>;
  setValues: (values: Record<string, string>) => void;
  setSetting: (key: string, value: string) => void;
  deleteSetting: (key: string) => void;
}

const saveTimers = new Map<string, number>();
let saveQueue: Promise<void> = Promise.resolve();
let lastSave: Promise<void> = Promise.resolve();
let initialization: Promise<Record<string, string>> | null = null;

export const useSettingsStore = create<SettingsState>((set) => ({
  values: {},
  setValues: (values) => set({ values }),
  setSetting: (key, value) => set((state) => ({
    values: { ...state.values, [key]: value },
  })),
  deleteSetting: (key) => set((state) => {
    const values = { ...state.values };
    delete values[key];
    return { values };
  }),
}));

function enqueueSave(operation: () => Promise<void>): void {
  const next = saveQueue.catch(() => undefined).then(operation);
  saveQueue = next;
  lastSave = next;
  void next.catch((error: unknown) => console.error("Could not persist application settings:", error));
}

function scheduleSettingSave(key: string): void {
  const existing = saveTimers.get(key);
  if (existing !== undefined) window.clearTimeout(existing);
  const timer = window.setTimeout(() => {
    saveTimers.delete(key);
    const value = useSettingsStore.getState().values[key];
    if (value !== undefined) enqueueSave(() => voraDb.saveSetting(key, value));
  }, 300);
  saveTimers.set(key, timer);
}

export function getSetting(key: string): string | null {
  return useSettingsStore.getState().values[key] ?? null;
}

export function setSetting(key: string, value: string): void {
  useSettingsStore.getState().setSetting(key, value);
  scheduleSettingSave(key);
}

export function deleteSetting(key: string): void {
  const timer = saveTimers.get(key);
  if (timer !== undefined) window.clearTimeout(timer);
  saveTimers.delete(key);
  useSettingsStore.getState().deleteSetting(key);
  enqueueSave(() => voraDb.deleteSetting(key));
}

export async function flushSettings(): Promise<void> {
  for (const [key, timer] of saveTimers) {
    window.clearTimeout(timer);
    const value = useSettingsStore.getState().values[key];
    if (value !== undefined) enqueueSave(() => voraDb.saveSetting(key, value));
  }
  saveTimers.clear();
  await lastSave;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readLegacySettings(): Record<string, string> {
  const values: Record<string, string> = {};
  try {
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (
        key &&
        key !== "vora.portable.active_database" &&
        key !== "vora.portable.legacy_snapshots_migrated" &&
        key !== "vora.snapshots.meta" &&
        (key.startsWith("vora.") || key.startsWith("vora_worker_") || key === "VORA_DEBUG")
      ) {
        const value = localStorage.getItem(key);
        if (value !== null) values[key] = value;
      }
    }

    const persistedAppStore = localStorage.getItem(LEGACY_APP_STORE_KEY);
    if (persistedAppStore !== null) {
      const parsed: unknown = JSON.parse(persistedAppStore);
      if (isRecord(parsed) && isRecord(parsed.state)) {
        const activeBoardId = parsed.state.activeBoardId;
        const textDirection = parsed.state.textDirection;
        if (typeof activeBoardId === "string") {
          values["vora.canvas.active_board_id"] = activeBoardId;
        }
        if (textDirection === "rtl" || textDirection === "ltr") {
          values["vora.ui.text_direction"] = textDirection;
        }
      }
    }
  } catch (error: unknown) {
    console.error("Could not read legacy browser settings:", error);
  }
  return values;
}

export function initializeSettingsStore(): Promise<Record<string, string>> {
  if (initialization) return initialization;

  initialization = (async () => {
    const storedValues = await voraDb.getAllSettings();
    const migratedValues = readLegacySettings();
    const legacyChartState = await voraDb.getItems<Record<string, string>>("chart");
    const oldChartData = legacyChartState.find((item) => item.id === LEGACY_CHART_STATE_ID)?.data;
    const oldChartValues = isRecord(oldChartData) ? oldChartData : {};
    const validOldChartValues = Object.fromEntries(
      Object.entries(oldChartValues).filter(
        (entry): entry is [string, string] => typeof entry[1] === "string",
      ),
    );

    const values = {
      ...migratedValues,
      ...validOldChartValues,
      ...storedValues,
    };
    if (Object.keys(values).some((key) => !(key in storedValues))) {
      await voraDb.replaceAllSettings(values);
    }

    for (const key of Object.keys(values)) {
      localStorage.removeItem(key);
    }
    localStorage.removeItem(LEGACY_APP_STORE_KEY);
    localStorage.removeItem("vora.portable.active_database");
    useSettingsStore.getState().setValues(values);
    return values;
  })();
  initialization.catch((error: unknown) => {
    initialization = null;
    console.error("Could not initialize database-backed application settings:", error);
  });
  return initialization;
}

export async function refreshSettings(): Promise<void> {
  const values = await voraDb.getAllSettings();
  useSettingsStore.getState().setValues(values);
}

export async function replaceAllSettings(values: Record<string, string>): Promise<void> {
  for (const timer of saveTimers.values()) window.clearTimeout(timer);
  saveTimers.clear();
  await flushSettings();
  await voraDb.replaceAllSettings(values);
  useSettingsStore.getState().setValues(values);
}
