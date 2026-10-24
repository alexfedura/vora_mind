import { useEffect, useState } from "react";

// v0.83.2.1 — Global interaction lock so open modals/overlays can
// disable chart interaction (pointer + keyboard) without conflicts.
// Module-level counter: multiple overlays may hold the lock.
let lockCount = 0;
const listeners = new Set<(locked: boolean) => void>();

function emit(): void {
  const locked = lockCount > 0;
  listeners.forEach((cb) => cb(locked));
}

export function acquireInteractionLock(): () => void {
  lockCount++;
  emit();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    lockCount = Math.max(0, lockCount - 1);
    emit();
  };
}

export function useInteractionLocked(): boolean {
  const [locked, setLocked] = useState(lockCount > 0);
  useEffect(() => {
    const cb = (l: boolean): void => setLocked(l);
    listeners.add(cb);
    // Sync in case lock state changed between render and effect.
    cb(lockCount > 0);
    return () => {
      listeners.delete(cb);
    };
  }, []);
  return locked;
}
