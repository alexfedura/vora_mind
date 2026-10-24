import { useEffect, useRef, useState, type ReactElement, type MouseEvent, type PointerEvent } from "react";
import { createPortal } from "react-dom";
import { Plus, Trash } from "lucide-react";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

export const CHART_DRAWING_COLORS = [
  "#ffffff", "#dbdbdb", "#b8b8b8", "#9c9c9c", "#808080", "#636363", "#4a4a4a", "#2e2e2e", "#0f0f0f", "#000000",
  "#f23645", "#ff9800", "#ffeb3b", "#4caf50", "#089981", "#00bcd4", "#2962ff", "#673ab7", "#9c27b0", "#e91e63",
  "#fccbcd", "#ffe0b2", "#fff9c4", "#c8e6c9", "#ace5dc", "#b2ebf2", "#bbd9fb", "#d1c4e9", "#e1bee7", "#f8bbd0",
  "#faa1a4", "#ffcc80", "#fff59d", "#a5d6a7", "#70ccbd", "#80deea", "#90bff9", "#b39ddb", "#ce93d8", "#f48fb1",
  "#f77c80", "#ffb74d", "#fff176", "#81c784", "#42bda8", "#4dd0e1", "#5b9cf6", "#9575cd", "#ba68c8", "#f06292",
  "#f7525f", "#ffa726", "#ffee58", "#66bb6a", "#22ab94", "#26c6da", "#3179f5", "#7e57c2", "#ab47bc", "#ec407a",
  "#b22833", "#f57c00", "#fbc02d", "#388e3c", "#056656", "#0097a7", "#1848cc", "#512da8", "#7b1fa2", "#c2185b",
  "#801922", "#e65100", "#f57f17", "#1b5e20", "#00332a", "#006064", "#0c3299", "#311b92", "#4a148c", "#880e4f",
] as const;

const CUSTOM_COLORS_KEY = "vora.drawing_custom_colors";
const REMOVED_COLORS_KEY = "vora.drawing_removed_colors";

function isValidHexColor(color: string): boolean {
  return /^#[\da-fA-F]{6}$/.test(color);
}

function isChartDrawingColor(color: string): boolean {
  return CHART_DRAWING_COLORS.some((candidate) => candidate === color);
}

interface HsvColor {
  hue: number;
  saturation: number;
  value: number;
}

interface DrawingColorPickerProps {
  color: string;
  opacity: number;
  label: string;
  onColor: (color: string) => void;
  onOpacity: (opacity: number) => void;
  onKeepOpen: () => void;
}

function loadColors(key: string): string[] {
  try {
    const stored: unknown = JSON.parse(getSetting(key) ?? "[]");
    return Array.isArray(stored)
      ? stored.filter((color): color is string => typeof color === "string" && isValidHexColor(color)).map((color) => color.toLowerCase())
      : [];
  } catch (error) {
    console.warn(`Could not load drawing colors from ${key}`, error);
    return [];
  }
}

function hexToHsv(hex: string): HsvColor {
  const value = parseInt(hex.slice(1), 16);
  const red = ((value >> 16) & 255) / 255;
  const green = ((value >> 8) & 255) / 255;
  const blue = (value & 255) / 255;
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const delta = maximum - minimum;
  let hue = 0;
  if (delta > 0) {
    if (maximum === red) hue = 60 * (((green - blue) / delta) % 6);
    else if (maximum === green) hue = 60 * ((blue - red) / delta + 2);
    else hue = 60 * ((red - green) / delta + 4);
  }
  return { hue: (hue + 360) % 360, saturation: maximum === 0 ? 0 : delta / maximum, value: maximum };
}

function hsvToHex({ hue, saturation, value }: HsvColor): string {
  const chroma = value * saturation;
  const secondary = chroma * (1 - Math.abs((hue / 60) % 2 - 1));
  const offset = value - chroma;
  const sector = Math.floor(hue / 60) % 6;
  const channels = sector === 0 ? [chroma, secondary, 0]
    : sector === 1 ? [secondary, chroma, 0]
      : sector === 2 ? [0, chroma, secondary]
        : sector === 3 ? [0, secondary, chroma]
          : sector === 4 ? [secondary, 0, chroma]
            : [chroma, 0, secondary];
  return `#${channels.map((channel) => Math.round((channel + offset) * 255).toString(16).padStart(2, "0")).join("")}`;
}

function persistColors(key: string, colors: string[]): void {
  try {
    setSetting(key, JSON.stringify(colors));
  } catch (error) {
    console.warn(`Could not save drawing colors to ${key}`, error);
  }
}

export function DrawingColorPicker(p: DrawingColorPickerProps): ReactElement {
  const opacityPercent = Math.round(p.opacity * 100);
  const [customOpen, setCustomOpen] = useState(false);
  const [customColors, setCustomColors] = useState<string[]>(() => loadColors(CUSTOM_COLORS_KEY));
  const [removedColors, setRemovedColors] = useState<string[]>(() => loadColors(REMOVED_COLORS_KEY));
  const [removeMenu, setRemoveMenu] = useState<{ color: string; x: number; y: number } | null>(null);
  const [hexValue, setHexValue] = useState(p.color.toLowerCase());
  const [hsv, setHsv] = useState<HsvColor>(() => hexToHsv(p.color));
  const saturationRef = useRef<HTMLDivElement | null>(null);
  const hueRef = useRef<HTMLDivElement | null>(null);
  const removeMenuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!removeMenu) return;
    const closeMenu = (event: globalThis.PointerEvent): void => {
      if (event.target instanceof Node && !removeMenuRef.current?.contains(event.target)) setRemoveMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setRemoveMenu(null);
    };
    document.addEventListener("pointerdown", closeMenu);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeMenu);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [removeMenu]);

  const setOpacity = (value: number): void => p.onOpacity(Math.max(0, Math.min(100, value)) / 100);
  const openRemoveMenu = (event: MouseEvent<HTMLButtonElement>, color: string): void => {
    event.preventDefault();
    event.stopPropagation();
    const swatch = event.currentTarget.getBoundingClientRect();
    const maxLeft = Math.max(8, window.innerWidth - 149);
    const maxTop = Math.max(8, window.innerHeight - 52);
    setRemoveMenu({
      color,
      x: Math.max(8, Math.min(swatch.left + 3, maxLeft)),
      y: Math.max(8, Math.min(swatch.bottom + 16, maxTop)),
    });
  };
  const swatches = (colors: readonly string[]): ReactElement[] => colors
    .filter((color) => !removedColors.includes(color))
    .map((color) => (
      <button
        key={color}
        type="button"
        className="drawing-toolbar__swatch"
        style={{ color }}
        aria-label={`Set ${p.label.toLowerCase()} ${color}`}
        aria-pressed={p.color.toLowerCase() === color}
        title={`${color} · right-click to remove`}
        onClick={() => p.onColor(color)}
        onContextMenu={(event) => openRemoveMenu(event, color)}
      />
    ));
  const setCustomColorsPersisted = (colors: string[]): void => {
    setCustomColors(colors);
    persistColors(CUSTOM_COLORS_KEY, colors);
  };
  const setRemovedColorsPersisted = (colors: string[]): void => {
    setRemovedColors(colors);
    persistColors(REMOVED_COLORS_KEY, colors);
  };
  const setHsvColor = (next: HsvColor): void => {
    setHsv(next);
    setHexValue(hsvToHex(next));
  };
  const updateSaturation = (event: PointerEvent<HTMLDivElement>): void => {
    const bounds = saturationRef.current?.getBoundingClientRect();
    if (!bounds) return;
    setHsvColor({
      ...hsv,
      saturation: Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width)),
      value: 1 - Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height)),
    });
  };
  const updateHue = (event: PointerEvent<HTMLDivElement>): void => {
    const bounds = hueRef.current?.getBoundingClientRect();
    if (!bounds) return;
    setHsvColor({ ...hsv, hue: Math.max(0, Math.min(359, ((event.clientY - bounds.top) / bounds.height) * 360)) });
  };
  const openCustomPicker = (): void => {
    setHexValue(p.color.toLowerCase());
    setHsv(hexToHsv(p.color));
    setCustomOpen(true);
  };
  const applyCustomColor = (): void => {
    if (!isValidHexColor(hexValue)) return;
    const color = hexValue.toLowerCase();
    if (!isChartDrawingColor(color) && !customColors.includes(color)) {
      setCustomColorsPersisted([...customColors, color]);
    }
    p.onColor(color);
    setCustomOpen(false);
  };
  const removeColor = (color: string): void => {
    setCustomColorsPersisted(customColors.filter((customColor) => customColor !== color));
    if (isChartDrawingColor(color) && !removedColors.includes(color)) {
      setRemovedColorsPersisted([...removedColors, color]);
    }
    p.onKeepOpen();
    setRemoveMenu(null);
  };

  if (customOpen) return (
    <div className="drawing-toolbar__custom-picker">
      <div className="drawing-toolbar__custom-picker-form">
        <span className="drawing-toolbar__custom-preview" style={{ backgroundColor: isValidHexColor(hexValue) ? hexValue : p.color }} />
        <label className="drawing-toolbar__hex-input">
          <span>#</span>
          <input
            value={hexValue.replace(/^#/, "")}
            maxLength={6}
            aria-label="Hex color"
            onChange={(event) => {
              const next = event.currentTarget.value.replace(/^#/, "");
              if (!/^[\da-fA-F]{0,6}$/.test(next)) return;
              setHexValue(`#${next}`);
              if (isValidHexColor(`#${next}`)) setHsv(hexToHsv(`#${next}`));
            }}
          />
        </label>
        <button type="button" className="drawing-toolbar__custom-add" disabled={!isValidHexColor(hexValue)} onClick={applyCustomColor}>Add</button>
      </div>
      <div className="drawing-toolbar__hsv-controls">
        <div
          ref={saturationRef}
          className="drawing-toolbar__saturation"
          role="slider"
          tabIndex={0}
          aria-label="Color saturation and brightness"
          aria-valuetext={`${Math.round(hsv.saturation * 100)}% saturation, ${Math.round(hsv.value * 100)}% brightness`}
          style={{ backgroundImage: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${hsv.hue} 100% 50%))` }}
          onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); updateSaturation(event); }}
          onPointerMove={(event) => { if (event.buttons > 0) updateSaturation(event); }}
          onKeyDown={(event) => {
            const step = event.shiftKey ? 0.1 : 0.02;
            if (event.key === "ArrowLeft") { event.preventDefault(); setHsvColor({ ...hsv, saturation: Math.max(0, hsv.saturation - step) }); }
            if (event.key === "ArrowRight") { event.preventDefault(); setHsvColor({ ...hsv, saturation: Math.min(1, hsv.saturation + step) }); }
            if (event.key === "ArrowUp") { event.preventDefault(); setHsvColor({ ...hsv, value: Math.min(1, hsv.value + step) }); }
            if (event.key === "ArrowDown") { event.preventDefault(); setHsvColor({ ...hsv, value: Math.max(0, hsv.value - step) }); }
          }}
        >
          <span className="drawing-toolbar__saturation-pointer" style={{ left: `${hsv.saturation * 100}%`, top: `${(1 - hsv.value) * 100}%` }} />
        </div>
        <div
          ref={hueRef}
          className="drawing-toolbar__hue"
          role="slider"
          tabIndex={0}
          aria-label="Color hue"
          aria-valuemin={0}
          aria-valuemax={359}
          aria-valuenow={Math.round(hsv.hue)}
          onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); updateHue(event); }}
          onPointerMove={(event) => { if (event.buttons > 0) updateHue(event); }}
          onKeyDown={(event) => {
            if (event.key === "ArrowUp" || event.key === "ArrowRight") { event.preventDefault(); setHsvColor({ ...hsv, hue: (hsv.hue + 1) % 360 }); }
            if (event.key === "ArrowDown" || event.key === "ArrowLeft") { event.preventDefault(); setHsvColor({ ...hsv, hue: (hsv.hue + 359) % 360 }); }
          }}
        >
          <span className="drawing-toolbar__hue-pointer" style={{ top: `${(hsv.hue / 360) * 100}%` }} />
        </div>
      </div>
    </div>
  );

  return (
    <>
      <div className="drawing-toolbar__swatch-group" role="group" aria-label="Basic colors">{swatches(CHART_DRAWING_COLORS.slice(0, 20))}</div>
      <div className="drawing-toolbar__swatch-group drawing-toolbar__swatch-group--tints" role="group" aria-label="Color shades">{swatches(CHART_DRAWING_COLORS.slice(20))}</div>
      <div className="drawing-toolbar__custom-color-row" role="group" aria-label="Custom colors">
        {customColors.filter((color) => !removedColors.includes(color)).map((color) => (
          <button
            key={color}
            type="button"
            className="drawing-toolbar__custom-swatch"
            style={{ color }}
            aria-label={`Set ${p.label.toLowerCase()} ${color}`}
            aria-pressed={p.color.toLowerCase() === color}
            title={`${color} · right-click to remove`}
            onClick={() => p.onColor(color)}
            onContextMenu={(event) => openRemoveMenu(event, color)}
          />
        ))}
        <button type="button" className="drawing-toolbar__custom-color" title="Add custom color" aria-label="Add custom color" onClick={openCustomPicker}>
          <Plus size={14} aria-hidden="true" />
        </button>
      </div>
      {removeMenu && createPortal(
        <div
          ref={removeMenuRef}
          className="drawing-toolbar__remove-color-menu"
          role="menu"
          style={{ left: removeMenu.x, top: removeMenu.y }}
          onPointerDown={(event) => { event.stopPropagation(); p.onKeepOpen(); }}
          onClick={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            role="menuitem"
            onPointerUp={(event) => {
              if (event.button !== 0) return;
              event.stopPropagation();
              removeColor(removeMenu.color);
            }}
            onClick={(event) => {
              event.stopPropagation();
              if (event.detail === 0) removeColor(removeMenu.color);
            }}
          >
            <Trash size={18} aria-hidden="true" />
            <span>Remove color</span>
          </button>
        </div>,
        document.body,
      )}
      <div className="drawing-toolbar__opacity">
        <div className="drawing-toolbar__opacity-title">Opacity</div>
        <div className="drawing-toolbar__opacity-controls">
          <input
            type="range"
            min="0"
            max="100"
            step="1"
            value={opacityPercent}
            onChange={(event) => setOpacity(Number(event.currentTarget.value))}
            aria-label={`${p.label} opacity`}
            style={{
              backgroundImage: `linear-gradient(90deg, transparent, ${p.color}), repeating-conic-gradient(#444 0% 25%, #666 0% 50%)`,
              backgroundSize: "100% 100%, 8px 8px",
            }}
          />
          <label className="drawing-toolbar__opacity-value">
            <input
              type="number"
              min="0"
              max="100"
              step="1"
              value={opacityPercent}
              onChange={(event) => setOpacity(Number(event.currentTarget.value))}
              aria-label={`${p.label} opacity percent`}
            />
            <span>%</span>
          </label>
        </div>
      </div>
    </>
  );
}
