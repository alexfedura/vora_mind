import type { IChartApi, Logical, Time, UTCTimestamp } from "lightweight-charts";

export function drawingTimeToCoordinate(
  timeScale: ReturnType<IChartApi["timeScale"]>,
  timestamp: number,
): number | null {
  const time = timestamp as UTCTimestamp as Time;
  const exactCoordinate = timeScale.timeToCoordinate(time);
  if (exactCoordinate !== null) return exactCoordinate as unknown as number;

  const nearestIndex = timeScale.timeToIndex(time, true);
  if (nearestIndex === null) return null;
  const nearestLogical = nearestIndex as unknown as number;
  const nearestCoordinate = timeScale.logicalToCoordinate(nearestIndex as unknown as Logical);
  if (nearestCoordinate === null) return null;
  const nearestTime = timeScale.coordinateToTime(nearestCoordinate as unknown as number);
  if (typeof nearestTime !== "number") return null;

  let neighborLogical = nearestLogical - 1;
  let neighborCoordinate = timeScale.logicalToCoordinate(neighborLogical as Logical);
  let neighborTime = neighborCoordinate === null
    ? null
    : timeScale.coordinateToTime(neighborCoordinate as unknown as number);
  if (typeof neighborTime !== "number") {
    neighborLogical = nearestLogical + 1;
    neighborCoordinate = timeScale.logicalToCoordinate(neighborLogical as Logical);
    neighborTime = neighborCoordinate === null
      ? null
      : timeScale.coordinateToTime(neighborCoordinate as unknown as number);
  }
  if (typeof neighborTime !== "number") return null;

  const intervalStart = Math.min(nearestTime, neighborTime);
  const intervalEnd = Math.max(nearestTime, neighborTime);
  let logical: number;
  if (timestamp >= intervalStart && timestamp <= intervalEnd) {
    logical = nearestLogical
      + ((timestamp - nearestTime) * (nearestLogical - neighborLogical))
        / (nearestTime - neighborTime);
  } else {
    const intervals: number[] = [];
    for (const direction of [-1, 1]) {
      let previousTime = nearestTime;
      for (let step = 1; step <= 8; step++) {
        const sampleIndex = nearestLogical + direction * step;
        const sampleCoordinate = timeScale.logicalToCoordinate(sampleIndex as Logical);
        if (sampleCoordinate === null) break;
        const sampleTime = timeScale.coordinateToTime(sampleCoordinate as unknown as number);
        if (typeof sampleTime !== "number") break;
        const seconds = Math.abs(sampleTime - previousTime);
        if (seconds > 0 && Number.isFinite(seconds)) intervals.push(seconds);
        previousTime = sampleTime;
      }
    }
    intervals.sort((a, b) => a - b);
    const medianInterval = intervals[Math.floor(intervals.length / 2)];
    if (!Number.isFinite(medianInterval) || medianInterval <= 0) return null;
    logical = nearestLogical + (timestamp - nearestTime) / medianInterval;
  }
  const lowerLogical = Math.floor(logical);
  const upperLogical = Math.ceil(logical);
  const lowerCoordinate = timeScale.logicalToCoordinate(lowerLogical as Logical);
  if (lowerCoordinate === null) return null;
  if (lowerLogical === upperLogical) return lowerCoordinate as unknown as number;
  const upperCoordinate = timeScale.logicalToCoordinate(upperLogical as Logical);
  if (upperCoordinate === null) return null;
  const fraction = logical - lowerLogical;
  return (lowerCoordinate + (upperCoordinate - lowerCoordinate) * fraction) as unknown as number;
}