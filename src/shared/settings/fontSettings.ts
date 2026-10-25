import type { CustomFontSetting } from "@/types";

export const CUSTOM_FONTS_SETTING_KEY = "vora.fonts.custom";

export const FONT_SETTING_TARGETS = [
  { id: "app", label: "Application UI", defaultSize: 16 },
  { id: "cards", label: "Canvas cards", defaultSize: 16 },
  { id: "text", label: "Canvas text", defaultSize: 16 },
  { id: "journal", label: "Journal", defaultSize: 16 },
] as const;

export type FontSettingTarget = (typeof FONT_SETTING_TARGETS)[number]["id"];

export function fontFamilySettingKey(target: FontSettingTarget): string {
  return `vora.fonts.${target}.family`;
}

export function fontSizeSettingKey(target: FontSettingTarget): string {
  return `vora.fonts.${target}.size`;
}

export function getCustomFontFamily(id: string): string {
  return `VoraCustom-${id}`;
}

function isCustomFontSetting(value: unknown): value is CustomFontSetting {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  return "id" in value && typeof value.id === "string" && value.id.length > 0 &&
    "name" in value && typeof value.name === "string" && value.name.length > 0 &&
    "dataUrl" in value && typeof value.dataUrl === "string" && value.dataUrl.startsWith("data:");
}

export function parseCustomFonts(value: string | null): CustomFontSetting[] {
  if (!value) return [];
  try {
    const parsed: unknown = JSON.parse(value);
    if (!Array.isArray(parsed)) throw new Error("The saved custom-font list is not an array.");
    return parsed.filter(isCustomFontSetting);
  } catch (error: unknown) {
    console.error("Could not read saved custom fonts:", error);
    return [];
  }
}
