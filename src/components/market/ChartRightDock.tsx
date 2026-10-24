import { Link } from "react-router-dom";
import {
  BarChart3,
  Bell,
  Bookmark,
  ChartNoAxesColumnIncreasing,
  Eye,
  EyeOff,
  GitBranch,
  House,
  Layers,
  LayoutGrid,
  MoveHorizontal,
  MoveVertical,
  Ruler,
  Smile,
  TrendingUp,
  X,
  type LucideIcon,
} from "lucide-react";
import type { DrawingAny } from "@/lib/market/lineDrawings";
import { BrushToolIcon } from "@/components/market/BrushToolIcon";
import { PathToolIcon } from "@/components/market/PathToolIcon";
import { RectangleToolIcon } from "@/components/market/RectangleToolIcon";
import { TOOL_GROUPS } from "@/components/market/drawingToolGroups";
import { FIB_GANN_TOOL_LABELS, GEOMETRY_TOOL_LABELS, type ChartDrawingAlert } from "@/lib/market/drawingTypes";
import { WatchlistPanel } from "@/components/market/WatchlistPanel";

export type ChartRightPanelId = "watchlist" | "alerts" | "objects" | "products";

interface ChartRightDockProps {
  activePanel: ChartRightPanelId | null;
  onPanelChange: (panel: ChartRightPanelId | null) => void;
  drawings: DrawingAny[];
  alerts: ChartDrawingAlert[];
  onSelectDrawing: (id: string) => void;
  onToggleDrawingHidden: (id: string, hidden: boolean) => void;
  onToggleAlert: (id: string, enabled: boolean) => void;
  onDeleteAlert: (id: string) => void;
}

interface ToolbarItem {
  id: ChartRightPanelId;
  label: string;
  icon: LucideIcon;
}

const PRIMARY_ITEMS: ToolbarItem[] = [
  { id: "watchlist", label: "Watchlist, details, and news", icon: Bookmark },
  { id: "alerts", label: "Alerts", icon: Bell },
  { id: "objects", label: "Object tree and data window", icon: Layers },
];

const FOOTER_ITEMS: ToolbarItem[] = [
  { id: "products", label: "Products", icon: LayoutGrid },
];

const PANEL_TITLES: Record<ChartRightPanelId, string> = {
  watchlist: "Watchlist",
  alerts: "Alerts",
  objects: "Object tree",
  products: "Products",
};

function drawingName(drawing: DrawingAny): string {
  switch (drawing.type) {
    case "rectangle": return "Rectangle";
    case "vline": return "Vertical line";
    case "hline": return "Horizontal line";
    case "hray": return "Horizontal ray";
    case "trendline": return "Trend line";
    case "ray": return "Ray";
    case "info-line": return "Info line";
    case "extended-line": return "Extended line";
    case "trend-angle": return "Trend angle";
    case "crossline": return "Crossline";
    case "parallel-channel": return "Parallel channel";
    case "pitchfork": return "Pitchfork";
    case "regression-trend": return "Regression trend";
    case "fib-gann": return FIB_GANN_TOOL_LABELS[drawing.tool];
    case "icon": return drawing.name ?? drawing.symbol;
    case "geometry": return GEOMETRY_TOOL_LABELS[drawing.tool];
  }
}

function drawingIcon(drawing: DrawingAny): LucideIcon | null {
  switch (drawing.type) {
    case "rectangle": return null;
    case "vline": return MoveVertical;
    case "hline":
    case "hray": return MoveHorizontal;
    case "trendline": return TrendingUp;
    case "ray": return TrendingUp;
    case "info-line": return Ruler;
    case "extended-line": return TrendingUp;
    case "trend-angle": return TrendingUp;
    case "crossline": return MoveHorizontal;
    case "parallel-channel": return TrendingUp;
    case "pitchfork": return GitBranch;
    case "regression-trend": return TrendingUp;
    case "fib-gann": return drawing.tool.startsWith("gann") ? GitBranch : ChartNoAxesColumnIncreasing;
    case "icon": return Smile;
    case "geometry": return drawing.tool === "brush" || drawing.tool === "path"
      ? null
      : TOOL_GROUPS.find((group) => group.id === "geometric")?.tools.find((tool) => tool.id === drawing.tool)?.icon ?? GitBranch;
  }
}

function ObjectTree({
  drawings,
  onSelectDrawing,
  onToggleDrawingHidden,
}: Pick<ChartRightDockProps, "drawings" | "onSelectDrawing" | "onToggleDrawingHidden">): React.ReactElement {
  const orderedDrawings = [...drawings].reverse();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-between border-b border-[#383838] px-3 py-2 text-[10px] font-semibold uppercase text-[#787b86]">
        <span>Drawings</span>
        <span>{drawings.length}</span>
      </div>
      {orderedDrawings.length === 0 ? (
        <p className="px-3 py-4 text-xs text-[#787b86]">No chart objects</p>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto py-1">
          {orderedDrawings.map((drawing) => {
            const Icon = drawingIcon(drawing);
            return (
              <div key={drawing.id} className={`flex items-center hover:bg-[#2a2a2a] ${drawing.hidden ? "opacity-55" : ""}`}>
                <button
                  type="button"
                  aria-pressed={drawing.selected}
                  onClick={() => onSelectDrawing(drawing.id)}
                  className={`flex min-w-0 flex-1 items-center gap-2 px-3 py-2 text-left text-xs ${drawing.selected ? "bg-[#2a2a2a] text-[#d6dbe2]" : "text-[#aeb4c0]"}`}
                >
                  {drawing.type === "rectangle" ? (
                    <RectangleToolIcon size={14} className="shrink-0" />
                  ) : drawing.type === "geometry" && drawing.tool === "brush" ? (
                    <BrushToolIcon size={14} className="shrink-0" />
                  ) : drawing.type === "geometry" && drawing.tool === "path" ? (
                    <PathToolIcon size={14} className="shrink-0" />
                  ) : Icon ? (
                    <Icon size={14} className="shrink-0" />
                  ) : null}
                  <span className="min-w-0 flex-1 truncate">{drawing.name ?? drawingName(drawing)}</span>
                  <span className="h-2.5 w-2.5 shrink-0 rounded-full border border-white/20" style={{ backgroundColor: drawing.type === "rectangle" ? drawing.borderColor : drawing.color, opacity: drawing.colorOpacity ?? 1 }} />
                </button>
                <button type="button" aria-label={drawing.hidden ? `Show ${drawingName(drawing)}` : `Hide ${drawingName(drawing)}`} title={drawing.hidden ? "Show drawing" : "Hide drawing"} onClick={() => onToggleDrawingHidden(drawing.id, !drawing.hidden)} className="flex h-8 w-8 shrink-0 items-center justify-center text-[#858a93] hover:text-[#d6dbe2]">
                  {drawing.hidden ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ProductsPanel(): React.ReactElement {
  const links = [
    { to: "/", label: "Canvas", icon: LayoutGrid },
    { to: "/chart", label: "Chart", icon: BarChart3 },
    { to: "/sources", label: "Sources", icon: Bookmark },
    { to: "/snapshots", label: "Snapshots", icon: Layers },
  ];

  return (
    <nav aria-label="VORA workspaces" className="py-1">
      {links.map(({ to, label, icon: Icon }) => (
        <Link
          key={to}
          to={to}
          className="flex items-center gap-2.5 px-3 py-2 text-xs text-[#aeb4c0] hover:bg-[#2a2a2a] hover:text-[#d6dbe2]"
        >
          <Icon size={15} />
          {label}
        </Link>
      ))}
    </nav>
  );
}

function PanelContent({
  panel,
  drawings,
  onSelectDrawing,
  onToggleDrawingHidden,
  alerts,
  onToggleAlert,
  onDeleteAlert,
}: Pick<ChartRightDockProps, "drawings" | "onSelectDrawing" | "onToggleDrawingHidden" | "alerts" | "onToggleAlert" | "onDeleteAlert"> & { panel: ChartRightPanelId }): React.ReactElement {
  switch (panel) {
    case "watchlist":
      return <WatchlistPanel />;
    case "alerts":
      return (
        <div className="flex min-h-0 flex-1 flex-col">
          {alerts.length === 0 ? <div className="flex flex-1 flex-col items-center justify-center gap-2 px-5 text-center"><Bell size={20} className="text-[#596371]" /><p className="text-xs font-medium text-[#d6dbe2]">No alerts</p></div> :
            <div className="min-h-0 flex-1 overflow-y-auto">{alerts.map((alert) => <div key={alert.id} className="flex items-start gap-2 border-b border-[#242424] px-3 py-3"><button type="button" role="switch" aria-checked={alert.enabled} aria-label={`${alert.enabled ? "Disable" : "Enable"} ${alert.message}`} onClick={() => onToggleAlert(alert.id, !alert.enabled)} className={`mt-0.5 h-4 w-7 shrink-0 rounded-full p-0.5 ${alert.enabled ? "bg-[#9c27b0]" : "bg-[#555]"}`}><span className={`block h-3 w-3 rounded-full bg-white transition-transform ${alert.enabled ? "translate-x-3" : ""}`} /></button><div className="min-w-0 flex-1"><div className="truncate text-xs text-[#d6dbe2]">{alert.message}</div><div className="mt-1 text-[10px] text-[#858a93]">M{alert.timeframe} · {alert.condition === "crossing" ? "Crossing" : "Entering drawing"} · {alert.triggerOnce ? "Once" : "Every time"}</div></div><button type="button" aria-label={`Delete alert ${alert.message}`} onClick={() => onDeleteAlert(alert.id)} className="shrink-0 p-1 text-[#858a93] hover:text-[#f87171]"><X size={14} /></button></div>)}</div>}
        </div>
      );
    case "objects":
      return <ObjectTree drawings={drawings} onSelectDrawing={onSelectDrawing} onToggleDrawingHidden={onToggleDrawingHidden} />;
    case "products":
      return <ProductsPanel />;
  }
}

export function ChartRightDock({
  activePanel,
  onPanelChange,
  drawings,
  onSelectDrawing,
  onToggleDrawingHidden,
  alerts,
  onToggleAlert,
  onDeleteAlert,
}: ChartRightDockProps): React.ReactElement {
  const renderButton = ({ id, label, icon: Icon }: ToolbarItem): React.ReactElement => (
    <button
      key={id}
      type="button"
      aria-label={label}
      aria-pressed={activePanel === id}
      title={label}
      onClick={() => onPanelChange(activePanel === id ? null : id)}
      className={`flex h-[44px] w-[44px] shrink-0 items-center justify-center border-l-2 text-[#d1d4dc] transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-[#ffb74d] ${
        activePanel === id
          ? "border-[#ff9800] bg-[#2a2a2a]"
          : "border-transparent hover:bg-[#2a2a2a]"
      }`}
    >
      <Icon size={20} strokeWidth={1.7} />
    </button>
  );

  return (
    <>
      {activePanel && (
        <aside
          aria-label={PANEL_TITLES[activePanel]}
          className="flex min-h-0 min-w-0 flex-col border-l border-[#2a2a2a] bg-[#0f0f0f]"
        >
          <div className="flex h-[44px] shrink-0 items-center justify-between border-b border-[#383838] px-3">
            <h2 className="truncate text-xs font-semibold text-[#d6dbe2]">{PANEL_TITLES[activePanel]}</h2>
            <button
              type="button"
              aria-label="Close side panel"
              title="Close"
              onClick={() => onPanelChange(null)}
              className="flex h-7 w-7 items-center justify-center rounded text-[#787b86] hover:bg-[#2a2a2a] hover:text-[#d6dbe2] focus-visible:outline focus-visible:outline-1 focus-visible:outline-[#ff9800]"
            >
              <X size={15} />
            </button>
          </div>
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
            <PanelContent panel={activePanel} drawings={drawings} onSelectDrawing={onSelectDrawing} onToggleDrawingHidden={onToggleDrawingHidden} alerts={alerts} onToggleAlert={onToggleAlert} onDeleteAlert={onDeleteAlert} />
          </div>
        </aside>
      )}
      <aside
        aria-label="Chart side toolbar"
        className="flex min-h-0 w-[45px] flex-col items-center border-l border-[#2a2a2a] bg-[#0f0f0f] py-1"
      >
        <div className="pt-1">
          {PRIMARY_ITEMS.slice(0, 1).map(renderButton)}
          <Link
            to="/home"
            aria-label="Go to VeraMind home"
            title="Go to VeraMind home"
            className="flex h-[44px] w-[44px] shrink-0 items-center justify-center border-l-2 border-transparent text-[#d1d4dc] transition-colors hover:bg-[#2a2a2a] focus-visible:outline focus-visible:outline-1 focus-visible:outline-[#ffb74d]"
          >
            <House size={20} strokeWidth={1.7} />
          </Link>
          {PRIMARY_ITEMS.slice(1).map(renderButton)}
        </div>
        <div className="flex-1" />
        <div className="mb-1 h-px w-7 bg-[#2a2a2a]" />
        {FOOTER_ITEMS.map(renderButton)}
      </aside>
    </>
  );
}