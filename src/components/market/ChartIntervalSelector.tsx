import { Fragment, useEffect, useRef, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";
import { ChevronDown, Plus, Star } from "lucide-react";
import {
  CHART_INTERVAL_OPTIONS,
  intervalLabel,
  isIntervalEnabled,
  normalizeCustomInterval,
  type IntervalGroupName,
} from "@/lib/market/chartIntervals";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

interface ChartIntervalSelectorProps {
  value: string;
  onChange: (interval: string) => void;
}

const FAVORITES_KEY = "vora.chart.interval_favorites";
const CUSTOM_INTERVALS_KEY = "vora.chart.custom_intervals";

function readStringList(key: string): string[] {
  try {
    const parsed: unknown = JSON.parse(getSetting(key) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return [];
  }
}

function writeStringList(key: string, values: string[]): void {
  setSetting(key, JSON.stringify(values));
}

const GROUP_ORDER: IntervalGroupName[] = ["Ticks", "Seconds", "Minutes", "Hours", "Days", "Weeks", "Months", "Ranges", "VORA M-scale"];

export function ChartIntervalSelector({ value, onChange }: ChartIntervalSelectorProps): React.ReactElement {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const dialogRef = useRef<HTMLFormElement | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [customOpen, setCustomOpen] = useState(false);
  const [position, setPosition] = useState<{ left: number; top: number }>({ left: 8, top: 42 });
  const [favorites, setFavorites] = useState<string[]>(() => readStringList(FAVORITES_KEY));
  const [customIntervals, setCustomIntervals] = useState<string[]>(() => readStringList(CUSTOM_INTERVALS_KEY));
  const [customAmount, setCustomAmount] = useState("2");
  const [customUnit, setCustomUnit] = useState("m");
  const [customError, setCustomError] = useState<string | null>(null);

  useEffect(() => {
    if (!menuOpen && !customOpen) return;
    const closeOnOutsidePointer = (event: PointerEvent): void => {
      const target = event.target;
      if (target instanceof Node && (rootRef.current?.contains(target) || menuRef.current?.contains(target) || dialogRef.current?.contains(target))) return;
      setMenuOpen(false);
      setCustomOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      if (customOpen) setCustomOpen(false);
      else setMenuOpen(false);
    };
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [menuOpen, customOpen]);

  const openMenu = (button: HTMLButtonElement): void => {
    if (menuOpen) {
      setMenuOpen(false);
      return;
    }
    const rect = button.getBoundingClientRect();
    const menuHeight = Math.min(window.innerHeight - 16, 690);
    setPosition({
      left: Math.max(8, Math.min(rect.left, window.innerWidth - 244)),
      top: Math.max(8, Math.min(rect.bottom + 4, window.innerHeight - menuHeight - 8)),
    });
    setMenuOpen(true);
  };

  const selectInterval = (interval: string): void => {
    if (!isIntervalEnabled(interval)) return;
    onChange(interval);
    setMenuOpen(false);
  };

  const toggleFavorite = (interval: string): void => {
    setFavorites((previous) => {
      const next = previous.includes(interval) ? previous.filter((item) => item !== interval) : [...previous, interval];
      writeStringList(FAVORITES_KEY, next);
      return next;
    });
  };

  const addCustomInterval = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const interval = normalizeCustomInterval(customAmount, customUnit);
    if (!interval) {
      setCustomError("Enter a valid interval from 1 second to 12 months.");
      return;
    }
    setCustomIntervals((previous) => {
      const next = [interval, ...previous.filter((item) => item !== interval)];
      writeStringList(CUSTOM_INTERVALS_KEY, next);
      return next;
    });
    onChange(interval);
    setCustomOpen(false);
    setCustomError(null);
  };

  const groups = GROUP_ORDER.map((group) => ({
    group,
    options: CHART_INTERVAL_OPTIONS.filter((option) => option.group === group),
  })).filter(({ options }) => options.length > 0);

  return (
    <div ref={rootRef} className="chart-interval-picker">
      <button
        type="button"
        className="chart-interval-picker__trigger"
        aria-label={`Chart interval ${value}`}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onClick={(event) => openMenu(event.currentTarget)}
      >
        <span>{value}</span>
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      {favorites.filter(isIntervalEnabled).length > 0 && <div className="chart-interval-picker__favorites" role="group" aria-label="Favorite chart intervals">
        {favorites.filter(isIntervalEnabled).map((interval) => <button
          key={`favorite-shortcut-${interval}`}
          type="button"
          className={`chart-shell__interval${value === interval ? " is-active" : ""}`}
          aria-label={intervalLabel(interval)}
          aria-pressed={value === interval}
          title={intervalLabel(interval)}
          onClick={() => selectInterval(interval)}
        >{interval}</button>)}
      </div>}
      {menuOpen && createPortal(
        <div ref={menuRef} role="menu" aria-label="Chart intervals" className="chart-interval-picker__menu" style={position}>
          <button type="button" className="chart-interval-picker__custom-action" onClick={() => { setMenuOpen(false); setCustomOpen(true); }}>
            <Plus size={18} aria-hidden="true" />
            <span>Add custom interval...</span>
          </button>
          {favorites.length > 0 && <>
            <div className="chart-interval-picker__heading">Favorites</div>
            {favorites.map((interval) => <div key={`favorite-${interval}`} className={`chart-interval-picker__row${value === interval ? " is-selected" : ""}`}>
              <button type="button" role="menuitem" disabled={!isIntervalEnabled(interval)} aria-pressed={value === interval} title={isIntervalEnabled(interval) ? intervalLabel(interval) : "This interval is no longer supported"} onClick={() => selectInterval(interval)}>{interval}</button>
              <button type="button" aria-label={`Remove ${interval} from favorites`} title="Remove from favorites" aria-pressed="true" onClick={() => toggleFavorite(interval)}><Star size={16} fill="currentColor" /></button>
            </div>)}
          </>}
          {customIntervals.length > 0 && <>
            <div className="chart-interval-picker__heading">Custom</div>
            {customIntervals.map((interval) => <div key={`custom-${interval}`} className={`chart-interval-picker__row${value === interval ? " is-selected" : ""}`}>
              <button type="button" role="menuitem" onClick={() => selectInterval(interval)}>{interval}</button>
              <button type="button" aria-label={`${favorites.includes(interval) ? "Remove" : "Add"} ${interval} ${favorites.includes(interval) ? "from" : "to"} favorites`} title={favorites.includes(interval) ? "Remove from favorites" : "Add to favorites"} aria-pressed={favorites.includes(interval)} onClick={() => toggleFavorite(interval)}><Star size={16} fill={favorites.includes(interval) ? "currentColor" : "none"} /></button>
            </div>)}
          </>}
          {groups.map(({ group, options }) => <Fragment key={group}>
            <div className="chart-interval-picker__heading">{group}</div>
            {options.map((option) => <div key={option.key} className={`chart-interval-picker__row${value === option.key ? " is-selected" : ""}`}>
              <button type="button" role="menuitem" disabled={!option.enabled} title={option.disabledReason ?? option.label} aria-pressed={value === option.key} onClick={() => selectInterval(option.key)}>{option.label}</button>
              {option.enabled && <button type="button" aria-label={`${favorites.includes(option.key) ? "Remove" : "Add"} ${option.label} ${favorites.includes(option.key) ? "from" : "to"} favorites`} title={favorites.includes(option.key) ? "Remove from favorites" : "Add to favorites"} aria-pressed={favorites.includes(option.key)} onClick={() => toggleFavorite(option.key)}><Star size={16} fill={favorites.includes(option.key) ? "currentColor" : "none"} /></button>}
            </div>)}
          </Fragment>)}
        </div>,
        document.body,
      )}
      {customOpen && createPortal(
        <div className="chart-interval-picker__backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setCustomOpen(false); }}>
          <form ref={dialogRef} className="chart-interval-picker__dialog" aria-label="Add custom interval" onSubmit={addCustomInterval}>
            <h2>Custom interval</h2>
            <div className="chart-interval-picker__custom-fields">
              <input aria-label="Interval amount" type="number" min="1" max={customUnit === "mo" ? 12 : customUnit === "w" ? 52 : customUnit === "d" ? 366 : customUnit === "h" ? 8784 : customUnit === "m" ? 527040 : 31622400} step="1" value={customAmount} onChange={(event) => setCustomAmount(event.currentTarget.value)} />
              <select aria-label="Interval unit" value={customUnit} onChange={(event) => setCustomUnit(event.currentTarget.value)}>
                <option value="s">Seconds</option><option value="m">Minutes</option><option value="h">Hours</option><option value="d">Days</option><option value="w">Weeks</option><option value="mo">Months</option>
              </select>
            </div>
            {customError && <p role="alert">{customError}</p>}
            <footer><button type="button" onClick={() => setCustomOpen(false)}>Cancel</button><button type="submit">Apply</button></footer>
          </form>
        </div>,
        document.body,
      )}
    </div>
  );
}