import React, { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import {
  Activity,
  AlertCircle,
  BarChart3,
  Bell,
  Check,
  ChevronDown,
  Crosshair,
  Database,
  Eraser,
  Layers,
  LayoutGrid,
  LineChart,
  Maximize2,
  Minus,
  MoreHorizontal,
  MousePointer2,
  PanelRight,
  Pencil,
  Play,
  RefreshCw,
  Redo2,
  Ruler,
  Search,
  Settings2,
  Square,
  Type,
  Undo2,
  X,
} from "lucide-react";
import type { CandleDto, MarketInstrument } from "@/lib/market/marketApi";
import { CandlestickChart } from "@/lib/market/CandlestickChart";
import "./MarketPage.css";

type ChartDisplayMode = "special" | "candlestick";
type MenuId = "symbol" | "chart" | "indicators" | "settings" | null;
type PanelId = "watchlist" | "details";

const TIMEFRAMES = ["1m", "5m", "15m", "30m", "1h", "4h", "1d"];
const RANGE_PRESETS = ["1D", "5D", "1M", "3M", "6M", "YTD", "1Y", "5Y", "All"];

const DRAWING_TOOLS: Array<{ id: string; label: string; icon: React.ReactNode }> = [
  { id: "cursor", label: "Cursor", icon: <MousePointer2 size={17} /> },
  { id: "crosshair", label: "Crosshair", icon: <Crosshair size={17} /> },
  { id: "trend", label: "Trend line", icon: <TrendingIcon /> },
  { id: "horizontal", label: "Horizontal line", icon: <Minus size={17} /> },
  { id: "vertical", label: "Vertical line", icon: <Activity size={17} /> },
  { id: "rectangle", label: "Rectangle", icon: <Square size={16} /> },
  { id: "text", label: "Text annotation", icon: <Type size={17} /> },
  { id: "measure", label: "Measure", icon: <Ruler size={17} /> },
  { id: "brush", label: "Brush", icon: <Pencil size={16} /> },
  { id: "erase", label: "Delete drawings", icon: <Eraser size={16} /> },
];

function TrendingIcon(): React.ReactElement {
  return <LineChart size={17} />;
}

function formatPrice(price: number): string {
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(price);
}

function formatUtc(timestamp: number): string {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return "Unavailable";
  return new Intl.DateTimeFormat("en-GB", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "UTC",
  }).format(new Date(timestamp * 1000)) + " UTC";
}

function presentMarketError(error: string): string {
  if (error.includes("Cannot read properties of undefined") && error.includes("invoke")) {
    return "The browser preview cannot access local tick files. Open this page in the desktop app to load Parquet data.";
  }
  return error;
}
interface WorkspaceProps {
  sources: MarketInstrument[];
  activeSource: MarketInstrument;
  symbol: string;
  timeframe: string;
  candles: CandleDto[];
  loading: boolean;
  error: string | null;
  hoveredCandle: CandleDto | null;
  onSelectSource: (source: MarketInstrument) => void;
  onTimeframeChange: (timeframe: string) => void;
  onRefresh: () => void;
  onCrosshairMove: (candle: CandleDto | null) => void;
}

export function MarketWorkspace(props: WorkspaceProps): React.ReactElement {
  const {
    sources,
    activeSource,
    symbol,
    timeframe,
    candles,
    loading,
    error,
    hoveredCandle,
    onSelectSource,
    onTimeframeChange,
    onRefresh,
    onCrosshairMove,
  } = props;
  const [selectedTool, setSelectedTool] = useState("cursor");
  const [chartType, setChartType] = useState<ChartDisplayMode>("candlestick");
  const [watchlistOpen, setWatchlistOpen] = useState(true);
  const [rightPanel, setRightPanel] = useState<PanelId>("watchlist");
  const [openMenu, setOpenMenu] = useState<MenuId>(null);
  const [sourceQuery, setSourceQuery] = useState("");
  const [fullscreen, setFullscreen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const workspaceRef = useRef<HTMLDivElement | null>(null);
  const lastCandle = candles[candles.length - 1] ?? null;
  const quote = hoveredCandle ?? lastCandle;

  const filteredSources = useMemo(() => {
    const query = sourceQuery.trim().toLowerCase();
    if (!query) return sources;
    return sources.filter((source) =>
      `${source.symbol} ${source.kind} ${source.id}`.toLowerCase().includes(query),
    );
  }, [sourceQuery, sources]);

  useEffect(() => {
    const onPointerDown = (event: PointerEvent): void => {
      const target = event.target;
      if (target instanceof Node && workspaceRef.current && !workspaceRef.current.contains(target)) {
        setOpenMenu(null);
      }
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setOpenMenu(null);
        setNotice(null);
      }
    };
    const onFullscreenChange = (): void => setFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("fullscreenchange", onFullscreenChange);
    };
  }, []);

  const toggleMenu = (menu: Exclude<MenuId, null>): void => {
    setOpenMenu((current) => current === menu ? null : menu);
  };

  const chooseSource = (source: MarketInstrument): void => {
    onSelectSource(source);
    setSourceQuery("");
    setOpenMenu(null);
  };

  const toggleFullscreen = async (): Promise<void> => {
    const root = workspaceRef.current;
    if (!root) return;
    try {
      if (document.fullscreenElement === root) await document.exitFullscreen();
      else if (!document.fullscreenElement) await root.requestFullscreen();
    } catch {
      setNotice("Fullscreen is unavailable in this runtime.");
    }
  };

  const displayName = symbol || "Select online instrument";
  const selectedSourcePrice = lastCandle ? formatPrice(lastCandle.close) : "—";

  return (
    <div ref={workspaceRef} className="market-workspace">
      <header className="market-topbar">
        <div className="market-topbar__left">
          <Link to="/" className="market-brand" aria-label="Return to Vora workspace">
            <span className="market-brand__mark"><Activity size={15} /></span>
            <span className="market-brand__name">VORA</span>
            <span className="market-brand__section">MARKETS</span>
          </Link>
          <nav className="market-nav" aria-label="Market workspaces">
            <Link to="/market" className="is-active">Chart</Link>
            <Link to="/scale-lab">M-Scale</Link>
            <Link to="/sources">Data sources</Link>
          </nav>
        </div>
        <div className="market-topbar__right">
          <span className={`market-badge ${error ? "is-warning" : ""}`}>
            <i className={`market-status-dot ${error ? "is-error" : ""}`} />
            {error ? "FEED UNAVAILABLE" : "DUKASCOPY ONLINE"}
          </span>
          <button type="button" className="market-icon-button" title="More workspace actions" aria-label="More workspace actions" onClick={() => toggleMenu("settings")}>
            <MoreHorizontal size={17} />
          </button>
        </div>
      </header>

      <section className="market-toolbar" aria-label="Chart toolbar">
        <div className="market-toolbar-scroll">
          <div className="market-symbol-control">
            <button
              type="button"
              className="market-toolbar-button market-symbol-button"
              aria-expanded={openMenu === "symbol"}
              onClick={() => toggleMenu("symbol")}
            >
              <span className="market-symbol-button__label"><Search size={14} />{displayName}</span>
              <ChevronDown size={13} />
            </button>
            {openMenu === "symbol" && (
              <div className="market-menu" role="dialog" aria-label="Search online symbols">
                <label className="market-symbol-search">
                  <Search size={14} />
                  <input
                    autoFocus
                    type="search"
                    value={sourceQuery}
                    onChange={(event) => setSourceQuery(event.target.value)}
                    placeholder="Search online symbols"
                  />
                </label>
                <div className="market-symbol-list">
                  {filteredSources.length === 0 ? (
                    <div className="market-watchlist-empty">No matching online instruments.</div>
                  ) : filteredSources.map((source) => (
                    <button key={source.id} type="button" className="market-source-option" onClick={() => chooseSource(source)}>
                      <span><span className="market-source-option__symbol">{source.symbol}</span><span className="market-source-option__meta">{source.kind} · {source.row_count.toLocaleString()} rows</span></span>
                      {source.id === activeSource?.id && <Check size={14} />}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="market-separator" />
          <div className="market-timeframes" role="group" aria-label="Timeframe">
            {TIMEFRAMES.map((interval) => (
              <button
                key={interval}
                type="button"
                className={timeframe === interval ? "is-active" : ""}
                aria-pressed={timeframe === interval}
                onClick={() => onTimeframeChange(interval)}
              >
                {interval}
              </button>
            ))}
          </div>
          <div className="market-separator" />

          <div className="market-menu-anchor">
            <button type="button" className="market-toolbar-button" aria-expanded={openMenu === "chart"} onClick={() => toggleMenu("chart")}>
              <BarChart3 size={15} />
              {chartType === "candlestick" ? "Candles" : "M-Scale"}
              <ChevronDown size={12} />
            </button>
            {openMenu === "chart" && (
              <div className="market-menu" role="menu" aria-label="Chart type">
                <div className="market-menu__title">Chart type</div>
                <button type="button" className="market-menu__row" role="menuitemradio" aria-checked={chartType === "candlestick"} onClick={() => { setChartType("candlestick"); setOpenMenu(null); }}>
                  <BarChart3 size={15} /> Candles {chartType === "candlestick" && <Check size={14} />}
                </button>
                <button type="button" className="market-menu__row" role="menuitemradio" aria-checked={chartType === "special"} onClick={() => { setChartType("special"); setOpenMenu(null); }}>
                  <LineChart size={15} /> M-Scale path {chartType === "special" && <Check size={14} />}
                </button>
              </div>
            )}
          </div>

          <div className="market-menu-anchor">
            <button type="button" className="market-toolbar-button" aria-expanded={openMenu === "indicators"} onClick={() => toggleMenu("indicators")}>
              <Activity size={15} /> Indicators <ChevronDown size={12} />
            </button>
            {openMenu === "indicators" && (
              <div className="market-menu" role="dialog" aria-label="Indicators">
                <div className="market-menu__title">Future phase</div>
                <button type="button" className="market-menu__row" disabled title="Indicator calculations are not included in Phase 1.">RSI <small>not calculated</small></button>
                <button type="button" className="market-menu__row" disabled title="Indicator calculations are not included in Phase 1.">MACD <small>not calculated</small></button>
              </div>
            )}
          </div>

          <button type="button" className="market-toolbar-button" disabled title="Price-alert evaluation is not included in Phase 1."><Bell size={15} /> Alert</button>
          <button type="button" className="market-toolbar-button" disabled title="Bar replay is not included in Phase 1."><Play size={14} /> Replay</button>
          <div className="market-separator" />
          <button type="button" className="market-icon-button" disabled title="Drawing history is not enabled in this chart workspace." aria-label="Undo"><Undo2 size={16} /></button>
          <button type="button" className="market-icon-button" disabled title="Drawing history is not enabled in this chart workspace." aria-label="Redo"><Redo2 size={16} /></button>
        </div>

        <div className="market-toolbar__right">
          <button type="button" className="market-icon-button" onClick={onRefresh} disabled={loading} title="Refresh local candles" aria-label="Refresh local candles">
            <RefreshCw size={15} className={loading ? "market-spin" : ""} />
          </button>
          <div className="market-menu-anchor">
            <button type="button" className="market-icon-button" aria-expanded={openMenu === "settings"} onClick={() => toggleMenu("settings")} title="Chart settings" aria-label="Chart settings"><Settings2 size={16} /></button>
            {openMenu === "settings" && (
              <div className="market-menu market-menu--right" role="dialog" aria-label="Chart settings">
                <div className="market-menu__row" aria-disabled="true"><Crosshair size={14} /><span>Crosshair</span><small>fixed style</small></div>
                <div className="market-menu__title">Layout</div>
                <button type="button" className="market-menu__row" onClick={() => { setWatchlistOpen((open) => !open); setOpenMenu(null); }}><PanelRight size={14} />{watchlistOpen ? "Hide right panel" : "Show right panel"}</button>
              </div>
            )}
          </div>
          <button type="button" className="market-icon-button" onClick={() => void toggleFullscreen()} title={fullscreen ? "Exit fullscreen" : "Fullscreen"} aria-label={fullscreen ? "Exit fullscreen" : "Fullscreen"}><Maximize2 size={15} /></button>
        </div>
      </section>

      <div className={`market-layout ${watchlistOpen ? "" : "market-layout--closed"}`}>
        <aside className="market-toolrail" aria-label="Drawing tools">
          {DRAWING_TOOLS.map((tool, index) => (
            <React.Fragment key={tool.id}>
              {index === 2 || index === 6 ? <span className="market-toolrail__separator" /> : null}
              <button
                type="button"
                className={`market-tool-button ${selectedTool === tool.id ? "is-active" : ""}`}
                aria-label={tool.label}
                aria-pressed={selectedTool === tool.id}
                title={`${tool.label} · visual selection only in Phase 1`}
                onClick={() => setSelectedTool(tool.id)}
              >
                {tool.icon}
              </button>
            </React.Fragment>
          ))}
          <span className="market-toolrail__spacer" />
          <button type="button" className="market-tool-button" title="Drawing persistence is not included in Phase 1" aria-label="Drawing visibility" disabled><Layers size={16} /></button>
          <button type="button" className="market-tool-button" title="Drawing deletion is not included in Phase 1" aria-label="Delete drawings" disabled><Eraser size={16} /></button>
        </aside>

        <main className="market-chart-column">
          <header className="market-chart-heading">
            <div className="market-chart-title">
              <span className="market-instrument-mark">{symbol ? symbol.slice(0, 2) : "--"}</span>
              <span><span className="market-instrument-name">{displayName}</span><span className="market-instrument-meta">{activeSource.kind} · {timeframe}</span></span>
            </div>
            {quote && (
              <div className="market-ohlc" aria-label={hoveredCandle ? "Hovered candle OHLC" : "Latest candle OHLC"}>
                <span>{hoveredCandle ? "HOVER" : "LAST"}</span>
                <span>O <b>{formatPrice(quote.open)}</b></span>
                <span>H <b className="is-up">{formatPrice(quote.high)}</b></span>
                <span>L <b className="is-down">{formatPrice(quote.low)}</b></span>
                <span>C <b className={quote.close >= quote.open ? "is-up" : "is-down"}>{formatPrice(quote.close)}</b></span>
              </div>
            )}
          </header>
          <section className="market-chart-stage" aria-label="Price chart">
            {error && <div className="market-error"><AlertCircle size={14} /><span>{presentMarketError(error)}</span></div>}
            {candles.length > 0 ? (
              <div className="market-chart-canvas">
                <CandlestickChart
                  candles={candles}
                  onCrosshairMove={onCrosshairMove}
                  displayMode={chartType}
                />
              </div>
            ) : !loading ? (
              <div className="market-chart-empty">
                <Database size={23} />
                <strong>No online candles for {symbol} {timeframe}</strong>
                <p>{error ? presentMarketError(error) : "No local Parquet data was returned for this instrument and timeframe."}</p>
              </div>
            ) : (
              <div className="market-chart-loading"><RefreshCw size={15} className="market-spin" /> Loading local candles…</div>
            )}
            {loading && candles.length > 0 && <div className="market-chart-loading"><RefreshCw size={14} className="market-spin" /> Updating local candles…</div>}
          </section>
          <div className="market-rangebar" aria-label="Historical range presets">
            <span className="market-rangebar__label">RANGE</span>
            {RANGE_PRESETS.map((range) => <button key={range} type="button" disabled title="Historical range navigation is not included in Phase 1">{range}</button>)}
            <span className="market-rangebar__note">Range navigation · future phase</span>
          </div>
        </main>

        {watchlistOpen && (
          <aside className="market-watchlist" aria-label="Market watchlist and details">
            <div className="market-watchlist-heading">
              <div className="market-panel-actions">
                <button type="button" className={`market-toolbar-button ${rightPanel === "watchlist" ? "is-active" : ""}`} onClick={() => setRightPanel("watchlist")}><LayoutGrid size={14} />Watchlist</button>
                <button type="button" className={`market-icon-button ${rightPanel === "details" ? "is-active" : ""}`} onClick={() => setRightPanel("details")} aria-label="Instrument details" title="Instrument details"><Database size={15} /></button>
              </div>
              <div className="market-panel-actions">
                <button type="button" className="market-icon-button" title="Adding external symbols is not included in Phase 1" aria-label="Add symbol" disabled><span aria-hidden="true">+</span></button>
                <button type="button" className="market-icon-button" onClick={() => setWatchlistOpen(false)} title="Hide panel" aria-label="Hide right panel"><X size={15} /></button>
              </div>
            </div>
            {rightPanel === "watchlist" ? (
              <>
                <label className="market-watchlist-search"><Search size={13} /><input value={sourceQuery} onChange={(event) => setSourceQuery(event.target.value)} placeholder="Filter online symbols" /></label>
                <div className="market-watchlist-columns"><span>SYMBOL</span><span style={{ textAlign: "right" }}>ROWS</span><span style={{ textAlign: "right" }}>LAST</span></div>
                <div className="market-watchlist-list">
                  {filteredSources.length === 0 ?                   <div className="market-watchlist-empty">No local instruments match this filter.</div> : filteredSources.map((source) => (
                    <button key={source.id} type="button" className={`market-watchlist-row ${activeSource?.id === source.id ? "is-selected" : ""}`} onClick={() => chooseSource(source)} aria-pressed={activeSource?.id === source.id}>
                      <span className="market-watchlist-row__symbol">{source.symbol}<span className="market-watchlist-row__kind">{source.kind}</span></span>
                      <span className="market-watchlist-row__value">{source.row_count.toLocaleString("en-US")}</span>
                      <span className="market-watchlist-row__value">{source.symbol === symbol ? selectedSourcePrice : "—"}</span>
                    </button>
                  ))}
                </div>
              </>
            ) : (
              <div className="market-watchlist-details-view">
                <h2>Instrument</h2>
                {activeSource ? (
                  <>
                    <div className="market-detail-row"><span>Symbol</span><span>{activeSource.symbol}</span></div>
                    <div className="market-detail-row"><span>Source type</span><span>{activeSource.kind}</span></div>
                    <div className="market-detail-row"><span>Records</span><span>{activeSource.row_count.toLocaleString("en-US")}</span></div>
                    <div className="market-detail-row"><span>Provider</span><span>{activeSource.kind}</span></div>
                    <div className="market-detail-row"><span>Loaded candles</span><span>{activeSource.row_count.toLocaleString("en-US")}</span></div>
                  </>
                ) : <div className="market-watchlist-empty">No instrument metadata is available.</div>}
              </div>
            )}
            <div className="market-watchlist-footer"><span>{sources.length} local instruments</span><span>PARQUET</span></div>
          </aside>
        )}

        <aside className="market-utility-rail" aria-label="Workspace panels">
          <button type="button" className={`market-tool-button ${watchlistOpen ? "is-active" : ""}`} onClick={() => setWatchlistOpen((open) => !open)} title={watchlistOpen ? "Hide watchlist" : "Show watchlist"} aria-label={watchlistOpen ? "Hide watchlist" : "Show watchlist"}><PanelRight size={17} /></button>
          <button type="button" className="market-tool-button" onClick={() => { setWatchlistOpen(true); setRightPanel("details"); }} title="Instrument details" aria-label="Instrument details"><Database size={16} /></button>
          <span className="market-toolrail__spacer" />
          <button type="button" className="market-tool-button" disabled title="Objects panel is not included in Phase 1" aria-label="Objects panel"><Layers size={16} /></button>
        </aside>
      </div>

      <footer className="market-statusbar">
        <div className="market-status-group">
          <span className="market-status-item market-status-accent"><i className={`market-status-dot ${error ? "is-error" : ""}`} />{error ? "LOCAL DATA ISSUE" : "LOCAL PARQUET"}</span>
          <span className="market-status-item">{symbol || "—"}</span>
          <span className="market-status-item">{timeframe}</span>
          <span className="market-status-item">{candles.length.toLocaleString("en-US")} candles</span>
        </div>
        <div className="market-status-group"><span className="market-status-item">{lastCandle ? `UTC ${formatUtc(lastCandle.ts)}` : "UTC —"}</span><span className="market-status-item">VORA · PHASE 1 UI</span></div>
      </footer>

      {notice && (
        <div className="market-info-dialog" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setNotice(null); }}>
          <section className="market-info-dialog__panel" role="dialog" aria-modal="true" aria-labelledby="market-notice-title">
            <div className="market-info-dialog__heading"><strong id="market-notice-title">Phase 1 workspace</strong><button type="button" className="market-icon-button" onClick={() => setNotice(null)} aria-label="Close"><X size={15} /></button></div>
            <p>{notice}</p>
          </section>
        </div>
      )}
    </div>
  );
}
