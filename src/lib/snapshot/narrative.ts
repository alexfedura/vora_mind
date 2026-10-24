import type { CompactStep, NarrativeEvent } from "./types";
import type { DrawingAny } from "@/lib/market/lineDrawings";

interface DrawingState {
  signature: string;
  drawing: DrawingAny;
  changedAt: number;
  pending: boolean;
}

function signatureOf(drawing: DrawingAny): string {
  return JSON.stringify(drawing, (key, value: unknown) =>
    key === "selected" || key === "hovered" ? undefined : value,
  );
}

export function deriveCompactSteps(
  events: readonly NarrativeEvent[],
  stabilityMs = 200,
): CompactStep[] {
  const states = new Map<string, DrawingState>();
  const steps: CompactStep[] = [];

  for (const [eventIndex, event] of events.entries()) {
    const presentIds = new Set<string>();
    for (const drawing of event.drawings) {
      presentIds.add(drawing.id);
      const signature = signatureOf(drawing);
      const previous = states.get(drawing.id);

      if (!previous) {
        states.set(drawing.id, {
          signature,
          drawing,
          changedAt: event.timestamp,
          pending: eventIndex > 0,
        });
        continue;
      }

      if (previous.signature !== signature) {
        states.set(drawing.id, {
          signature,
          drawing,
          changedAt: event.timestamp,
          pending: true,
        });
        continue;
      }

      previous.drawing = drawing;
      if (previous.pending && event.timestamp - previous.changedAt >= stabilityMs) {
        steps.push({
          stepId: steps.length + 1,
          committedDrawing: structuredClone(drawing),
          activeScale: { ...event.activeScale },
        });
        previous.pending = false;
      }
    }

    for (const id of states.keys()) {
      if (!presentIds.has(id)) states.delete(id);
    }
  }

  return steps;
}