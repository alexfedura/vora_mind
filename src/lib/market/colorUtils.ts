/** Matches #RRGGBB (7 chars, case-insensitive). */
export function isValidHex(s: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(s);
}

/** Returns the first valid hex from `list`, else #888888. */
export function firstValidHex(
  ...list: (string | null | undefined)[]
): string {
  for (const s of list) {
    if (s && isValidHex(s)) return s;
  }
  return "#888888";
}
