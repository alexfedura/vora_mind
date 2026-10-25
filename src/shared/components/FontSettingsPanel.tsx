import React, { useMemo, useState } from "react";
import { Trash2, Upload } from "lucide-react";
import type { CustomFontSetting } from "@/types";
import { deleteSetting, getSetting, setSetting, useSettingsStore } from "@/shared/store/useSettingsStore";
import {
  CUSTOM_FONTS_SETTING_KEY,
  FONT_SETTING_TARGETS,
  fontFamilySettingKey,
  fontSizeSettingKey,
  getCustomFontFamily,
  parseCustomFonts,
} from "@/shared/settings/fontSettings";

function readFontFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error("Could not read the selected font file."));
    reader.onload = () => {
      if (typeof reader.result !== "string") {
        reject(new Error("The selected font file could not be read."));
        return;
      }
      resolve(reader.result);
    };
    reader.readAsDataURL(file);
  });
}

export const FontSettingsPanel: React.FC = () => {
  const settings = useSettingsStore((state) => state.values);
  const [fontError, setFontError] = useState<string | null>(null);
  const customFonts = useMemo(
    () => parseCustomFonts(settings[CUSTOM_FONTS_SETTING_KEY] ?? null),
    [settings],
  );

  const addFont = async (file: File): Promise<void> => {
    setFontError(null);
    const extension = /\.((ttf)|(otf)|(woff2?))$/i.exec(file.name)?.[1]?.toLowerCase();
    if (!extension) {
      setFontError("Choose a TTF, OTF, WOFF, or WOFF2 font file.");
      return;
    }

    try {
      const font: CustomFontSetting = {
        id: crypto.randomUUID(),
        name: file.name.replace(/\.(ttf|otf|woff2?)$/i, ""),
        dataUrl: await readFontFile(file),
      };
      await new FontFace(getCustomFontFamily(font.id), `url("${font.dataUrl}")`).load();
      setSetting(CUSTOM_FONTS_SETTING_KEY, JSON.stringify([...customFonts, font]));
    } catch (error: unknown) {
      console.error("Could not import the selected font:", error);
      setFontError(error instanceof Error ? error.message : String(error));
    }
  };

  const removeFont = (fontId: string): void => {
    const remainingFonts = customFonts.filter((font) => font.id !== fontId);
    setSetting(CUSTOM_FONTS_SETTING_KEY, JSON.stringify(remainingFonts));
    for (const { id } of FONT_SETTING_TARGETS) {
      const key = fontFamilySettingKey(id);
      if (getSetting(key) === fontId) deleteSetting(key);
    }
  };

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-[#242835] bg-[#1a1d25] p-4" aria-labelledby="font-settings-heading">
      <div>
        <h3 id="font-settings-heading" className="text-xs font-semibold text-gray-100">Fonts & text size</h3>
        <p className="mt-1 text-[11px] leading-relaxed text-gray-400">
          Choose separate fonts and sizes for each area. Imported fonts stay on this device.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {FONT_SETTING_TARGETS.map(({ id, label, defaultSize }) => {
          const selectedFontId = settings[fontFamilySettingKey(id)] ?? "";
          const storedSize = Number(settings[fontSizeSettingKey(id)]);
          const fontSize = Number.isFinite(storedSize) && storedSize > 0 ? storedSize : defaultSize;
          return (
            <div key={id} className="flex min-w-0 flex-col gap-2 rounded-lg border border-[#292e3b] bg-[#14161c] p-3">
              <label htmlFor={`font-family-${id}`} className="text-[11px] font-medium text-gray-300">{label}</label>
              <select
                id={`font-family-${id}`}
                value={selectedFontId}
                onChange={(event) => {
                  if (event.target.value) setSetting(fontFamilySettingKey(id), event.target.value);
                  else deleteSetting(fontFamilySettingKey(id));
                }}
                className="w-full rounded-md border border-[#303543] bg-[#1b1e27] px-2 py-1.5 text-xs text-gray-200 outline-none focus:border-purple-500"
              >
                <option value="">Default font</option>
                {customFonts.map((font) => (
                  <option key={font.id} value={font.id}>{font.name}</option>
                ))}
              </select>
              <label htmlFor={`font-size-${id}`} className="flex items-center justify-between gap-2 text-[11px] text-gray-400">
                Size
                <span className="flex items-center gap-1.5">
                  <input
                    id={`font-size-${id}`}
                    type="number"
                    min="1"
                    step="1"
                    value={fontSize}
                    onChange={(event) => {
                      const nextSize = Number(event.target.value);
                      if (Number.isFinite(nextSize) && nextSize > 0) {
                        setSetting(fontSizeSettingKey(id), String(nextSize));
                      }
                    }}
                    className="w-20 rounded-md border border-[#303543] bg-[#1b1e27] px-2 py-1 text-right text-xs text-gray-200 outline-none focus:border-purple-500"
                  />
                  px
                </span>
              </label>
            </div>
          );
        })}
      </div>

      <div className="flex flex-col gap-2 border-t border-[#292e3b] pt-3">
        <label className="inline-flex w-fit cursor-pointer items-center gap-2 rounded-lg border border-[#343a49] bg-[#20232c] px-3 py-2 text-xs font-medium text-gray-200 transition hover:border-purple-500/50 hover:text-white">
          <Upload size={14} />
          Import font file
          <input
            type="file"
            accept=".ttf,.otf,.woff,.woff2"
            className="sr-only"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = "";
              if (file) void addFont(file);
            }}
          />
        </label>
        {fontError && <p role="alert" className="text-xs text-rose-300">{fontError}</p>}
        {customFonts.map((font) => (
          <div key={font.id} className="flex items-center justify-between gap-2 text-xs text-gray-300">
            <span className="min-w-0 truncate">{font.name}</span>
            <button
              type="button"
              onClick={() => removeFont(font.id)}
              aria-label={`Remove ${font.name}`}
              className="shrink-0 rounded-md p-1 text-gray-500 transition hover:bg-rose-500/10 hover:text-rose-300"
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
      </div>
    </section>
  );
};
