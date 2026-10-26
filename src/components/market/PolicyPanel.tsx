// ═══════════════════════════════════════════════════════════════
// PolicyPanel — live tuning controls for box detection
// ═══════════════════════════════════════════════════════════════

import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, RotateCcw, X } from "lucide-react";
import type { BoxExtend, Scale, ScaleRegistry } from "@/lib/market/marketApi";
import { firstValidHex } from "@/lib/market/colorUtils";
import { UpdateCheckButton } from "@/shared/components/UpdateCheckButton";
import packageJson from "../../../package.json";

export interface PolicyDefaults {
  s_small:      number;
  s_medium:     number;
  s_large:      number;
  box_extend: BoxExtend;   // per-edge fractional candles, -20..+20, default {0,0}
  show_engulf_zone: boolean;
  use_local_proxy: boolean;
  proxy_protocol: "socks5" | "http";
  proxy_host: string;
  proxy_port: number;
  z0_scale: number;
  extend_to_engulf: boolean;
  user_step: number;       // MKT-6b-6.3.0d: user step 0..10 (x5 ladder, 11 steps)
  narrative_speed_font_size: number;
  narrative_count_font_size: number;
}

// M-specific role defaults (small, medium, large)
export const M_ROLE_DEFAULTS: Record<number, [number, number, number]> = {
  0: [0, 1, 2],
  1: [1, 2, 3],
  2: [2, 3, 4],
  3: [3, 4, 5],
  4: [4, 5, 6],
  5: [5, 6, 7],
  6: [6, 7, 8],
  7: [7, 8, 9],
  8: [7, 8, 9],
  9: [7, 8, 9],
  10: [7, 8, 9],
};

export function getDefaultsForM(m: number): PolicyDefaults {
  const [s, md, l] = M_ROLE_DEFAULTS[m] ?? [4, 5, 6];
  return {
    s_small:      s,
    s_medium:     md,
    s_large:      l,
    box_extend: { left: 0, right: 0 },
    show_engulf_zone: true,
    use_local_proxy: true,
    proxy_protocol: "socks5",
    proxy_host: "127.0.0.1",
    proxy_port: 10808,
    z0_scale: 5,
    extend_to_engulf: false,
    user_step: Math.max(0, Math.min(10, Math.round(m))),
    narrative_speed_font_size: 20,
    narrative_count_font_size: 20,
  };
}

export const DEFAULT_POLICY: PolicyDefaults = getDefaultsForM(3);

interface Props {
  values: PolicyDefaults;
  onChange: (v: PolicyDefaults) => void;
  onClose: () => void;
  registry: ScaleRegistry | null;
  onScaleChange: (i: number, patch: Partial<Scale>) => void;
  onAddScale: () => void;
  onRemoveScale: (i: number) => void;
  onSaveRegistry: () => void;
  isSaving: boolean;
  saveOk: boolean;
  validationError: string | null;
}

interface SectionProps {
  title: string;
  children: ReactNode;
}

function Section({ title, children }: SectionProps) {
  const [open, setOpen] = useState(true);
  return (
    <div className="border-b border-[#383838] last:border-b-0">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-1 px-3 py-2
                   text-[13px] font-medium uppercase tracking-wider text-[#d6dbe2] hover:text-white transition"
      >
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        {title}
      </button>
      {open && <div className="px-3 pb-3">{children}</div>}
    </div>
  );
}

export function PolicyPanel({
  values,
  onChange,
  onClose,
  registry,
  onScaleChange,
  onAddScale,
  onRemoveScale,
  onSaveRegistry,
  isSaving,
  saveOk,
  validationError,
  autoDetect,
  onAutoDetectChange,
  defaultRouteSetting,
  onDefaultRouteChange,
  drawings: _drawings,
}: Props & {
  /** v7.2.0 Part A — master switch for the auto RBR/DBD detection pipeline. */
  autoDetect: boolean;
  onAutoDetectChange: (v: boolean) => void;
  /** v7.4.2 — default route is fixed to Home; the segmented control is intentionally removed. */
  defaultRouteSetting?: "home" | "chart";
  onDefaultRouteChange?: (r: "home" | "chart") => void;
  /** v0.82.0 — DRAWINGS removed from Policy Panel (now in floating DrawingToolbar).
      Kept optional for backward-compat with callers still passing prefs. */
  drawings?: {
    color: string;
    rectW: number;
    hW: number;
    vW: number;
    rW: number;
    tW: number;
    onPatch: (p: { color?: string; rectW?: number; hW?: number; vW?: number; rW?: number; tW?: number }) => void;
  };
}) {
  const [draftNum, setDraftNumMap] = useState<Record<string, string>>({});
  const set = <K extends keyof PolicyDefaults>(
    key: K,
    v: PolicyDefaults[K],
  ) => {
    onChange({ ...values, [key]: v });
  };

  const reset = () => onChange({ ...DEFAULT_POLICY });
  void _drawings;
  const setDraftNum = (key: string, v: string) =>
    setDraftNumMap((d) => ({ ...d, [key]: v }));
  const clearDraftNum = (key: string) =>
    setDraftNumMap((d) => {
      const n = { ...d }; delete n[key]; return n;
    });

  return (
    <div className="vora-floating-ui absolute top-2 right-2 z-20 w-72
            max-h-[calc(100vh-100px)] overflow-y-auto
            rounded bg-[#1e1e1e]
            border border-[#383838] shadow-[0_8px_24px_rgba(0,0,0,0.45)]
            text-[#d6dbe2] font-sans">
      <div className="flex items-center justify-between px-3 py-2
              border-b border-[#383838] sticky top-0
              bg-[#1e1e1e] z-10">
      <div className="flex min-w-0 items-center gap-1.5">
        <span className="text-[13px] font-semibold text-[#ff9800]">
          Policy Panel
        </span>
        <span className="text-[13px] font-medium text-[#d6dbe2]">
          v{packageJson.version}
        </span>
        <UpdateCheckButton className="flex h-4 w-4 shrink-0 items-center justify-center rounded text-[#9a9da5] transition hover:text-white disabled:cursor-wait" />
      </div>
        <button
          onClick={onClose}
          className="w-5 h-5 flex items-center justify-center
                     rounded hover:bg-[#2a2a2a] text-[#787b86]
                     hover:text-[#d6dbe2]"
        >
          <X size={12} />
        </button>
      </div>

      <div className="px-3 pt-2">
        <label className="flex items-center gap-2 text-[13px] text-[#d6dbe2] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={autoDetect}
            onChange={(e) => onAutoDetectChange(e.target.checked)}
            className="w-3.5 h-3.5 accent-[#ff9800]"
          />
          Auto-detect Boxes (RBR / DBD)
        </label>
      </div>

      <Section title="Local Proxy Settings">
        <label className="flex items-center gap-2 text-[13px] text-[#d6dbe2] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={values.use_local_proxy}
            onChange={(e) => set("use_local_proxy", e.target.checked)}
            className="w-3.5 h-3.5 accent-[#ff9800]"
          />
          Enable local proxy
        </label>
        <label className="block text-[13px] text-[#d6dbe2] mt-2">
          Protocol
          <select
            value={values.proxy_protocol}
            onChange={(e) => {
              const protocol = e.target.value;
              if (protocol === "socks5" || protocol === "http") {
                set("proxy_protocol", protocol);
              }
            }}
            className="block w-full mt-1 px-2 py-1 text-[13px] bg-[#0f0f0f] border border-[#383838] rounded"
          >
            <option value="socks5">SOCKS5</option>
            <option value="http">HTTP</option>
          </select>
        </label>
        <label className="block text-[13px] text-[#d6dbe2] mt-2">
          Proxy IP
          <input
            type="text"
            value={values.proxy_host}
            onChange={(e) => set("proxy_host", e.target.value)}
            placeholder="127.0.0.1"
            className="block w-full mt-1 px-2 py-1 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded"
          />
        </label>
        <label className="block text-[13px] text-[#d6dbe2] mt-2">
          Proxy Port
          <input
            type="number"
            min={1}
            max={65535}
            step={1}
            value={values.proxy_port}
            onChange={(e) => {
              const port = Number(e.target.value);
              if (Number.isInteger(port) && port >= 1 && port <= 65535) {
                set("proxy_port", port);
              }
            }}
            className="block w-full mt-1 px-2 py-1 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded"
          />
        </label>
      </Section>

      <Section title="Layout">
        {/* Left Edge (AC) */}
        <label className="block text-[13px] font-mono text-white mt-1">Box Extend — Left Edge (AC)</label>
        <input
          type="number"
          step={0.01}
          min={-20}
          max={20}
          value={values.box_extend.left}
          onChange={(e) => onChange({
            ...values,
            box_extend: {
              ...values.box_extend,
              left: parseFloat(e.target.value) || 0,
            },
          })}
          className="w-full px-1 py-0.5 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded"
        />
        {/* Right Edge (BD) */}
        <label className="block text-[13px] font-mono text-white mt-1">Box Extend — Right Edge (BD)</label>
        <input
          type="number"
          step={0.01}
          min={-20}
          max={20}
          value={values.box_extend.right}
          onChange={(e) => onChange({
            ...values,
            box_extend: {
              ...values.box_extend,
              right: parseFloat(e.target.value) || 0,
            },
          })}
          className="w-full px-1 py-0.5 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded"
        />
        {/* Engulf Zone toggle */}
        <label className="flex items-center gap-2 text-[13px] font-mono text-white mt-2">
          <input
            type="checkbox"
            checked={values.show_engulf_zone}
            onChange={(e) => onChange({
              ...values,
              show_engulf_zone: e.target.checked,
            })}
          />
          Show Engulf Zone
        </label>
        <label className="flex items-center gap-2 text-[13px] font-mono text-white mt-2">
          <input
            type="checkbox"
            checked={values.extend_to_engulf}
            onChange={(e) => onChange({
              ...values,
              extend_to_engulf: e.target.checked,
            })}
          />
          Extend Box to Engulf
        </label>
            
          
        <label className="block text-[13px] font-mono text-white mt-2">Z0 Scale (base)</label>
        <input
          type="number"
          min={0}
          max={9}
          step={1}
          value={values.z0_scale}
          onChange={(e) => onChange({
            ...values,
            z0_scale: Math.max(0, Math.min(9,
              parseInt(e.target.value, 10) || 0)),
          })}
          className="w-full px-1 py-0.5 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded"
        />
      </Section>

      <Section title="S-Scale Roles">
        <div className="grid grid-cols-3 gap-2 mb-2">
          <div className="text-center">
            <div className="text-[12px] text-[#4caf50] mb-1">small</div>
            <input
              type="number" min={0} max={9}
              value={values.s_small}
              onChange={(e) => set("s_small", parseInt(e.target.value) || 0)}
              className="w-full px-1 py-0.5 text-center text-[13px] font-mono
                         bg-[#0f0f0f] border border-[#383838] rounded"
            />
          </div>
          <div className="text-center">
            <div className="text-[12px] text-[#ff9800] mb-1">medium</div>
            <input
              type="number" min={0} max={9}
              value={values.s_medium}
              onChange={(e) => set("s_medium", parseInt(e.target.value) || 0)}
              className="w-full px-1 py-0.5 text-center text-[13px] font-mono
                         bg-[#0f0f0f] border border-[#383838] rounded"
            />
          </div>
          <div className="text-center">
            <div className="text-[12px] text-[#9c27b0] mb-1">large</div>
            <input
              type="number" min={0} max={9}
              value={values.s_large}
              onChange={(e) => set("s_large", parseInt(e.target.value) || 0)}
              className="w-full px-1 py-0.5 text-center text-[13px] font-mono
                         bg-[#0f0f0f] border border-[#383838] rounded"
            />
          </div>
        </div>
      </Section>

      <Section title="Narrative Player">
        <div className="flex flex-col gap-3">
          <label className="flex flex-col gap-1.5 text-[12px] text-[#d6dbe2]">
            <span className="flex items-center justify-between">
              <span>Speed font</span>
              <span className="font-mono text-[#ffb74d]">{values.narrative_speed_font_size}px</span>
            </span>
            <input
              type="range"
              min={10}
              max={32}
              step={1}
              value={values.narrative_speed_font_size}
              onChange={(e) => set("narrative_speed_font_size", Number(e.target.value))}
              className="w-full accent-[#ff9800]"
            />
          </label>
          <label className="flex flex-col gap-1.5 text-[12px] text-[#d6dbe2]">
            <span className="flex items-center justify-between">
              <span>Counter font</span>
              <span className="font-mono text-[#ffb74d]">{values.narrative_count_font_size}px</span>
            </span>
            <input
              type="range"
              min={10}
              max={32}
              step={1}
              value={values.narrative_count_font_size}
              onChange={(e) => set("narrative_count_font_size", Number(e.target.value))}
              className="w-full accent-[#ff9800]"
            />
          </label>
        </div>
      </Section>

      {/* S-SCALE REGISTRY (v6.4.0) — tolerance_pct hidden */}
      <Section title="S-Scale Registry">
        {!registry && (
          <span className="block text-[13px] font-mono text-white">
            Loading registry…
          </span>
        )}
        {registry && (
          <div className="flex flex-col gap-1.5">
            <button
              type="button"
              onClick={onAddScale}
              disabled={registry.scales.length >= 20}
              className="self-start px-2 py-1 rounded bg-[#0f0f0f] hover:bg-[#2a2a2a]
                         border border-[#383838] text-[13px] text-[#d6dbe2]
                         disabled:opacity-40 disabled:cursor-not-allowed transition"
            >
              + Add Scale
            </button>
            {registry.scales.map((s, i) => (
              <div key={s.id} className="grid grid-cols-12 gap-1 items-center">
                <input
                  type="checkbox"
                  className="col-span-1"
                  checked={s.enabled}
                  onChange={(e) =>
                    onScaleChange(i, { enabled: e.target.checked })
                  }
                />
                <span className="col-span-1 text-[13px] font-mono text-white">
                  {s.label}
                </span>
                {/* circle swatch — clickable (v6.4.5: only color control) */}
                <label
                  className="col-span-1 relative flex items-center justify-center cursor-pointer"
                  title={s.color}
                >
                  <input
                    type="color"
                    value={firstValidHex(s.color)}
                    onChange={(e) => {
                      onScaleChange(i, { color: e.target.value });
                    }}
                    aria-label={`${s.label} color`}
                    className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                  />
                  <span
                    className="block rounded-full border-2 border-white/40 shadow-[0_0_0_1px_rgba(0,0,0,0.6)] pointer-events-none"
                    style={{
                      width: 14,
                      height: 14,
                      flex: '0 0 14px',
                      backgroundColor: firstValidHex(s.color),
                    }}
                  />
                </label>
                <input
                  type="text"
                  inputMode="decimal"
                  value={draftNum[`s${i}`] ?? String(s.start_pct)}
                  onChange={(e) => {
                    const v = e.target.value;
                    setDraftNum(`s${i}`, v);
                    const n = parseFloat(v);
                    if (Number.isFinite(n) && n >= 0) {
                      onScaleChange(i, { start_pct: n });
                    }
                  }}
                  onBlur={() => {
                    const v = draftNum[`s${i}`];
                    if (v === undefined) return;
                    const n = parseFloat(v);
                    if (!Number.isFinite(n) || n < 0) {
                      setDraftNum(`s${i}`, String(s.start_pct));
                    }
                    clearDraftNum(`s${i}`);
                  }}
                  className="col-span-4 h-6 px-1.5 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded text-[#d6dbe2] outline-none focus:border-[#ff9800] text-right"
                />
                <input
                  type="text"
                  inputMode="decimal"
                  value={draftNum[`e${i}`] ?? String(s.end_pct)}
                  onChange={(e) => {
                    const v = e.target.value;
                    setDraftNum(`e${i}`, v);
                    const n = parseFloat(v);
                    if (Number.isFinite(n) && n >= 0) {
                      onScaleChange(i, { end_pct: n });
                    }
                  }}
                  onBlur={() => {
                    const v = draftNum[`e${i}`];
                    if (v === undefined) return;
                    const n = parseFloat(v);
                    if (!Number.isFinite(n) || n < 0) {
                      setDraftNum(`e${i}`, String(s.end_pct));
                    }
                    clearDraftNum(`e${i}`);
                  }}
                  className="col-span-4 h-6 px-1.5 text-[13px] font-mono bg-[#0f0f0f] border border-[#383838] rounded text-[#d6dbe2] outline-none focus:border-[#ff9800] text-right"
                />
                <button
                  type="button"
                  title="Remove scale"
                  onClick={() => onRemoveScale(i)}
                  disabled={registry.scales.length <= 1}
                  className="col-span-1 text-center text-white hover:text-[#ef5350] disabled:opacity-30 disabled:cursor-not-allowed"
                >
                  ×
                </button>
              </div>
            ))}
            {validationError && (
              <div className="text-red-400 text-[13px] font-mono mt-1">
                {validationError}
              </div>
            )}
            <button
              onClick={onSaveRegistry}
              disabled={isSaving || !!validationError}
              className="mt-1 px-3 py-1.5 rounded bg-[#ffb74d] hover:bg-[#ffa726]
                         disabled:opacity-40 disabled:cursor-not-allowed
                         text-[13px] font-semibold text-black transition"
            >
              {isSaving ? "Saving…" : "Save Registry"}
            </button>
            {saveOk && (
              <span className="text-green-400 text-[13px]">Saved ✓</span>
            )}
          </div>
        )}
      </Section>

      <div className="p-3 border-t border-[#383838]">
        <button
          onClick={reset}
          className="w-full flex items-center justify-center gap-2
                     px-3 py-1.5 rounded bg-[#0f0f0f] hover:bg-[#2a2a2a]
                     border border-[#383838] text-[13px] text-[#d6dbe2]
                     transition"
        >
          <RotateCcw size={12} />
          Reset to Defaults
        </button>
      </div>
    </div>
  );
}
