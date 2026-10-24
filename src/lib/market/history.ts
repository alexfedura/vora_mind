// v7.3.2 - in-memory Undo/Redo stack for drawings.
// v7.4.4 - stack now stores DrawingAny (rectangle + 4 line tools).
import type { DrawingAny } from "./lineDrawings";

export type Snapshot = {
  drawings: DrawingAny[];
};

class History {
  private past: Snapshot[] = [];
  private future: Snapshot[] = [];
  private max = 50;

  push(snap: Snapshot): void {
    this.past.push({ drawings: structuredClone(snap.drawings) });
    if (this.past.length > this.max) this.past.shift();
    this.future = [];
  }

  undo(current: Snapshot): Snapshot | null {
    const prev = this.past.pop();
    if (!prev) return null;
    this.future.push({ drawings: structuredClone(current.drawings) });
    return prev;
  }

  redo(current: Snapshot): Snapshot | null {
    const next = this.future.pop();
    if (!next) return null;
    this.past.push({ drawings: structuredClone(current.drawings) });
    return next;
  }

  canUndo(): boolean {
    return this.past.length > 0;
  }

  canRedo(): boolean {
    return this.future.length > 0;
  }

  size(): { past: number; future: number } {
    return { past: this.past.length, future: this.future.length };
  }

  clear(): void {
    this.past = [];
    this.future = [];
  }
}

export const history = new History();
