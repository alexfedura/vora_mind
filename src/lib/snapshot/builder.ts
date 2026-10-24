import type {
  VoraSnapshot,
  SnapshotViewport,
  SnapshotChartState,
  SnapshotDrawing,
  SnapshotPolicy,
  SnapshotTheme,
  NarrativeMode,
  NarrativeEvent,
  CompactStep,
} from "./types";
import { SNAPSHOT_FORMAT, SNAPSHOT_SCHEMA_VERSION } from "./types";
import { computeChecksum } from "./serialize";

export interface BuildSnapshotInput {
  name: string;
  notes?: string;
  tags?: string[];

  chart: SnapshotChartState;
  viewport: SnapshotViewport;
  drawings: SnapshotDrawing[];
  policy: SnapshotPolicy;
  theme: SnapshotTheme;
  narrativeMode?: NarrativeMode;
  narrativeLog?: NarrativeEvent[] | CompactStep[];
}

export async function buildSnapshot(
  input: BuildSnapshotInput
): Promise<VoraSnapshot> {
  const now = Date.now();
  const base: VoraSnapshot = {
    format: SNAPSHOT_FORMAT,
    schemaVersion: SNAPSHOT_SCHEMA_VERSION,
    id: crypto.randomUUID(),
    name: input.name,
    notes: input.notes ?? "",
    tags: input.tags ?? [],
    createdAt: now,
    updatedAt: now,

    chart: input.chart,
    viewport: input.viewport,
    drawings: input.drawings,
    policy: input.policy,
    theme: input.theme,
    ...(input.narrativeMode && input.narrativeLog
      ? { narrativeMode: input.narrativeMode, narrativeLog: input.narrativeLog }
      : {}),

    checksum: "",
  };

  base.checksum = await computeChecksum(base);
  return base;
}
