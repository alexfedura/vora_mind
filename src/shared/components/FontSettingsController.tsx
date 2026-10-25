import React, { useEffect, useMemo } from "react";
import { useSettingsStore } from "@/shared/store/useSettingsStore";
import {
  CUSTOM_FONTS_SETTING_KEY,
  fontFamilySettingKey,
  fontSizeSettingKey,
  getCustomFontFamily,
  parseCustomFonts,
  type FontSettingTarget,
} from "@/shared/settings/fontSettings";

const DEFAULT_UI_FONT = "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

function getFontSize(value: string | undefined, defaultSize: number): number {
  const size = Number(value);
  return Number.isFinite(size) && size > 0 ? size : defaultSize;
}

function selectedFamily(
  target: FontSettingTarget,
  settings: Record<string, string>,
  fontsById: Map<string, string>,
): string | null {
  const selectedId = settings[fontFamilySettingKey(target)];
  const family = selectedId ? fontsById.get(selectedId) : undefined;
  return family ? `"${family}"` : null;
}

export const FontSettingsController: React.FC = () => {
  const settings = useSettingsStore((state) => state.values);
  const customFonts = useMemo(
    () => parseCustomFonts(settings[CUSTOM_FONTS_SETTING_KEY] ?? null),
    [settings[CUSTOM_FONTS_SETTING_KEY]],
  );

  useEffect(() => {
    const root = document.documentElement;
    const fontsById = new Map(customFonts.map((font) => [font.id, getCustomFontFamily(font.id)]));
    const faces: FontFace[] = [];
    let cancelled = false;

    const appFamily = selectedFamily("app", settings, fontsById) ?? DEFAULT_UI_FONT;
    root.style.setProperty("--vora-ui-font-family", appFamily);
    root.style.setProperty(
      "--vora-ui-font-size",
      `${getFontSize(settings[fontSizeSettingKey("app")], 16)}px`,
    );

    const cardsFamily = selectedFamily("cards", settings, fontsById) ?? "var(--vora-ui-font-family)";
    const cardsSize = getFontSize(settings[fontSizeSettingKey("cards")], 16);
    root.style.setProperty("--vora-card-font-family", cardsFamily);
    root.style.setProperty("--vora-card-font-size", `${cardsSize}px`);

    const textFamily = selectedFamily("text", settings, fontsById) ?? "var(--vora-ui-font-family)";
    root.style.setProperty("--vora-text-font-family", textFamily);
    root.style.setProperty(
      "--vora-text-font-scale",
      String(getFontSize(settings[fontSizeSettingKey("text")], 16) / 16),
    );

    const journalFamily = selectedFamily("journal", settings, fontsById) ?? "var(--vora-ui-font-family)";
    root.style.setProperty("--vora-journal-font-family", journalFamily);
    root.style.setProperty(
      "--vora-journal-font-size",
      `${getFontSize(settings[fontSizeSettingKey("journal")], 16)}px`,
    );

    const loadFonts = async (): Promise<void> => {
      for (const font of customFonts) {
        try {
          const face = await new FontFace(
            getCustomFontFamily(font.id),
            `url("${font.dataUrl}")`,
          ).load();
          if (cancelled) return;
          document.fonts.add(face);
          faces.push(face);
        } catch (error: unknown) {
          console.error(`Could not load saved font "${font.name}":`, error);
        }
      }
    };
    void loadFonts();

    return () => {
      cancelled = true;
      for (const face of faces) document.fonts.delete(face);
    };
  }, [
    customFonts,
    settings[fontSizeSettingKey("app")],
    settings[fontSizeSettingKey("cards")],
    settings[fontSizeSettingKey("text")],
    settings[fontSizeSettingKey("journal")],
    settings[fontFamilySettingKey("app")],
    settings[fontFamilySettingKey("cards")],
    settings[fontFamilySettingKey("text")],
    settings[fontFamilySettingKey("journal")],
  ]);

  return null;
};
