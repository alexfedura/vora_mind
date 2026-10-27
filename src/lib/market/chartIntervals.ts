export type IntervalGroupName = "Ticks" | "Seconds" | "Minutes" | "Hours" | "Days" | "Weeks" | "Months" | "Ranges" | "VORA M-scale";

export interface ChartIntervalOption {
  key: string;
  label: string;
  group: IntervalGroupName;
  enabled: boolean;
  disabledReason?: string;
}

const INTERVAL_GROUPS: ReadonlyArray<{ group: IntervalGroupName; values: ReadonlyArray<{ value: number; unit: string; labelUnit: string }>; enabled: boolean }> = [
  { group: "Ticks", values: [1, 10, 100, 1000].map((value) => ({ value, unit: "t", labelUnit: "tick" })), enabled: false },
  { group: "Seconds", values: [1, 5, 10, 15, 30].map((value) => ({ value, unit: "s", labelUnit: "second" })), enabled: false },
  { group: "Minutes", values: [1, 2, 3, 5, 10, 15, 30, 45].map((value) => ({ value, unit: "m", labelUnit: "minute" })), enabled: true },
  { group: "Hours", values: [1, 2, 3, 4, 6, 8, 12].map((value) => ({ value, unit: "h", labelUnit: "hour" })), enabled: true },
  { group: "Days", values: [1, 2, 3, 5, 10].map((value) => ({ value, unit: "d", labelUnit: "day" })), enabled: true },
  { group: "Weeks", values: [1, 2, 3, 4].map((value) => ({ value, unit: "w", labelUnit: "week" })), enabled: true },
  { group: "Months", values: [1, 3, 6, 12].map((value) => ({ value, unit: "mo", labelUnit: "month" })), enabled: true },
  { group: "Ranges", values: [1, 10, 100, 1000].map((value) => ({ value, unit: "r", labelUnit: "range" })), enabled: false },
];

const M_SCALE_SECONDS = [1, 5, 25, 125, 625, 3125, 15625, 78125, 390625, 1953125, 9765625];
const M_SCALE_LABELS = ["1 sec", "5 sec", "25 sec", "2.08 min", "10.42 min", "52.08 min", "4.34 hr", "21.7 hr", "4.52 days", "22.6 days", "113 days"];

export const CHART_INTERVAL_OPTIONS: ChartIntervalOption[] = [
  ...INTERVAL_GROUPS.flatMap(({ group, values, enabled }) => values.map(({ value, unit, labelUnit }) => ({
    key: `${value}${unit}`,
    label: `${value} ${labelUnit}${value === 1 ? "" : "s"}`,
    group,
    enabled,
    ...(!enabled ? { disabledReason: group === "Ticks" ? "Tick-count bars are not available in the local time-bucket feed" : group === "Seconds" ? "The local candle command currently supports whole-minute intervals only" : "Range bars need a verified instrument tick size" } : {}),
  }))),
  ...M_SCALE_SECONDS.map((_seconds, step) => ({ key: `M${step}`, label: `M${step} · ${M_SCALE_LABELS[step]}`, group: "VORA M-scale" as const, enabled: false, disabledReason: "VORA M-scale buckets are not whole-minute intervals" })),
];

const LABEL_BY_KEY = new Map(CHART_INTERVAL_OPTIONS.map((option) => [option.key, option.label]));

export function intervalLabel(key: string): string {
  return LABEL_BY_KEY.get(key) ?? key;
}

export function intervalGroupName(key: string): IntervalGroupName | null {
  return CHART_INTERVAL_OPTIONS.find((option) => option.key === key)?.group ?? null;
}

export function isIntervalEnabled(key: string): boolean {
  const option = CHART_INTERVAL_OPTIONS.find((candidate) => candidate.key === key);
  return (option ? option.enabled : parseCustomInterval(key) !== null)
    && chartIntervalMinutes(key) !== null;
}

export function parseCustomInterval(key: string): { amount: number; unit: "s" | "m" | "h" | "d" | "w" | "mo" } | null {
  const match = /^(\d+)(s|m|h|d|w|mo)$/.exec(key);
  if (!match) return null;
  const amount = Number(match[1]);
  const unit = match[2];
  if (unit !== "s" && unit !== "m" && unit !== "h" && unit !== "d" && unit !== "w" && unit !== "mo") return null;
  if (!Number.isSafeInteger(amount) || amount < 1) return null;
  const secondsByUnit: Record<typeof unit, number> = { s: 1, m: 60, h: 3600, d: 86400, w: 604800, mo: 30 * 86400 };
  if (unit === "mo" ? amount > 12 : amount * secondsByUnit[unit] > 366 * 86400) return null;
  return { amount, unit };
}

export function isChartIntervalKey(key: string): boolean {
  return isIntervalEnabled(key);
}

export function chartIntervalMinutes(key: string): number | null {
  const option = CHART_INTERVAL_OPTIONS.find((candidate) => candidate.key === key);
  if (!option && parseCustomInterval(key) === null) return null;
  const seconds = intervalWindowSeconds(key);
  return seconds >= 60 && seconds % 60 === 0 ? seconds / 60 : null;
}

export function intervalWindowSeconds(key: string): number {
  const mScaleMatch = /^M(\d+)$/.exec(key);
  if (mScaleMatch) return M_SCALE_SECONDS[Number(mScaleMatch[1])] ?? 1;
  const parsed = parseCustomInterval(key);
  if (!parsed) return 60;
  const multiplier: Record<typeof parsed.unit, number> = { s: 1, m: 60, h: 3600, d: 86400, w: 604800, mo: 30 * 86400 };
  return parsed.amount * multiplier[parsed.unit];
}

export function nearestMScaleStep(key: string): number {
  const seconds = intervalWindowSeconds(key);
  let nearest = 0;
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (let step = 0; step < M_SCALE_SECONDS.length; step += 1) {
    const value = M_SCALE_SECONDS[step] ?? 1;
    const distance = Math.abs(Math.log(seconds) - Math.log(value));
    if (distance < nearestDistance) {
      nearest = step;
      nearestDistance = distance;
    }
  }
  return nearest;
}

export function normalizeCustomInterval(amountText: string, unit: string): string | null {
  const amount = Number(amountText);
  if (unit !== "s" && unit !== "m" && unit !== "h" && unit !== "d" && unit !== "w" && unit !== "mo") return null;
  const secondsByUnit: Record<typeof unit, number> = { s: 1, m: 60, h: 3600, d: 86400, w: 604800, mo: 30 * 86400 };
  if (!Number.isSafeInteger(amount) || amount < 1 || (unit === "mo" ? amount > 12 : amount * secondsByUnit[unit] > 366 * 86400)) return null;
  return `${amount}${unit}`;
}