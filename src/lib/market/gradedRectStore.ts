import { create } from "zustand";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

export interface GradedRectConfig {
  height: number;
  width: number;
  defaultBaseline: "top" | "bottom";
}

const CONFIG_KEY = "vora.chart.graded_rectangle.config";
const DEFAULT_CONFIG: GradedRectConfig = {
  height: 1,
  width: 10,
  defaultBaseline: "top",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseConfig(value: string | null): GradedRectConfig {
  if (!value) return DEFAULT_CONFIG;
  try {
    const parsed: unknown = JSON.parse(value);
    if (!isRecord(parsed)) return DEFAULT_CONFIG;
    const candidate = parsed;
    if (
      typeof candidate.height !== "number" ||
      !Number.isFinite(candidate.height) ||
      candidate.height <= 0 ||
      typeof candidate.width !== "number" ||
      !Number.isFinite(candidate.width) ||
      candidate.width < 1 ||
      (candidate.defaultBaseline !== "top" && candidate.defaultBaseline !== "bottom")
    ) return DEFAULT_CONFIG;
    return {
      height: candidate.height,
      width: candidate.width,
      defaultBaseline: candidate.defaultBaseline,
    };
  } catch (error: unknown) {
    console.error("Could not parse graded rectangle settings:", error);
    return DEFAULT_CONFIG;
  }
}

interface GradedRectState {
  gradedRectConfig: GradedRectConfig;
  hydrateGradedRectConfig: () => void;
  setGradedRectConfig: (config: GradedRectConfig) => void;
}

export const useGradedRectStore = create<GradedRectState>((set) => ({
  gradedRectConfig: parseConfig(getSetting(CONFIG_KEY)),
  hydrateGradedRectConfig: () => set({ gradedRectConfig: parseConfig(getSetting(CONFIG_KEY)) }),
  setGradedRectConfig: (gradedRectConfig) => {
    set({ gradedRectConfig });
    setSetting(CONFIG_KEY, JSON.stringify(gradedRectConfig));
  },
}));
