import { useEffect, useRef, useState, type FormEvent } from "react";
import { CalendarClock, ChevronDown, Pause, Play, Settings2, SkipForward, StepForward, TrendingDown, TrendingUp, X } from "lucide-react";

export type ReplayUpdateInterval = "tick" | "second" | "chart";

interface ChartReplayToolbarProps {
  canSelectStart: boolean;
  canPlay: boolean;
  canStepForward: boolean;
  canJumpToLatest: boolean;
  isSelecting: boolean;
  isPlaying: boolean;
  selectedTimestamp: number | null;
  speed: number;
  activeInterval: string;
  updateInterval: ReplayUpdateInterval;
  autoSelectUpdateInterval: boolean;
  onSelectBarMode: () => void;
  onSelectDateTime: (timestamp: number) => void;
  onSelectFirstAvailable: () => void;
  onSelectRandomBar: () => void;
  onTogglePlayback: () => void;
  onStepForward: () => void;
  onSetSpeed: (speed: number) => void;
  onSetUpdateInterval: (interval: ReplayUpdateInterval) => void;
  onToggleAutoSelectUpdateInterval: () => void;
  onJumpToLatest: () => void;
  onClose: () => void;
}

const SPEED_OPTIONS = [0.25, 0.5, 1, 2, 3, 5, 10];

type OpenMenu = "select" | "date" | "speed" | "step" | "update" | null;

function toUtcInputValue(timestamp: number | null): string {
  const date = new Date((timestamp ?? Math.floor(Date.now() / 1000)) * 1000);
  return date.toISOString().slice(0, 16);
}

export function ChartReplayToolbar({
  canSelectStart,
  canPlay,
  canStepForward,
  canJumpToLatest,
  isSelecting,
  isPlaying,
  selectedTimestamp,
  speed,
  activeInterval,
  updateInterval,
  autoSelectUpdateInterval,
  onSelectBarMode,
  onSelectDateTime,
  onSelectFirstAvailable,
  onSelectRandomBar,
  onTogglePlayback,
  onStepForward,
  onSetSpeed,
  onSetUpdateInterval,
  onToggleAutoSelectUpdateInterval,
  onJumpToLatest,
  onClose,
}: ChartReplayToolbarProps): React.ReactElement {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [openMenu, setOpenMenu] = useState<OpenMenu>(null);
  const [dateTime, setDateTime] = useState(() => toUtcInputValue(selectedTimestamp));
  const updateLabel = autoSelectUpdateInterval || updateInterval === "chart"
    ? activeInterval
    : updateInterval === "second" ? "1s" : "1 tick";

  useEffect(() => {
    if (openMenu === null) return;
    const closeMenu = (event: PointerEvent): void => {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) setOpenMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setOpenMenu(null);
    };
    document.addEventListener("pointerdown", closeMenu);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeMenu);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [openMenu]);

  useEffect(() => {
    if (selectedTimestamp !== null) setDateTime(toUtcInputValue(selectedTimestamp));
  }, [selectedTimestamp]);

  const submitDateTime = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const timestamp = Date.parse(`${dateTime}:00Z`);
    if (!Number.isFinite(timestamp)) return;
    onSelectDateTime(Math.floor(timestamp / 1000));
    setOpenMenu(null);
  };

  const toggleMenu = (menu: Exclude<OpenMenu, null>): void => {
    setOpenMenu((current) => current === menu ? null : menu);
  };

  return (
    <div ref={rootRef} className="chart-replay-toolbar vora-floating-ui" role="group" aria-label="Bar Replay controls">
      <div className="chart-replay-toolbar__controls">
        <div className="chart-replay-toolbar__select-control">
          <button
            type="button"
            className={`chart-replay-toolbar__button chart-replay-toolbar__select${isSelecting ? " is-active" : ""}`}
            aria-label={isSelecting ? "Selecting replay start bar" : "Select replay start bar"}
            aria-pressed={isSelecting}
            disabled={!canSelectStart}
            title={selectedTimestamp === null ? "Select bar" : `Replay starts at ${new Date(selectedTimestamp * 1000).toISOString()}`}
            onClick={onSelectBarMode}
          >
            <svg viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
              <path fill="currentColor" fillRule="evenodd" d="M10 6H8v16h2V6ZM8 5a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1H8Zm15 9.5h-6V19l-5-5 5-5v4.5h6v1Zm-9.586-.5L16 11.414v5.172L13.414 14Z" />
            </svg>
            <span>Select bar</span>
          </button>
          <button
            type="button"
            className="chart-replay-toolbar__button chart-replay-toolbar__select-arrow"
            aria-label="Select replay start mode"
            aria-haspopup="menu"
            aria-expanded={openMenu === "select"}
            disabled={!canSelectStart}
            onClick={() => toggleMenu("select")}
          ><ChevronDown size={14} aria-hidden="true" /></button>
          {openMenu === "select" && <div role="menu" aria-label="Replay start mode" className="chart-replay-toolbar__menu chart-replay-toolbar__menu--select">
            <button type="button" role="menuitem" onClick={() => { onSelectBarMode(); setOpenMenu(null); }}>Bar</button>
            <button type="button" role="menuitem" onClick={() => { setDateTime(toUtcInputValue(selectedTimestamp)); setOpenMenu("date"); }}><CalendarClock size={15} aria-hidden="true" /> Select date and time</button>
            <button type="button" role="menuitem" onClick={() => { onSelectFirstAvailable(); setOpenMenu(null); }}>First available date</button>
            <button type="button" role="menuitem" onClick={() => { onSelectRandomBar(); setOpenMenu(null); }}>Random bar</button>
          </div>}
          {openMenu === "date" && <form className="chart-replay-toolbar__menu chart-replay-toolbar__menu--date" aria-label="Select replay date and time" onSubmit={submitDateTime}>
            <label htmlFor="chart-replay-date">Date and time (UTC)</label>
            <input id="chart-replay-date" type="datetime-local" value={dateTime} onChange={(event) => setDateTime(event.currentTarget.value)} />
            <div className="chart-replay-toolbar__menu-actions">
              <button type="button" onClick={() => setOpenMenu(null)}>Cancel</button>
              <button type="submit" disabled={!dateTime}>Go</button>
            </div>
          </form>}
        </div>

        <span className="chart-replay-toolbar__divider" role="separator" aria-orientation="vertical" />

        <button
          type="button"
          className="chart-replay-toolbar__button chart-replay-toolbar__icon-button"
          aria-label={isPlaying ? "Pause replay" : "Play replay"}
          title={isPlaying ? "Pause replay" : "Play replay"}
          disabled={!canPlay}
          onClick={onTogglePlayback}
        >{isPlaying ? <Pause size={20} fill="currentColor" aria-hidden="true" /> : <Play size={20} fill="currentColor" aria-hidden="true" />}</button>
        <button
          type="button"
          className="chart-replay-toolbar__button chart-replay-toolbar__icon-button"
          aria-label="Step forward"
          title="Step forward"
          disabled={!canStepForward}
          onClick={onStepForward}
        ><StepForward size={20} aria-hidden="true" /></button>

        <div className="chart-replay-toolbar__menu-control">
          <button
            type="button"
            className="chart-replay-toolbar__button chart-replay-toolbar__text-button"
            aria-label={`Playback speed ${speed}x`}
            aria-haspopup="menu"
            aria-expanded={openMenu === "speed"}
            onClick={() => toggleMenu("speed")}
          >{speed}x</button>
          {openMenu === "speed" && <div role="menu" aria-label="Playback speed" className="chart-replay-toolbar__menu chart-replay-toolbar__menu--options">
            {SPEED_OPTIONS.map((option) => <button key={option} type="button" role="menuitemradio" aria-checked={speed === option} onClick={() => { onSetSpeed(option); setOpenMenu(null); }}>{option}x</button>)}
          </div>}
        </div>

        <div className="chart-replay-toolbar__menu-control">
          <button
            type="button"
            className="chart-replay-toolbar__button chart-replay-toolbar__text-button chart-replay-toolbar__step-button"
            aria-label={`Update interval ${updateLabel}`}
            aria-haspopup="menu"
            aria-expanded={openMenu === "update"}
            onClick={() => toggleMenu("update")}
          >{updateLabel}<ChevronDown size={12} aria-hidden="true" /></button>
          {openMenu === "update" && <div role="menu" aria-label="Replay update interval" className="chart-replay-toolbar__menu chart-replay-toolbar__menu--update">
            <button type="button" role="menuitemradio" aria-checked={updateInterval === "tick" && !autoSelectUpdateInterval} disabled title="Tick data is not available" onClick={() => { onSetUpdateInterval("tick"); setOpenMenu(null); }}>1 tick</button>
            <button type="button" role="menuitemradio" aria-checked={updateInterval === "second" && !autoSelectUpdateInterval} onClick={() => { onSetUpdateInterval("second"); setOpenMenu(null); }}>1 second</button>
            <button type="button" role="menuitemradio" aria-checked={(updateInterval === "chart" || autoSelectUpdateInterval)} onClick={() => { onSetUpdateInterval("chart"); setOpenMenu(null); }}>{activeInterval}</button>
            <label className="chart-replay-toolbar__auto-select">
              <span>Auto select interval</span>
              <input type="checkbox" checked={autoSelectUpdateInterval} onChange={onToggleAutoSelectUpdateInterval} />
            </label>
          </div>}
        </div>

        <span className="chart-replay-toolbar__spacer" />
        <span className="chart-replay-toolbar__divider" role="separator" aria-orientation="vertical" />
        <button
          type="button"
          className="chart-replay-toolbar__button chart-replay-toolbar__icon-button"
          aria-label="Jump to latest bar"
          title="Jump to latest bar"
          disabled={!canJumpToLatest}
          onClick={onJumpToLatest}
        ><SkipForward size={20} aria-hidden="true" /></button>
      </div>
      <button type="button" className="chart-replay-toolbar__button chart-replay-toolbar__close" aria-label="Exit Bar Replay" title="Exit Bar Replay" onClick={onClose}><X size={20} aria-hidden="true" /></button>
    </div>
  );
}

const REPLAY_RANGES = ["1D", "5D", "1M", "3M", "6M", "YTD", "1Y", "5Y", "All"] as const;
export type ReplayRange = typeof REPLAY_RANGES[number];

interface ChartReplayFooterProps {
  currentTimestamp: number | null;
  selectedRange: ReplayRange;
  onSelectRange: (range: ReplayRange) => void;
  onSelectDate: (timestamp: number) => void;
}

export function ChartReplayFooter({ currentTimestamp, selectedRange, onSelectRange, onSelectDate }: ChartReplayFooterProps): React.ReactElement {
  const utcClock = currentTimestamp === null
    ? "--:--:-- UTC"
    : `${new Intl.DateTimeFormat("en-GB", { hour: "2-digit", minute: "2-digit", second: "2-digit", timeZone: "UTC", hour12: false }).format(new Date(currentTimestamp * 1000))} UTC`;

  return (
    <div className="chart-replay-footer vora-floating-ui">
      <div className="chart-replay-footer__controls">
        <div className="chart-replay-range-strip" role="group" aria-label="Replay chart range">
          {REPLAY_RANGES.map((range) => <button key={range} type="button" aria-pressed={selectedRange === range} onClick={() => onSelectRange(range)}>{range}</button>)}
          <label className="chart-replay-date-picker" title="Go to date">
            <CalendarClock size={13} aria-hidden="true" />
            <input type="date" aria-label="Go to replay date" onChange={(event) => {
              const timestamp = Date.parse(`${event.currentTarget.value}T00:00:00Z`);
              if (Number.isFinite(timestamp)) onSelectDate(Math.floor(timestamp / 1000));
            }} />
          </label>
        </div>
        <div className="chart-replay-trading-dock" aria-label="Replay Trading tools">
          <span className="chart-replay-trading-dock__title">Replay Trading</span>
          <span className="chart-replay-trading-dock__divider" aria-hidden="true" />
          <button type="button" disabled title="Long position tools are not available in replay" aria-label="Long position"><TrendingUp size={15} aria-hidden="true" /></button>
          <button type="button" disabled title="Short position tools are not available in replay" aria-label="Short position"><TrendingDown size={15} aria-hidden="true" /></button>
          <button type="button" disabled title="Replay trading settings are not available" aria-label="Replay trading settings"><Settings2 size={15} aria-hidden="true" /></button>
        </div>
      </div>
      <time className="chart-replay-utc-clock">{utcClock}</time>
    </div>
  );
}
