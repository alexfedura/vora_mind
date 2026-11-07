import type { ParsedCardVersion } from "@/mind/types";

export function parseCardVersion(version: string): ParsedCardVersion | null {
  const numericVersion = version.trim().replace(/^v/, "");
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(numericVersion);
  if (!match) return null;

  const [major, minor, patch] = match.slice(1).map(Number);
  if ([major, minor, patch].some((component) =>
    !Number.isSafeInteger(component) || component > 2_147_483_647
  )) {
    return null;
  }
  return { major, minor, patch };
}

export function nextPatchVersion(version: string): string {
  const parsed = parseCardVersion(version);
  if (!parsed) return "v0.1.1";
  if (parsed.patch === 2_147_483_647) {
    throw new Error("The card version patch number cannot be incremented further.");
  }
  return `v${parsed.major}.${parsed.minor}.${parsed.patch + 1}`;
}
