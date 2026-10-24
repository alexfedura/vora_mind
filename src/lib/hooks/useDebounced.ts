import { useEffect, useState } from 'react';

/**
 * Returns a debounced copy of `value` that updates only after
 * `delayMs` of no changes. Useful for expensive effects (e.g.
 * backend fetch) triggered by fast-changing UI (e.g. slider drag).
 */
export function useDebounced<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}
