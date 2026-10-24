import React, { useEffect, useRef, useState } from "react";
import { BrushToolIcon } from "@/components/market/BrushToolIcon";
import { PathToolIcon } from "@/components/market/PathToolIcon";
import { CircleDot, Eraser, MousePointer2, MousePointerClick, WandSparkles } from "lucide-react";
import { TOOL_GROUPS } from "@/components/market/drawingToolGroups";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

interface Props {
  visible: boolean;
  onToggle: () => void;
  activeTool: string;
  onSelectTool: (id: string) => void;
  cursorFavorites: string[];
  drawingToolFavorites: string[];
}

const TOOLS_A: Array<{ id: string; label: string; icon: React.ReactNode }> = [
  {
    id: "path",
    label: "Path",
    icon: <PathToolIcon />,
  },
  {
    id: "rect",
    label: "Rectangle",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M7.5 6h13v-1h-13z" id="Line" />
          <path d="M7.5 23h13v-1h-13z" />
          <path d="M5 7.5v13h1v-13z" />
          <path d="M22 7.5v13h1v-13z" />
          <path d="M5.5 7c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM22.5 7c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM22.5 24c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM5.5 24c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "trendline",
    label: "Trend Line",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M7.354 21.354l14-14-.707-.707-14 14z" />
          <path d="M22.5 7c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM5.5 24c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "circle",
    label: "Circle",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="none">
        <path stroke="currentColor" d="M16 14a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z" />
        <path fill="currentColor" fillRule="evenodd" d="M4.5 14a9.5 9.5 0 0 1 18.7-2.37 2.5 2.5 0 0 0 0 4.74A9.5 9.5 0 0 1 4.5 14Zm19.7 2.5a10.5 10.5 0 1 1 0-5 2.5 2.5 0 0 1 0 5ZM22.5 14a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Z" />
      </svg>
    ),
  },
  {
    id: "horzray",
    label: "Horizontal Ray",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M8.5 15h16.5v-1h-16.5z" />
          <path d="M6.5 16c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "text",
    label: "Text",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <path fill="currentColor" d="M8 6.5c0-.28.22-.5.5-.5H14v16h-2v1h5v-1h-2V6h5.5c.28 0 .5.22.5.5V9h1V6.5c0-.83-.67-1.5-1.5-1.5h-12C7.67 5 7 5.67 7 6.5V9h1V6.5Z" />
      </svg>
    ),
  },
  {
    id: "bezier",
    label: "Bezier Quadro",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M6.256 20.652c.548-3.024 1.607-5.962 3.329-8.312l-.807-.591c-1.825 2.493-2.933 5.565-3.506 8.725l.984.178z" />
          <path d="M12.243 9.657c2.365-1.764 5.345-2.846 8.416-3.402l-.178-.984c-3.21.581-6.326 1.712-8.836 3.584l.598.802z" />
          <path d="M10.5 12c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM22.5 7c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM5.5 24c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "horzline",
    label: "Horizontal Line",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M4 15h8.5v-1h-8.5zM16.5 15h8.5v-1h-8.5z" />
          <path d="M14.5 16c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "vertline",
    label: "Vertical Line",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M15 12.5v-8.5h-1v8.5zM14 16.5v8.5h1v-8.5z" />
          <path d="M14.5 16c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "crossline",
    label: "Crossline",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor" fillRule="nonzero">
          <path d="M4 15h8.5v-1h-8.5zM16.5 15h8.5v-1h-8.5z" />
          <path d="M15 12v-8.5h-1v8.5zM14 16.5v8.5h1v-8.5z" />
          <path d="M14.5 16c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "brush",
    label: "Brush",
    icon: <BrushToolIcon />,
  },
  {
    id: "highlighter",
    label: "Highlighter",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 23" width="25" height="23">
        <path fill="currentColor" fillRule="evenodd" clipRule="evenodd" d="M13.402 0L6.78144 6.71532C6.67313 6.82518 6.5917 6.95862 6.54354 7.10518L5.13578 11.3889C4.89843 12.1111 5.08375 12.9074 5.61449 13.4458L5.68264 13.5149L0 19.2789L8.12695 22.4056L11.2874 19.1999L11.3556 19.269C11.8863 19.8073 12.6713 19.9953 13.3834 19.7546L17.6013 18.3285C17.7493 18.2784 17.8835 18.1945 17.9931 18.0832L24.6912 11.2892L23.9857 10.5837L17.515 17.147L7.70658 7.19818L14.1076 0.705575L13.402 0ZM6.07573 11.7067L7.24437 8.15061L16.576 17.6158L13.0701 18.8012C12.7141 18.9215 12.3215 18.8275 12.0562 18.5584L6.31509 12.7351C6.04972 12.466 5.95706 12.0678 6.07573 11.7067ZM6.30539 14.3045L10.509 18.5682L7.87935 21.2355L1.78414 18.8904L6.30539 14.3045Z" />
      </svg>
    ),
  },
];

const TOOLS_C: Array<{ id: string; label: string; icon: React.ReactNode }> = [
  {
    id: "rr_long",
    label: "Long Position",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <path fill="currentColor" d="M5.5 20c1.2 0 2.22.86 2.45 2H25v1H7.95a2.5 2.5 0 1 1-2.45-3m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3M25 18H5v-1h20zm-11-4h3v1h-4V9h1zM5.5 4c1.2 0 2.22.86 2.45 2H25v1H7.95A2.5 2.5 0 1 1 5.5 4m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3" />
      </svg>
    ),
  },
  {
    id: "rr_short",
    label: "Short Position",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <path fill="currentColor" d="M5.5 20c1.2 0 2.22.86 2.45 2H25v1H7.95a2.5 2.5 0 1 1-2.45-3m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m9.52-7q.53 0 .93.2l.2.1q.27.17.46.43l.06.1q.2.3.25.73v.02h-.82v-.01a1 1 0 0 0-.36-.55 1.2 1.2 0 0 0-.73-.2q-.22 0-.4.06l-.12.04a1 1 0 0 0-.36.3 1 1 0 0 0-.13.43v.01q0 .2.09.34t.26.25q.19.1.48.2l.76.21q.71.2 1.06.57l.08.1q.27.36.27.9v.02q0 .45-.2.81l-.07.1a2 2 0 0 1-.72.62q-.45.22-1.02.22-.42 0-.77-.1l-.22-.1a2 2 0 0 1-.7-.55 2 2 0 0 1-.3-.84v-.02h.86v.01q.1.36.39.57t.76.22q.34 0 .59-.11a1 1 0 0 0 .4-.31l.06-.1q.09-.17.08-.36a1 1 0 0 0-.1-.38l-.1-.1a1.5 1.5 0 0 0-.65-.34l-.78-.21q-.46-.13-.77-.34-.3-.21-.45-.51-.14-.3-.14-.73 0-.5.24-.88l.06-.09q.24-.32.61-.5.43-.23.96-.23M25 12H5v-1h20zM5.5 4c1.2 0 2.22.86 2.45 2H25v1H7.95A2.5 2.5 0 1 1 5.5 4m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3" />
      </svg>
    ),
  },
  {
    id: "date_range",
    label: "Date and Price Range",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28">
        <g fill="currentColor">
          <path fillRule="nonzero" d="M6.5 23v1h17.5v-17.5h-1v16.5z" />
          <path fillRule="nonzero" d="M21.5 5v-1h-17.5v17.5h1v-16.5z" />
          <path fillRule="nonzero" d="M4.5 25c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5zM23.5 6c.828 0 1.5-.672 1.5-1.5s-.672-1.5-1.5-1.5-1.5.672-1.5 1.5.672 1.5 1.5 1.5zm0 1c-1.381 0-2.5-1.119-2.5-2.5s1.119-2.5 2.5-2.5 2.5 1.119 2.5 2.5-1.119 2.5-2.5 2.5z" />
          <path fillRule="nonzero" d="M13 9v13h1v-13z" id="Line" />
          <path d="M13.5 6l2.5 3h-5z" />
          <path fillRule="nonzero" d="M19 14h-13v1h13z" />
          <path d="M19 17v-5l3 2.5z" />
        </g>
      </svg>
    ),
  },
  {
    id: "arrow_down",
    label: "Arrow Mark Down",
    icon: (
      <svg viewBox="0 0 28 28" width="20" height="20" fill="currentColor" fillRule="nonzero">
        <path d="M17 12v-6h-6v6h-4.865l7.865 9.438 7.865-9.438h-4.865zm-7-7h8v6h6l-10 12-10-12h6v-6z" />
      </svg>
    ),
  },
  {
    id: "arrow_up",
    label: "Arrow Mark Up",
    icon: (
      <svg viewBox="0 0 28 28" width="20" height="20" fill="currentColor" fillRule="nonzero">
        <path d="M11 16v6h6v-6h4.865l-7.865-9.438-7.865 9.438h4.865zm7 7h-8v-6h-6l10-12 10 12h-6v6z" />
      </svg>
    ),
  },
  {
    id: "measure",
    label: "Measure",
    icon: (
      <svg viewBox="0 0 28 28" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M3 20 L25 8" />
        <path d="M3 17 L6 20 L3 23" />
        <path d="M25 5 L22 8 L25 11" />
      </svg>
    ),
  },
];

const IMPLEMENTED_TOOL_IDS = new Set(["rect", "trendline", "horzline", "horzray", "vertline", "measure"]);
const TOOLS: Array<{ id: string; label: string; icon: React.ReactNode }> =
  [...TOOLS_A, ...TOOLS_C].filter((tool) => IMPLEMENTED_TOOL_IDS.has(tool.id));

const CURSOR_FAVORITE_TOOLS: Array<{ id: string; label: string; icon: React.ReactNode; implemented: boolean }> = [
  {
    id: "cursor",
    label: "Cross",
    implemented: true,
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
        <g fill="currentColor">
          <path d="M18 15h8v-1h-8z" />
          <path d="M14 18v8h1v-8zM14 3v8h1V3zM3 15h8v-1H3z" />
        </g>
      </svg>
    ),
  },
  { id: "cursor-dot", label: "Dot", icon: <CircleDot size={22} />, implemented: false },
  { id: "cursor-arrow", label: "Arrow", icon: <MousePointer2 size={22} />, implemented: false },
  { id: "cursor-demo", label: "Demonstration", icon: <MousePointerClick size={22} />, implemented: false },
  { id: "cursor-magic", label: "Magic", icon: <WandSparkles size={22} />, implemented: false },
  { id: "cursor-eraser", label: "Eraser", icon: <Eraser size={22} />, implemented: false },
];

function DragHandle({ onMouseDown }: { onMouseDown: (e: React.MouseEvent) => void }) {
  return (
    <div
      onMouseDown={onMouseDown}
      data-vora-drag-handle
      style={{ width: 12, height: 24, display: "grid", placeItems: "center", cursor: "grab", color: "#7f8997" }}
      title="Drag to move"
    >
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 12" width="8" height="12" fill="currentColor">
        <rect width="2" height="2" rx="1"></rect>
        <rect width="2" height="2" rx="1" y="5"></rect>
        <rect width="2" height="2" rx="1" y="10"></rect>
        <rect width="2" height="2" rx="1" x="6"></rect>
        <rect width="2" height="2" rx="1" x="6" y="5"></rect>
        <rect width="2" height="2" rx="1" x="6" y="10"></rect>
      </svg>
    </div>
  );
}

function ToolButton({ id, icon, label, active, disabled = false, onClick }: { id: string; icon: React.ReactNode; label: string; active: boolean; disabled?: boolean; onClick: () => void }) {
  const [showTip, setShowTip] = useState(false);
  const timer = useRef<number | null>(null);
  useEffect(() => {
    return () => {
      if (timer.current !== null) { window.clearTimeout(timer.current); timer.current = null; }
    };
  }, []);
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      data-name={id}
      title=""
      onClick={onClick}
      onMouseEnter={() => {
        if (timer.current !== null) window.clearTimeout(timer.current);
        timer.current = window.setTimeout(() => setShowTip(true), 2000);
      }}
      onMouseLeave={() => {
        if (timer.current !== null) { window.clearTimeout(timer.current); timer.current = null; }
        setShowTip(false);
      }}
      style={{
        position: "relative", width: 32, height: 32, display: "grid", placeItems: "center",
        background: active ? "rgba(255,183,77,0.2)" : "transparent",
        color: active ? "#ffb74d" : "#bdbdbd",
        border: "none", borderRadius: 4, cursor: disabled ? "not-allowed" : "pointer", opacity: disabled ? 0.4 : 1,
      }}
    >
      {icon}
      {showTip && (
        <span style={{
          position: "absolute", top: "calc(100% + 6px)", left: "50%", transform: "translateX(-50%)",
          background: "#181818", color: "#d6dbe2", padding: "2px 6px", border: "1px solid #2a2a2a", borderRadius: 3,
          fontSize: 11, whiteSpace: "nowrap", pointerEvents: "none", zIndex: 30,
        }}>{label}</span>
      )}
    </button>
  );
}

export function FloatingToolbar({ visible, onToggle, activeTool, onSelectTool, cursorFavorites, drawingToolFavorites }: Props) {
  void onToggle;
  const [pos, setPos] = useState<{ x: number; y: number } | null>(() => {
    try {
      const raw = getSetting("vora.floating_toolbar.pos.v2");
      if (!raw) return null;
      const stored: unknown = JSON.parse(raw);
      if (
        typeof stored === "object" && stored !== null &&
        "x" in stored && "y" in stored &&
        typeof stored.x === "number" && Number.isFinite(stored.x) &&
        typeof stored.y === "number" && Number.isFinite(stored.y)
      ) {
        return { x: stored.x, y: stored.y };
      }
    } catch { /* ignore */ }
    return null;
  });

  useEffect(() => {
    if (!pos) return;
    setSetting("vora.floating_toolbar.pos.v2", JSON.stringify(pos));
  }, [pos]);

  function startDrag(e: React.MouseEvent) {
    e.preventDefault();
    const toolbar = e.currentTarget.parentElement;
    const container = toolbar?.parentElement;
    if (!toolbar || !container) return;
    const toolbarRect = toolbar.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const originX = pos?.x ?? toolbarRect.left - containerRect.left;
    const originY = pos?.y ?? toolbarRect.top - containerRect.top;
    const pointerX = e.clientX;
    const pointerY = e.clientY;
    const onMove = (ev: MouseEvent) => {
      setPos({
        x: Math.max(0, Math.min(container.clientWidth - toolbar.offsetWidth, originX + ev.clientX - pointerX)),
        y: Math.max(0, Math.min(container.clientHeight - toolbar.offsetHeight, originY + ev.clientY - pointerY)),
      });
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }

  if (!visible) return null;
  return (
    <div
      className="vora-floating-toolbar vora-floating-ui"
      style={{
        position: "absolute",
        ...(pos ? { left: pos.x, top: pos.y } : { left: "50%", bottom: 8, transform: "translateX(-50%)" }),
        zIndex: 20,
        background: "#181818", border: "1px solid #2a2a2a", borderRadius: 4,
        padding: "4px 6px", display: "flex", alignItems: "center", gap: 2,
        boxShadow: "0 8px 24px rgba(0,0,0,0.45)", userSelect: "none",
        maxWidth: "calc(100% - 16px)", overflowX: "auto",
      }}
    >
      <DragHandle onMouseDown={startDrag} />
      {TOOLS.map((t) => (
        <ToolButton
          key={t.id}
          id={t.id}
          icon={t.icon}
          label={t.label}
          active={activeTool === t.id}
          onClick={() => onSelectTool(t.id)}
        />
      ))}
      {CURSOR_FAVORITE_TOOLS.filter((tool) => cursorFavorites.includes(tool.id)).map((tool) => (
        <ToolButton
          key={tool.id}
          id={tool.id}
          icon={tool.icon}
          label={tool.label}
          active={activeTool === tool.id}
          disabled={!tool.implemented}
          onClick={() => onSelectTool(tool.id)}
        />
      ))}
      {TOOL_GROUPS.flatMap((group) => group.tools)
        .filter((tool) => tool.implemented === true && drawingToolFavorites.includes(tool.id) && !TOOLS.some((baseTool) => baseTool.id === tool.id))
        .map((tool) => {
          const Icon = tool.icon;
          const icon = tool.id === "brush"
            ? <BrushToolIcon size={22} />
            : tool.id === "path"
              ? <PathToolIcon size={22} />
            : <Icon size={22} strokeWidth={1.6} />;
          return (
            <ToolButton
              key={tool.id}
              id={tool.id}
              icon={icon}
              label={tool.label}
              active={activeTool === tool.id}
              onClick={() => onSelectTool(tool.id)}
            />
          );
        })}
    </div>
  );
}
